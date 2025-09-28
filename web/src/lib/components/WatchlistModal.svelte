<script lang="ts">
	import { Switch, Segment } from '@skeletonlabs/skeleton-svelte';
	import { Plus, X, Plane, Antenna, Eye } from '@lucide/svelte';
	import { browser } from '$app/environment';
	import { serverCall } from '$lib/api/server';
	import { watchlist } from '$lib/stores/watchlist';
	import { DeviceRegistry } from '$lib/services/DeviceRegistry';
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

	// Clear error message when user interacts with form
	function clearError() {
		errorMessage = '';
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

	// Load watchlist on mount
	$effect(() => {
		if (browser) {
			watchlist.loadFromStorage();
		}
	});

	// Get devices from registry for watchlist entries
	const deviceRegistry = $derived(DeviceRegistry.getInstance());
	const entriesWithDevices = $derived($watchlist.entries.map(entry => {
		const device = deviceRegistry.getDevice(entry.deviceId);
		return {
			...entry,
			device: device || {
				id: entry.deviceId,
				registration: 'Unknown',
				aircraft_model: 'Unknown',
				address_type: 'Unknown',
				address: 'Unknown',
				cn: '',
				tracked: false,
				identified: false
			}
		};
	}));
</script>

<!-- Watchlist Modal -->
{#if showModal}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-surface-950-50/50"
		role="dialog"
		onclick={() => (showModal = false)}
		onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
		tabindex="-1"
	>
		<div
			class="flex h-full max-h-9/10 w-full max-w-9/10 flex-col card bg-white text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
			role="dialog"
			tabindex="0"
		>
			<!-- Header -->
			<div class="flex-shrink-0 p-4 pb-0">
				<div class="mb-4 flex items-center justify-between">
					<h2 class="text-xl font-bold">Watchlist</h2>
					<button class="variant-ghost-surface btn btn-sm" onclick={() => (showModal = false)}>
						<X size={20} />
					</button>
				</div>
			</div>

			<!-- Content area with flex layout -->
			<div class="flex min-h-0 flex-1 flex-col space-y-6 p-4 pt-0">
				<!-- Add new entry -->
				<section class="flex-shrink-0">
					<h3 class="mb-3 text-lg font-semibold">Add Aircraft</h3>
					<div class="mb-3 space-y-3 rounded-lg border p-3">
						<!-- Segment selector for type -->
						<Segment
							name="watchlist-type"
							value={newWatchlistEntry.type}
							onValueChange={(e) => {
								if (e.value) {
									newWatchlistEntry.type = e.value;
									clearError();
								}
							}}
						>
							<Segment.Item value="registration">
								<div class="flex flex-row items-center">
									<Plane size={16} />
									<span class="ml-1">Registration</span>
								</div>
							</Segment.Item>
							<Segment.Item value="device">
								<div class="flex flex-row items-center">
									<Antenna size={16} />
									<span class="ml-1">Device</span>
								</div>
							</Segment.Item>
						</Segment>

						{#if newWatchlistEntry.type === 'registration'}
							<input
								class="input"
								placeholder="Aircraft registration (e.g., N12345)"
								bind:value={newWatchlistEntry.registration}
								onkeydown={(e) => e.key === 'Enter' && addWatchlistEntry()}
								oninput={() => clearError()}
								disabled={searchInProgress}
							/>
						{:else}
							<div class="grid grid-cols-2 gap-2">
								<Segment
									name="address-type"
									value={newWatchlistEntry.deviceAddressType}
									onValueChange={(e) => {
										if (e.value) {
											newWatchlistEntry.deviceAddressType = e.value;
											clearError();
										}
									}}
								>
									<Segment.Item value="I">ICAO</Segment.Item>
									<Segment.Item value="O">OGN</Segment.Item>
									<Segment.Item value="F">FLARM</Segment.Item>
								</Segment>
								<input
									class="input"
									placeholder="Device address"
									bind:value={newWatchlistEntry.deviceAddress}
									onkeydown={(e) => e.key === 'Enter' && addWatchlistEntry()}
									oninput={() => clearError()}
									disabled={searchInProgress}
								/>
							</div>
						{/if}

						<button
							class="variant-filled-primary btn w-full btn-sm"
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

						<!-- Error message display -->
						{#if errorMessage}
							<div class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600">
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
								{#each entriesWithDevices as entry (entry.id)}
									<div
										class="rounded border p-3 {entry.active
											? 'bg-gray-50'
											: 'bg-gray-100 opacity-75'}"
									>
										<div class="flex flex-col space-y-2">
											<div class="flex items-start justify-between">
												<div class="min-w-0 flex-1">
													<div class="space-y-1">
														<div class="flex items-center gap-2">
															<span class="truncate text-lg font-medium"
																>{entry.device.registration || 'Unknown Registration'}</span
															>
															{#if entry.device.cn}
																<span
																	class="flex-shrink-0 rounded bg-blue-100 px-2 py-1 text-xs font-medium text-blue-800"
																	>{entry.device.cn}</span
																>
															{/if}
														</div>
														<div class="truncate text-sm text-gray-600">
															{entry.device.aircraft_model || 'Unknown Aircraft Model'}
														</div>
														<div class="text-xs text-gray-500">
															<div class="truncate">
																{entry.device.address_type}: {entry.device.address}
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
													name="watchlist-{entry.id}"
													checked={entry.active}
													onCheckedChange={() => toggleWatchlistEntry(entry.id)}
												/>
												<button
													class="variant-ghost-error btn btn-sm"
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
							<p class="text-center text-sm text-gray-500">No aircraft in watchlist</p>
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
