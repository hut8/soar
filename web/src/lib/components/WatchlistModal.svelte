<script lang="ts">
	import { Switch, SegmentedControl } from '@skeletonlabs/skeleton-svelte';
	import { Plus, X, Plane, Radio, Eye, Building2 } from '@lucide/svelte';
	import { browser } from '$app/environment';
	import { serverCall } from '$lib/api/server';
	import { watchlist } from '$lib/stores/watchlist';
	import { DeviceRegistry } from '$lib/services/DeviceRegistry';
	import ClubSelector from '$lib/components/ClubSelector.svelte';
	import { getAddressTypeLabel } from '$lib/formatters';
	import type { Device } from '$lib/types';
	let { showModal = $bindable() } = $props();

	// State
	let newWatchlistEntry = $state({
		type: 'registration',
		registration: '',
		deviceAddressType: 'I',
		deviceAddress: ''
	});
	let searchInProgress = $state(false);
	let errorMessage = $state('');

	// Club tab state
	let selectedClub = $state<string[]>([]);
	let clubDevices = $state<Device[]>([]);
	let clubSearchInProgress = $state(false);
	let clubErrorMessage = $state('');

	// Club names cache for watchlist entries
	let clubNames = $state(new Map<string, string>());

	// Clear error message when user interacts with form
	function clearError() {
		errorMessage = '';
	}

	// Clear club error message
	function clearClubError() {
		clubErrorMessage = '';
	}

	// Add entry to watchlist
	async function addWatchlistEntry() {
		if (searchInProgress) return;

		// Clear any previous error messages
		clearError();

		let device: Device | null = null;

		if (newWatchlistEntry.type === 'registration') {
			const registration = newWatchlistEntry.registration.trim().toUpperCase();
			if (!registration) return;

			// Search for device by registration
			searchInProgress = true;
			try {
				const response = await serverCall<{ devices: Device[] }>(
					`/devices?registration=${encodeURIComponent(registration)}`
				);
				if (response.devices && response.devices.length > 0) {
					device = response.devices[0];
				} else {
					errorMessage = `Aircraft with registration "${registration}" not found`;
				}
			} catch (error) {
				console.warn(`Failed to fetch device for registration ${registration}:`, error);
				errorMessage = 'Failed to search for device. Please try again.';
			} finally {
				searchInProgress = false;
			}
		} else {
			const addressType = newWatchlistEntry.deviceAddressType.trim();
			const address = newWatchlistEntry.deviceAddress.trim().toUpperCase();
			if (!addressType || !address) return;

			// Search for device by address and type
			searchInProgress = true;
			try {
				const response = await serverCall<{ devices: Device[] }>(
					`/devices?address=${encodeURIComponent(address)}&address-type=${encodeURIComponent(addressType)}`
				);
				if (response.devices && response.devices.length > 0) {
					device = response.devices[0];
				} else {
					errorMessage = `Device with address "${address}" (${addressType}) not found`;
				}
			} catch (error) {
				console.warn(`Failed to fetch device for address ${address} (${addressType}):`, error);
				errorMessage = 'Failed to search for device. Please try again.';
			} finally {
				searchInProgress = false;
			}
		}

		// Only add to watchlist if device was found
		if (device && device.id) {
			// Check for duplicates
			const existingEntry = $watchlist.entries.find((entry) => entry.deviceId === device.id);
			if (existingEntry) {
				errorMessage = 'This aircraft is already in your watchlist';
				return;
			}

			// Add the device to the registry and watchlist
			DeviceRegistry.getInstance().setDevice(device);
			watchlist.add(device.id);
			// Clear the search inputs on success
			newWatchlistEntry = {
				type: 'registration',
				registration: '',
				deviceAddressType: 'I',
				deviceAddress: ''
			};
		}
	}

	// Remove entry from watchlist
	function removeWatchlistEntry(id: string) {
		watchlist.remove(id);
	}

	// Toggle entry active state
	function toggleWatchlistEntry(id: string) {
		watchlist.toggleActive(id);
	}

	// Load devices for selected club
	async function loadClubDevices() {
		if (!selectedClub.length || clubSearchInProgress) return;

		const clubId = selectedClub[0];
		if (!clubId) return;

		clubSearchInProgress = true;
		clubErrorMessage = '';

		try {
			const response = await serverCall<{ devices: Device[] }>(`/clubs/${clubId}/devices`);
			// Only update if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubDevices = response.devices || [];
			}
		} catch (error) {
			console.warn(`Failed to fetch devices for club:`, error);
			// Only show error if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubErrorMessage = 'Failed to load club devices. Please try again.';
				clubDevices = [];
			}
		} finally {
			clubSearchInProgress = false;
		}
	}

	// Add individual device to watchlist
	function addDeviceToWatchlist(device: Device) {
		// Check for duplicates
		const existingEntry = $watchlist.entries.find((entry) => entry.deviceId === device.id);
		if (existingEntry) {
			return; // Already in watchlist
		}

		DeviceRegistry.getInstance().setDevice(device);
		watchlist.add(device.id);
	}

	// Add all club devices to watchlist
	function addAllClubDevices() {
		if (!clubDevices.length) return;

		clubDevices.forEach((device) => {
			addDeviceToWatchlist(device);
		});
	}

	// Check if device is already in watchlist
	function isDeviceInWatchlist(deviceId: string): boolean {
		return $watchlist.entries.some((entry) => entry.deviceId === deviceId);
	}

	// Handle club selection change
	function handleClubChange(details: { value: string[] }) {
		selectedClub = details.value;
		clearClubError();

		if (selectedClub.length > 0) {
			loadClubDevices();
		} else {
			clubDevices = [];
		}
	}

	// Load watchlist on mount
	$effect(() => {
		if (browser) {
			watchlist.loadFromStorage();
		}
	});

	// Get devices from registry for watchlist entries
	const deviceRegistry = $derived(DeviceRegistry.getInstance());
	const entriesWithDevices = $derived(
		$watchlist.entries.map((entry) => {
			const device = deviceRegistry.getDevice(entry.deviceId);
			return {
				...entry,
				device: device || {
					id: entry.deviceId,
					registration: 'Unknown',
					aircraft_model: 'Unknown',
					address_type: 'Unknown',
					address: 'Unknown',
					competition_number: '',
					tracked: false,
					identified: false,
					device_address: 'Unknown',
					created_at: '',
					updated_at: '',
					from_ddb: false
				}
			};
		})
	);

	// Fetch club name for a device
	async function fetchClubName(clubId: string): Promise<void> {
		if (clubNames.has(clubId)) {
			return; // Already fetched
		}

		try {
			const club = await serverCall<{ name: string }>(`/clubs/${clubId}`);
			clubNames.set(clubId, club.name);
		} catch (error) {
			console.warn(`Failed to fetch club name for ${clubId}:`, error);
			clubNames.set(clubId, 'Unknown Club');
		}
	}

	// Effect to fetch club names when entries change
	$effect(() => {
		// Get unique club IDs from all devices
		const clubIds: string[] = [];
		for (const entry of entriesWithDevices) {
			if (entry.device.club_id && !clubIds.includes(entry.device.club_id)) {
				clubIds.push(entry.device.club_id);
			}
		}

		// Fetch club names for each unique club ID
		for (const clubId of clubIds) {
			void fetchClubName(clubId);
		}
	});
