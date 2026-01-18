<script lang="ts">
	import { Switch, SegmentedControl } from '@skeletonlabs/skeleton-svelte';
	import { Plus, X, Plane, Radio, Eye, Building2 } from '@lucide/svelte';
	import { browser } from '$app/environment';
	import { serverCall } from '$lib/api/server';
	import { watchlist } from '$lib/stores/watchlist';
	import { auth } from '$lib/stores/auth';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import ClubSelector from '$lib/components/ClubSelector.svelte';
	import { getAddressTypeLabel } from '$lib/formatters';
	import type { Aircraft, DataListResponse, DataResponse, Club } from '$lib/types';
	import { getLogger } from '$lib/logging';
	import { toaster } from '$lib/toaster';

	const logger = getLogger(['soar', 'WatchlistModal']);

	let { showModal = $bindable() } = $props();

	// State
	let newWatchlistEntry = $state({
		type: 'registration',
		registration: '',
		aircraftAddressType: 'I',
		aircraftAddress: ''
	});
	let searchInProgress = $state(false);
	let errorMessage = $state('');

	// Club tab state
	let selectedClub = $state<string[]>([]);
	let clubAircraft = $state<Aircraft[]>([]);
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

		let aircraft: Aircraft | null = null;

		if (newWatchlistEntry.type === 'registration') {
			const registration = newWatchlistEntry.registration.trim().toUpperCase();
			if (!registration) return;

			// Search for aircraft by registration
			searchInProgress = true;
			try {
				const response = await serverCall<DataListResponse<Aircraft>>(
					`/aircraft?registration=${encodeURIComponent(registration)}`
				);
				if (response.data && response.data.length > 0) {
					aircraft = response.data[0];
				} else {
					errorMessage = `Aircraft with registration "${registration}" not found`;
				}
			} catch (error) {
				logger.warn('Failed to fetch aircraft for registration {registration}: {error}', {
					registration,
					error
				});
				errorMessage = 'Failed to search for aircraft. Please try again.';
			} finally {
				searchInProgress = false;
			}
		} else {
			const addressType = newWatchlistEntry.aircraftAddressType.trim();
			const address = newWatchlistEntry.aircraftAddress.trim().toUpperCase();
			if (!addressType || !address) return;

			// Search for aircraft by address and type
			searchInProgress = true;
			try {
				const response = await serverCall<DataListResponse<Aircraft>>(
					`/aircraft?address=${encodeURIComponent(address)}&address-type=${encodeURIComponent(addressType)}`
				);
				if (response.data && response.data.length > 0) {
					aircraft = response.data[0];
				} else {
					errorMessage = `Aircraft with address "${address}" (${addressType}) not found`;
				}
			} catch (error) {
				logger.warn('Failed to fetch aircraft for address {address} ({addressType}): {error}', {
					address,
					addressType,
					error
				});
				errorMessage = 'Failed to search for aircraft. Please try again.';
			} finally {
				searchInProgress = false;
			}
		}

		// Only add to watchlist if aircraft was found
		if (aircraft && aircraft.id) {
			// Check for duplicates
			const existingEntry = $watchlist.entries.find((entry) => entry.aircraftId === aircraft.id);
			if (existingEntry) {
				errorMessage = 'This aircraft is already in your watchlist';
				return;
			}

			// Add the aircraft to the registry and watchlist
			AircraftRegistry.getInstance().setAircraft(aircraft);
			try {
				await watchlist.add(aircraft.id, false);
				// Clear the search inputs on success
				newWatchlistEntry = {
					type: 'registration',
					registration: '',
					aircraftAddressType: 'I',
					aircraftAddress: ''
				};
			} catch (error) {
				logger.warn('Failed to add aircraft to watchlist: {error}', { error });
				const message = error instanceof Error ? error.message : 'Failed to add to watchlist';
				toaster.error({ title: 'Failed to add to watchlist', description: message });
			}
		}
	}

	// Remove entry from watchlist
	async function removeWatchlistEntry(id: string) {
		try {
			await watchlist.remove(id);
		} catch (error) {
			logger.warn('Failed to remove from watchlist: {error}', { error });
			const message = error instanceof Error ? error.message : 'Failed to remove from watchlist';
			toaster.error({ title: 'Failed to remove from watchlist', description: message });
		}
	}

	// Toggle email notification for entry
	async function toggleEmailNotification(id: string, currentValue: boolean) {
		try {
			await watchlist.updateEmailPreference(id, !currentValue);
		} catch (error) {
			logger.warn('Failed to update email preference: {error}', { error });
			const message = error instanceof Error ? error.message : 'Failed to update email preference';
			toaster.error({ title: 'Failed to update email preference', description: message });
		}
	}

	// Load aircraft for selected club
	async function loadClubAircraft() {
		if (!selectedClub.length || clubSearchInProgress) return;

		const clubId = selectedClub[0];
		if (!clubId) return;

		clubSearchInProgress = true;
		clubErrorMessage = '';

		try {
			const response = await serverCall<DataListResponse<Aircraft>>(`/clubs/${clubId}/aircraft`);
			// Only update if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubAircraft = response.data || [];
			}
		} catch (error) {
			logger.warn('Failed to fetch aircraft for club: {error}', { error });
			// Only show error if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubErrorMessage = 'Failed to load club aircraft. Please try again.';
				clubAircraft = [];
			}
		} finally {
			clubSearchInProgress = false;
		}
	}

	// Add individual aircraft to watchlist
	async function addAircraftToWatchlist(aircraft: Aircraft) {
		// Check for duplicates
		const existingEntry = $watchlist.entries.find((entry) => entry.aircraftId === aircraft.id);
		if (existingEntry) {
			return; // Already in watchlist
		}

		AircraftRegistry.getInstance().setAircraft(aircraft);
		try {
			await watchlist.add(aircraft.id, false);
		} catch (error) {
			logger.warn('Failed to add aircraft to watchlist: {error}', { error });
			const message = error instanceof Error ? error.message : 'Failed to add to watchlist';
			toaster.error({ title: 'Failed to add to watchlist', description: message });
		}
	}

	// Add all club aircraft to watchlist
	function addAllClubAircraft() {
		if (!clubAircraft.length) return;

		clubAircraft.forEach((aircraft) => {
			addAircraftToWatchlist(aircraft);
		});
	}

	// Check if aircraft is already in watchlist
	function isAircraftInWatchlist(aircraftId: string): boolean {
		return $watchlist.entries.some((entry) => entry.aircraftId === aircraftId);
	}

	// Handle club selection change
	function handleClubChange(details: { value: string[] }) {
		selectedClub = details.value;
		clearClubError();

		if (selectedClub.length > 0) {
			loadClubAircraft();
		} else {
			clubAircraft = [];
		}
	}

	// Load watchlist on mount (only if authenticated)
	$effect(() => {
		if (browser && $auth.isAuthenticated) {
			watchlist.load();
		}
	});

	// Get aircraft from registry for watchlist entries
	const aircraftRegistry = $derived(AircraftRegistry.getInstance());
	const entriesWithAircraft = $derived(
		$watchlist.entries.map((entry) => {
			const aircraft = entry.aircraft || aircraftRegistry.getAircraft(entry.aircraftId);
			return {
				...entry,
				aircraft: aircraft || {
					id: entry.aircraftId,
					registration: 'Unknown',
					aircraftModel: 'Unknown',
					addressType: 'Unknown',
					address: 'Unknown',
					competitionNumber: '',
					tracked: false,
					identified: false,
					clubId: null,
					createdAt: '',
					updatedAt: '',
					fromOgnDdb: false,
					fromAdsbxDdb: false,
					frequencyMhz: null,
					pilotName: null,
					homeBaseAirportIdent: null,
					aircraftTypeOgn: null,
					lastFixAt: null,
					trackerDeviceType: null,
					icaoModelCode: null,
					countryCode: null,
					ownerOperator: null,
					addressCountry: null,
					latitude: null,
					longitude: null,
					adsbEmitterCategory: null,
					currentFix: null,
					fixes: null
				}
			};
		})
	);

	// Fetch club name for an aircraft
	async function fetchClubName(clubId: string): Promise<void> {
		if (clubNames.has(clubId)) {
			return; // Already fetched
		}

		try {
			const response = await serverCall<DataResponse<Club>>(`/clubs/${clubId}`);
			clubNames.set(clubId, response.data.name);
		} catch (error) {
			logger.warn('Failed to fetch club name for {clubId}: {error}', { clubId, error });
			clubNames.set(clubId, 'Unknown Club');
		}
	}

	// Effect to fetch club names when entries change
	$effect(() => {
		// Get unique club IDs from all aircraft
		const clubIds: string[] = [];
		for (const entry of entriesWithAircraft) {
			if (entry.aircraft.clubId && !clubIds.includes(entry.aircraft.clubId)) {
				clubIds.push(entry.aircraft.clubId);
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
				{#if !$auth.isAuthenticated}
					<!-- Unauthenticated message -->
					<div
						class="flex flex-1 flex-col items-center justify-center space-y-4 rounded-lg border-2 border-dashed border-surface-300 p-8 text-center dark:border-surface-600"
					>
						<div class="text-surface-500 dark:text-surface-400">
							<Eye size={48} class="mx-auto mb-4" />
							<h3 class="mb-2 text-lg font-semibold text-surface-900 dark:text-surface-50">
								Sign in to use Watchlist
							</h3>
							<p class="mb-4 max-w-md">
								The Watchlist feature allows you to track specific aircraft and receive
								notifications when they fly. Sign in or create an account to get started.
							</p>
							<div class="flex flex-col gap-2 sm:flex-row sm:justify-center">
								<a
									href="/login"
									class="variant-filled-primary btn"
									onclick={() => (showModal = false)}
								>
									Sign In
								</a>
								<a
									href="/register"
									class="variant-filled-secondary btn"
									onclick={() => (showModal = false)}
								>
									Create Account
								</a>
							</div>
						</div>
					</div>
				{:else}
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
									<SegmentedControl.Control>
										<SegmentedControl.Indicator />
										<SegmentedControl.Item value="registration">
											<SegmentedControl.ItemText>
												<div class="flex flex-row items-center">
													<Plane size={16} />
													<span class="ml-1">Registration</span>
												</div>
											</SegmentedControl.ItemText>
											<SegmentedControl.ItemHiddenInput />
										</SegmentedControl.Item>
										<SegmentedControl.Item value="device">
											<SegmentedControl.ItemText>
												<div class="flex flex-row items-center">
													<Radio size={16} />
													<span class="ml-1">Aircraft</span>
												</div>
											</SegmentedControl.ItemText>
											<SegmentedControl.ItemHiddenInput />
										</SegmentedControl.Item>
										<SegmentedControl.Item value="club">
											<SegmentedControl.ItemText>
												<div class="flex flex-row items-center">
													<Building2 size={16} />
													<span class="ml-1">Club</span>
												</div>
											</SegmentedControl.ItemText>
											<SegmentedControl.ItemHiddenInput />
										</SegmentedControl.Item>
									</SegmentedControl.Control>
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
											value={newWatchlistEntry.aircraftAddressType}
											orientation="vertical"
											onValueChange={(details) => {
												if (details.value) {
													newWatchlistEntry.aircraftAddressType = details.value;
													clearError();
												}
											}}
										>
											<SegmentedControl.Control>
												<SegmentedControl.Indicator />
												<SegmentedControl.Item value="I">
													<SegmentedControl.ItemText>ICAO</SegmentedControl.ItemText>
													<SegmentedControl.ItemHiddenInput />
												</SegmentedControl.Item>
												<SegmentedControl.Item value="O">
													<SegmentedControl.ItemText>OGN</SegmentedControl.ItemText>
													<SegmentedControl.ItemHiddenInput />
												</SegmentedControl.Item>
												<SegmentedControl.Item value="F">
													<SegmentedControl.ItemText>FLARM</SegmentedControl.ItemText>
													<SegmentedControl.ItemHiddenInput />
												</SegmentedControl.Item>
											</SegmentedControl.Control>
										</SegmentedControl>
										<input
											class="input"
											placeholder="Aircraft address"
											bind:value={newWatchlistEntry.aircraftAddress}
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

										{#if clubAircraft.length > 0}
											<div class="space-y-2">
												<div class="flex items-center justify-between">
													<span class="text-sm font-medium text-surface-700 dark:text-surface-200">
														Club Aircraft ({clubAircraft.length})
													</span>
													<button
														class="btn preset-filled-primary-500 btn-sm"
														onclick={addAllClubAircraft}
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
														{#each clubAircraft as aircraft (aircraft.id)}
															<div
																class="flex items-center justify-between rounded bg-surface-50 p-2 shadow-sm dark:bg-surface-700"
															>
																<div class="min-w-0 flex-1">
																	<div class="truncate text-sm font-medium">
																		{aircraft.registration || 'Unknown Registration'}
																	</div>
																	<div
																		class="truncate text-xs text-surface-600 dark:text-surface-400"
																	>
																		{aircraft.aircraftModel || 'Unknown Model'}
																	</div>
																</div>
																{#if isAircraftInWatchlist(aircraft.id)}
																	<span
																		class="rounded bg-green-100 px-2 py-1 text-xs font-medium text-green-800 dark:bg-green-900 dark:text-green-100"
																	>
																		Watched
																	</span>
																{:else}
																	<button
																		class="btn preset-filled-primary-500 btn-sm"
																		onclick={() => addAircraftToWatchlist(aircraft)}
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
										<SegmentedControl.Control>
											<SegmentedControl.Indicator />
											<SegmentedControl.Item value="registration">
												<SegmentedControl.ItemText>
													<div class="flex flex-row items-center">
														<Plane size={16} />
														<span class="ml-1">Registration</span>
													</div>
												</SegmentedControl.ItemText>
												<SegmentedControl.ItemHiddenInput />
											</SegmentedControl.Item>
											<SegmentedControl.Item value="device">
												<SegmentedControl.ItemText>
													<div class="flex flex-row items-center">
														<Radio size={16} />
														<span class="ml-1">Aircraft</span>
													</div>
												</SegmentedControl.ItemText>
												<SegmentedControl.ItemHiddenInput />
											</SegmentedControl.Item>
											<SegmentedControl.Item value="club">
												<SegmentedControl.ItemText>
													<div class="flex flex-row items-center">
														<Building2 size={16} />
														<span class="ml-1">Club</span>
													</div>
												</SegmentedControl.ItemText>
												<SegmentedControl.ItemHiddenInput />
											</SegmentedControl.Item>
										</SegmentedControl.Control>
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
													value={newWatchlistEntry.aircraftAddressType}
													orientation="vertical"
													onValueChange={(details) => {
														if (details.value) {
															newWatchlistEntry.aircraftAddressType = details.value;
															clearError();
														}
													}}
												>
													<SegmentedControl.Control>
														<SegmentedControl.Indicator />
														<SegmentedControl.Item value="I">
															<SegmentedControl.ItemText>ICAO</SegmentedControl.ItemText>
															<SegmentedControl.ItemHiddenInput />
														</SegmentedControl.Item>
														<SegmentedControl.Item value="O">
															<SegmentedControl.ItemText>OGN</SegmentedControl.ItemText>
															<SegmentedControl.ItemHiddenInput />
														</SegmentedControl.Item>
														<SegmentedControl.Item value="F">
															<SegmentedControl.ItemText>FLARM</SegmentedControl.ItemText>
															<SegmentedControl.ItemHiddenInput />
														</SegmentedControl.Item>
													</SegmentedControl.Control>
												</SegmentedControl>
												<input
													class="input"
													placeholder="Aircraft address"
													bind:value={newWatchlistEntry.aircraftAddress}
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

												{#if clubAircraft.length > 0}
													<div class="space-y-2">
														<div class="flex items-center justify-between">
															<span
																class="text-sm font-medium text-surface-700 dark:text-surface-200"
															>
																Club Aircraft ({clubAircraft.length})
															</span>
															<button
																class="btn preset-filled-primary-500 btn-sm"
																onclick={addAllClubAircraft}
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
																{#each clubAircraft as aircraft (aircraft.id)}
																	<div
																		class="flex items-center justify-between rounded bg-surface-50 p-2 shadow-sm dark:bg-surface-700"
																	>
																		<div class="min-w-0 flex-1">
																			<div class="truncate text-sm font-medium">
																				{aircraft.registration || 'Unknown Registration'}
																			</div>
																			<div
																				class="truncate text-xs text-surface-600 dark:text-surface-400"
																			>
																				{aircraft.aircraftModel || 'Unknown Model'}
																			</div>
																		</div>
																		{#if isAircraftInWatchlist(aircraft.id)}
																			<span
																				class="rounded bg-green-100 px-2 py-1 text-xs font-medium text-green-800 dark:bg-green-900 dark:text-green-100"
																			>
																				Watched
																			</span>
																		{:else}
																			<button
																				class="btn preset-filled-primary-500 btn-sm"
																				onclick={() => addAircraftToWatchlist(aircraft)}
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
							<Eye size={16} /> Watched Aircraft ({entriesWithAircraft.length})
						</h3>
						{#if entriesWithAircraft.length > 0}
							<div class="flex-1 overflow-y-auto">
								<div class="grid grid-cols-1 gap-3 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
									{#each entriesWithAircraft as entry (entry.aircraftId)}
										<div
											class="rounded border border-surface-300 bg-surface-50 p-3 dark:border-surface-600 dark:bg-surface-800"
										>
											<div class="flex flex-col space-y-2">
												<div class="flex items-start justify-between">
													<div class="min-w-0 flex-1">
														<div class="space-y-1">
															<div class="flex items-center gap-2">
																<span class="truncate text-lg font-medium"
																	>{entry.aircraft.registration || 'Unknown Registration'}</span
																>
																{#if entry.aircraft.competitionNumber}
																	<span
																		class="flex-shrink-0 rounded bg-blue-100 px-2 py-1 text-xs font-medium text-blue-800 dark:bg-blue-900 dark:text-blue-100"
																		>{entry.aircraft.competitionNumber}</span
																	>
																{/if}
															</div>
															<div class="truncate text-sm text-surface-700 dark:text-surface-300">
																{entry.aircraft.aircraftModel || 'Unknown Aircraft Model'}
															</div>
															{#if entry.aircraft.clubId}
																<div
																	class="truncate text-xs text-surface-600 dark:text-surface-400"
																>
																	Club: {clubNames.get(entry.aircraft.clubId) || 'Loading...'}
																</div>
															{/if}
															<div class="text-xs text-surface-600 dark:text-surface-400">
																<div class="truncate">
																	{getAddressTypeLabel(entry.aircraft.addressType)}: {entry.aircraft
																		.address}
																</div>
																<div class="mt-1 flex flex-wrap gap-1">
																	{#if entry.aircraft.tracked}
																		<span class="text-green-600">• Tracked</span>
																	{/if}
																	{#if entry.aircraft.identified}
																		<span class="text-blue-600">• Identified</span>
																	{/if}
																</div>
															</div>
														</div>
													</div>
												</div>
												<div class="flex items-center justify-between pt-1">
													<Switch
														class="flex justify-between p-2"
														checked={entry.sendEmail}
														onCheckedChange={() =>
															toggleEmailNotification(entry.aircraftId, entry.sendEmail)}
													>
														<Switch.Label class="text-sm font-medium">Email</Switch.Label>
														<Switch.Control>
															<Switch.Thumb />
														</Switch.Control>
														<Switch.HiddenInput name="watchlist-{entry.aircraftId}" />
													</Switch>
													<button
														class="preset-tonal-error-500 btn btn-sm"
														onclick={() => removeWatchlistEntry(entry.aircraftId)}
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
				{/if}
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
