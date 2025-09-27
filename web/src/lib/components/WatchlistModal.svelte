<script lang="ts">
	import { Switch, Segment } from '@skeletonlabs/skeleton-svelte';
	import { Plus, X, Plane, Antenna, Eye } from '@lucide/svelte';
	import { browser } from '$app/environment';
	import { serverCall } from '$lib/api/server';
	import type { Device } from '$lib/types';
	let { showModal = $bindable() } = $props();

	// Enhanced WatchlistEntry with full device data
	interface WatchlistEntry {
		id: string;
		type: 'registration' | 'device';
		registration?: string;
		deviceAddressType?: string;
		deviceAddress?: string;
		device?: Device; // Full device data from API
		active: boolean;
	}

	// State
	let watchlist: WatchlistEntry[] = $state([]);
	let newWatchlistEntry = $state({
		type: 'registration',
		registration: '',
		deviceAddressType: 'I',
		deviceAddress: ''
	});
	let searchInProgress = $state(false);

	// Load watchlist from localStorage
	function loadWatchlist() {
		if (!browser) return;

		const saved = localStorage.getItem('watchlist');
		if (saved) {
			try {
				watchlist = JSON.parse(saved);
			} catch (e) {
				console.warn('Failed to load watchlist from localStorage:', e);
				watchlist = [];
			}
		}
	}

	// Save watchlist to localStorage
	function saveWatchlist() {
		if (!browser) return;
		localStorage.setItem('watchlist', JSON.stringify(watchlist));
	}

	// Add entry to watchlist
	async function addWatchlistEntry() {
		if (searchInProgress) return;

		const entry: WatchlistEntry = {
			id: Date.now().toString(),
			type: newWatchlistEntry.type as 'registration' | 'device',
			active: true
		};

		if (newWatchlistEntry.type === 'registration') {
			const registration = newWatchlistEntry.registration.trim().toUpperCase();
			if (!registration) return;

			entry.registration = registration;

			// Search for device by registration
			searchInProgress = true;
			try {
				const response = await serverCall<{ devices: Device[] }>(
					`/devices?registration=${encodeURIComponent(registration)}`
				);
				if (response.devices && response.devices.length > 0) {
					entry.device = response.devices[0];
				}
			} catch (error) {
				console.warn(`Failed to fetch device for registration ${registration}:`, error);
			} finally {
				searchInProgress = false;
			}
		} else {
			const addressType = newWatchlistEntry.deviceAddressType.trim();
			const address = newWatchlistEntry.deviceAddress.trim().toUpperCase();
			if (!addressType || !address) return;

			entry.deviceAddressType = addressType;
			entry.deviceAddress = address;

			// Search for device by address and type
			searchInProgress = true;
			try {
				const response = await serverCall<{ devices: Device[] }>(
					`/devices?address=${encodeURIComponent(address)}&address-type=${encodeURIComponent(addressType)}`
				);
				if (response.devices && response.devices.length > 0) {
					entry.device = response.devices[0];
				}
			} catch (error) {
				console.warn(`Failed to fetch device for address ${address} (${addressType}):`, error);
			} finally {
				searchInProgress = false;
			}
		}

		watchlist = [...watchlist, entry];
		newWatchlistEntry = {
			type: 'registration',
			registration: '',
			deviceAddressType: 'I',
			deviceAddress: ''
		};
		saveWatchlist();
	}

	// Remove entry from watchlist
	function removeWatchlistEntry(id: string) {
		watchlist = watchlist.filter((entry) => entry.id !== id);
		saveWatchlist();
	}

	// Toggle entry active state
	function toggleWatchlistEntry(id: string) {
		watchlist = watchlist.map((entry) =>
			entry.id === id ? { ...entry, active: !entry.active } : entry
		);
		saveWatchlist();
	}

	// Load watchlist on mount
	$effect(() => {
		if (browser) {
			loadWatchlist();
		}
	});
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
			class="h-full max-h-9/10 w-full max-w-9/10 overflow-y-auto card bg-white p-4 text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
            onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
            role="dialog"
            tabindex="0"
		>
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-xl font-bold">Watchlist</h2>
				<button class="variant-ghost-surface btn btn-sm" onclick={() => (showModal = false)}>
					<X size={20} />
				</button>
			</div>

			<div class="space-y-6">
				<!-- Add new entry -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Add Aircraft</h3>
					<div class="mb-3 space-y-3 rounded-lg border p-3">
						<!-- Segment selector for type -->
						<Segment
							name="watchlist-type"
							value={newWatchlistEntry.type}
							onValueChange={(e) => e.value && (newWatchlistEntry.type = e.value)}
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
								disabled={searchInProgress}
							/>
						{:else}
							<div class="grid grid-cols-2 gap-2">
								<Segment
									name="address-type"
									value={newWatchlistEntry.deviceAddressType}
									onValueChange={(e) => e.value && (newWatchlistEntry.deviceAddressType = e.value)}
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
									disabled={searchInProgress}
								/>
							</div>
						{/if}

						<button
							class="variant-filled-primary btn btn-sm w-full"
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
					</div>
				</section>

				<!-- Watchlist entries -->
				<section>
					<h3 class="mb-3 text-lg font-semibold align-middle flex flex-row items-center"><Eye size={16} /> Watched Aircraft ({watchlist.length})</h3>
					{#if watchlist.length > 0}
						<div class="max-h-48 space-y-2 overflow-y-auto">
							{#each watchlist as entry (entry.id)}
								<div
									class="rounded border p-3 {entry.active
										? 'bg-gray-50'
										: 'bg-gray-100 opacity-75'}"
								>
									<div class="flex items-start justify-between">
										<div class="flex-1">
											{#if entry.device}
												<!-- Show full device data -->
												<div class="space-y-1">
													<div class="flex items-center gap-2">
														<span class="text-lg font-medium"
															>{entry.device.registration || 'Unknown Registration'}</span
														>
														{#if entry.device.cn}
															<span
																class="rounded bg-blue-100 px-2 py-1 text-xs font-medium text-blue-800"
																>{entry.device.cn}</span
															>
														{/if}
													</div>
													<div class="text-sm text-gray-600">
														{entry.device.aircraft_model || 'Unknown Aircraft Model'}
													</div>
													<div class="text-xs text-gray-500">
														{entry.device.address_type}: {entry.device.address}
														{#if entry.device.tracked}
															<span class="ml-2 text-green-600">• Tracked</span>
														{/if}
														{#if entry.device.identified}
															<span class="ml-2 text-blue-600">• Identified</span>
														{/if}
													</div>
												</div>
											{:else}
												<!-- Show basic info when no device data available -->
												<div class="space-y-1">
													{#if entry.type === 'registration'}
														<span class="font-medium">{entry.registration}</span>
														<div class="text-xs text-gray-500">Registration</div>
													{:else}
														<span class="font-medium"
															>{entry.deviceAddressType}: {entry.deviceAddress}</span
														>
														<div class="text-xs text-gray-500">Device</div>
													{/if}
													<div class="text-xs text-orange-600">• Device not found</div>
												</div>
											{/if}
										</div>
										<div class="ml-3 flex items-center gap-2">
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
					{:else}
						<p class="py-4 text-center text-sm text-gray-500">No aircraft in watchlist</p>
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