</script>

<!-- Watchlist Modal -->
{#if showModal}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-20 dark:bg-black/70"
		role="presentation"
		onclick={() => (showModal = false)}
		onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
	>
		<div
			class="flex h-full max-h-[calc(90vh-5rem)] w-full max-w-9/10 flex-col card bg-surface-50 text-surface-900 shadow-xl dark:bg-surface-900 dark:text-surface-50"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
			role="dialog"
			aria-modal="true"
			aria-labelledby="watchlist-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div class="flex-shrink-0 p-4 pb-0">
				<div class="mb-4 flex items-center justify-between">
					<h2 id="watchlist-title" class="text-xl font-bold">Watchlist</h2>
					<button class="preset-tonal-surface-500 btn btn-sm" onclick={() => (showModal = false)}>
						<X size={20} />
					</button>
				</div>
			</div>

			<!-- Content area with flex layout -->
			<div class="flex min-h-0 flex-1 flex-col space-y-6 p-4 pt-0">
				<!-- Add new entry -->
				<section class="flex-shrink-0">
					<h3 class="mb-3 text-lg font-semibold">Add Aircraft</h3>
					<div
						class="mb-3 space-y-3 rounded-lg border border-surface-300 p-3 dark:border-surface-600"
					>
						<!-- Mobile: Vertical layout (segment above inputs) -->
						<div class="space-y-3 md:hidden">
							<!-- Search type selector -->
							<SegmentedControl
								name="watchlist-type-mobile"
								value={newWatchlistEntry.type}
								orientation="vertical"
								onValueChange={(details) => {
									if (details.value) {
										newWatchlistEntry.type = details.value;
										clearError();
										clearClubError();
									}
								}}
							>
								<SegmentedControl.Item value="registration">
									<div class="flex flex-row items-center">
										<Plane size={16} />
										<span class="ml-1">Registration</span>
									</div>
								</SegmentedControl.Item>
								<SegmentedControl.Item value="device">
									<div class="flex flex-row items-center">
										<Radio size={16} />
										<span class="ml-1">Device</span>
									</div>
								</SegmentedControl.Item>
								<SegmentedControl.Item value="club">
									<div class="flex flex-row items-center">
										<Building2 size={16} />
										<span class="ml-1">Club</span>
									</div>
								</SegmentedControl.Item>
							</SegmentedControl>

							{#if newWatchlistEntry.type === 'registration'}
								<input
									class="input"
									placeholder="Aircraft registration (e.g., N12345)"
									bind:value={newWatchlistEntry.registration}
									onkeydown={(e) => e.key === 'Enter' && addWatchlistEntry()}
									oninput={() => clearError()}
									disabled={searchInProgress}
								/>
							{:else if newWatchlistEntry.type === 'device'}
								<div class="space-y-3">
									<SegmentedControl
										name="address-type-mobile"
										value={newWatchlistEntry.deviceAddressType}
										orientation="vertical"
										onValueChange={(details) => {
											if (details.value) {
												newWatchlistEntry.deviceAddressType = details.value;
												clearError();
											}
										}}
									>
										<SegmentedControl.Item value="I">ICAO</SegmentedControl.Item>
										<SegmentedControl.Item value="O">OGN</SegmentedControl.Item>
										<SegmentedControl.Item value="F">FLARM</SegmentedControl.Item>
									</SegmentedControl>
									<input
										class="input"
										placeholder="Device address"
										bind:value={newWatchlistEntry.deviceAddress}
										onkeydown={(e) => e.key === 'Enter' && addWatchlistEntry()}
										oninput={() => clearError()}
										disabled={searchInProgress}
									/>
								</div>
							{:else if newWatchlistEntry.type === 'club'}
								<div class="space-y-3">
									<ClubSelector
										bind:value={selectedClub}
										placeholder="Select a club..."
										onValueChange={handleClubChange}
									/>

									{#if clubDevices.length > 0}
										<div class="space-y-2">
											<div class="flex items-center justify-between">
												<span class="text-sm font-medium text-surface-700 dark:text-surface-200">
													Club Aircraft ({clubDevices.length})
												</span>
												<button
													class="btn preset-filled-primary-500 btn-sm"
													onclick={addAllClubDevices}
													disabled={clubSearchInProgress}
												>
													<Plus size={16} />
													Add All
												</button>
											</div>

											<div
												class="max-h-48 overflow-y-auto rounded border border-surface-300 bg-surface-100 p-2 dark:border-surface-600 dark:bg-surface-800"
											>
												<div class="grid gap-2">
													{#each clubDevices as device (device.id)}
														<div
															class="flex items-center justify-between rounded bg-surface-50 p-2 shadow-sm dark:bg-surface-700"
														>
															<div class="min-w-0 flex-1">
																<div class="truncate text-sm font-medium">
																	{device.registration || 'Unknown Registration'}
																</div>
																<div
																	class="truncate text-xs text-surface-600 dark:text-surface-400"
																>
																	{device.aircraft_model || 'Unknown Model'}
																</div>
															</div>
															{#if isDeviceInWatchlist(device.id)}
																<span
																	class="rounded bg-green-100 px-2 py-1 text-xs font-medium text-green-800 dark:bg-green-900 dark:text-green-100"
																>
																	Watched
																</span>
															{:else}
																<button
																	class="btn preset-filled-primary-500 btn-sm"
																	onclick={() => addDeviceToWatchlist(device)}
																>
																	<Plus size={14} />
																	Watch
																</button>
															{/if}
														</div>
													{/each}
												</div>
											</div>
										</div>
									{:else if clubSearchInProgress}
										<div class="py-4 text-center text-sm text-surface-600 dark:text-surface-400">
											<div
												class="mx-auto mb-2 h-4 w-4 animate-spin rounded-full border-2 border-surface-300 border-t-primary-500 dark:border-surface-600"
											></div>
											Loading club aircraft...
										</div>
									{:else if selectedClub.length > 0}
										<div class="py-4 text-center text-sm text-surface-600 dark:text-surface-400">
											No aircraft found for this club.
										</div>
									{/if}

									<!-- Club error message display -->
									{#if clubErrorMessage}
										<div
											class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600 dark:border-red-800 dark:bg-red-950 dark:text-red-400"
										>
											{clubErrorMessage}
										</div>
									{/if}
								</div>
							{/if}
						</div>

						<!-- Desktop: Horizontal layout (segment to the left of inputs) -->
						<div class="hidden md:block">
							<div class="grid grid-cols-[200px_1fr] items-start gap-4">
								<!-- Search type selector -->
								<SegmentedControl
									name="watchlist-type-desktop"
									value={newWatchlistEntry.type}
									orientation="vertical"
									onValueChange={(details) => {
										if (details.value) {
											newWatchlistEntry.type = details.value;
											clearError();
											clearClubError();
										}
									}}
								>
									<SegmentedControl.Item value="registration">
										<div class="flex flex-row items-center">
											<Plane size={16} />
											<span class="ml-1">Registration</span>
										</div>
									</SegmentedControl.Item>
									<SegmentedControl.Item value="device">
										<div class="flex flex-row items-center">
											<Radio size={16} />
											<span class="ml-1">Device</span>
										</div>
									</SegmentedControl.Item>
									<SegmentedControl.Item value="club">
										<div class="flex flex-row items-center">
											<Building2 size={16} />
											<span class="ml-1">Club</span>
										</div>
									</SegmentedControl.Item>
								</SegmentedControl>

								<!-- Input area -->
								<div>
									{#if newWatchlistEntry.type === 'registration'}
										<input
											class="input"
											placeholder="Aircraft registration (e.g., N12345)"
											bind:value={newWatchlistEntry.registration}
											onkeydown={(e) => e.key === 'Enter' && addWatchlistEntry()}
											oninput={() => clearError()}
											disabled={searchInProgress}
										/>
									{:else if newWatchlistEntry.type === 'device'}
										<div class="space-y-3">
											<SegmentedControl
												name="address-type-desktop"
												value={newWatchlistEntry.deviceAddressType}
												orientation="vertical"
												onValueChange={(details) => {
													if (details.value) {
														newWatchlistEntry.deviceAddressType = details.value;
														clearError();
													}
												}}
											>
												<SegmentedControl.Item value="I">ICAO</SegmentedControl.Item>
												<SegmentedControl.Item value="O">OGN</SegmentedControl.Item>
												<SegmentedControl.Item value="F">FLARM</SegmentedControl.Item>
											</SegmentedControl>
											<input
												class="input"
												placeholder="Device address"
												bind:value={newWatchlistEntry.deviceAddress}
												onkeydown={(e) => e.key === 'Enter' && addWatchlistEntry()}
												oninput={() => clearError()}
												disabled={searchInProgress}
											/>
										</div>
									{:else if newWatchlistEntry.type === 'club'}
										<div class="space-y-3">
											<ClubSelector
												bind:value={selectedClub}
												placeholder="Select a club..."
												onValueChange={handleClubChange}
											/>

											{#if clubDevices.length > 0}
												<div class="space-y-2">
													<div class="flex items-center justify-between">
														<span
															class="text-sm font-medium text-surface-700 dark:text-surface-200"
														>
															Club Aircraft ({clubDevices.length})
														</span>
														<button
															class="btn preset-filled-primary-500 btn-sm"
															onclick={addAllClubDevices}
															disabled={clubSearchInProgress}
														>
															<Plus size={16} />
															Add All
														</button>
													</div>

													<div
														class="max-h-48 overflow-y-auto rounded border border-surface-300 bg-surface-100 p-2 dark:border-surface-600 dark:bg-surface-800"
													>
														<div class="grid gap-2">
															{#each clubDevices as device (device.id)}
																<div
																	class="flex items-center justify-between rounded bg-surface-50 p-2 shadow-sm dark:bg-surface-700"
																>
																	<div class="min-w-0 flex-1">
																		<div class="truncate text-sm font-medium">
																			{device.registration || 'Unknown Registration'}
																		</div>
																		<div
																			class="truncate text-xs text-surface-600 dark:text-surface-400"
																		>
																			{device.aircraft_model || 'Unknown Model'}
																		</div>
																	</div>
																	{#if isDeviceInWatchlist(device.id)}
																		<span
																			class="rounded bg-green-100 px-2 py-1 text-xs font-medium text-green-800 dark:bg-green-900 dark:text-green-100"
																		>
																			Watched
																		</span>
																	{:else}
																		<button
																			class="btn preset-filled-primary-500 btn-sm"
																			onclick={() => addDeviceToWatchlist(device)}
																		>
																			<Plus size={14} />
																			Watch
																		</button>
																	{/if}
																</div>
															{/each}
														</div>
													</div>
												</div>
											{:else if clubSearchInProgress}
												<div
													class="py-4 text-center text-sm text-surface-600 dark:text-surface-400"
												>
													<div
														class="mx-auto mb-2 h-4 w-4 animate-spin rounded-full border-2 border-surface-300 border-t-primary-500 dark:border-surface-600"
													></div>
													Loading club aircraft...
												</div>
											{:else if selectedClub.length > 0}
												<div
													class="py-4 text-center text-sm text-surface-600 dark:text-surface-400"
												>
													No aircraft found for this club.
												</div>
											{/if}

											<!-- Club error message display -->
											{#if clubErrorMessage}
												<div
													class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600 dark:border-red-800 dark:bg-red-950 dark:text-red-400"
												>
													{clubErrorMessage}
												</div>
											{/if}
										</div>
									{/if}
								</div>
							</div>
						</div>

						{#if newWatchlistEntry.type !== 'club'}
							<button
								class="btn w-full preset-filled-primary-500 btn-sm"
								onclick={addWatchlistEntry}
								disabled={searchInProgress}
							>
								{#if searchInProgress}
									<div
										class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
									></div>
									Searching...
								{:else}
									<Plus size={16} />
									Add to Watchlist
								{/if}
							</button>
						{/if}

						<!-- Error message display -->
						{#if errorMessage}
							<div
								class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600 dark:border-red-800 dark:bg-red-950 dark:text-red-400"
							>
								{errorMessage}
							</div>
						{/if}
					</div>
				</section>

				<!-- Watchlist entries - takes remaining space -->
				<section class="flex min-h-0 flex-1 flex-col">
					<h3
						class="mb-3 flex flex-shrink-0 flex-row items-center align-middle text-lg font-semibold"
					>
						<Eye size={16} /> Watched Aircraft ({entriesWithDevices.length})
					</h3>
					{#if entriesWithDevices.length > 0}
						<div class="flex-1 overflow-y-auto">
							<div class="grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
								{#each entriesWithDevices as entry (entry.deviceId)}
									<div
										class="rounded border border-surface-300 p-3 dark:border-surface-600 {entry.active
											? 'bg-surface-50 dark:bg-surface-800'
											: 'bg-surface-100 opacity-75 dark:bg-surface-700'}"
									>
										<div class="flex flex-col space-y-2">
											<div class="flex items-start justify-between">
												<div class="min-w-0 flex-1">
													<div class="space-y-1">
														<div class="flex items-center gap-2">
															<span class="truncate text-lg font-medium"
																>{entry.device.registration || 'Unknown Registration'}</span
															>
															{#if entry.device.competition_number}
																<span
																	class="flex-shrink-0 rounded bg-blue-100 px-2 py-1 text-xs font-medium text-blue-800 dark:bg-blue-900 dark:text-blue-100"
																	>{entry.device.competition_number}</span
																>
															{/if}
														</div>
														<div class="truncate text-sm text-surface-700 dark:text-surface-300">
															{entry.device.aircraft_model || 'Unknown Aircraft Model'}
														</div>
														{#if entry.device.club_id}
															<div class="truncate text-xs text-surface-600 dark:text-surface-400">
																Club: {clubNames.get(entry.device.club_id) || 'Loading...'}
															</div>
														{/if}
														<div class="text-xs text-surface-600 dark:text-surface-400">
															<div class="truncate">
																{getAddressTypeLabel(entry.device.address_type)}: {entry.device
																	.address}
															</div>
															<div class="mt-1 flex flex-wrap gap-1">
																{#if entry.device.tracked}
																	<span class="text-green-600">• Tracked</span>
																{/if}
																{#if entry.device.identified}
																	<span class="text-blue-600">• Identified</span>
																{/if}
															</div>
														</div>
													</div>
												</div>
											</div>
											<div class="flex items-center justify-between pt-1">
												<Switch
													checked={entry.active}
													onCheckedChange={() => toggleWatchlistEntry(entry.id)}
												>
													<Switch.Control>
														<Switch.Thumb />
													</Switch.Control>
													<Switch.HiddenInput name="watchlist-{entry.id}" />
												</Switch>
												<button
													class="preset-tonal-error-500 btn btn-sm"
													onclick={() => removeWatchlistEntry(entry.id)}
												>
													<X size={16} />
												</button>
											</div>
										</div>
									</div>
								{/each}
							</div>
						</div>
					{:else}
						<div class="flex flex-1 items-center justify-center">
							<p class="text-center text-sm text-surface-600 dark:text-surface-400">
								No aircraft in watchlist
							</p>
						</div>
					{/if}
				</section>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Loading spinner animation */
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.animate-spin {
		animation: spin 1s linear infinite;
	}
</style>
