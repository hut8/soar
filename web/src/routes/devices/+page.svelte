<script lang="ts">
	import { onMount } from 'svelte';
	import { Search, Radio, Plane, ExternalLink, User } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';

	interface Device {
		device_id: number;
		device_type: string;
		aircraft_model: string;
		registration: string;
		competition_number: string;
		tracked: boolean;
		identified: boolean;
		user_id?: string;
		created_at: string;
		updated_at: string;
	}

	let devices: Device[] = [];
	let loading = false;
	let error = '';
	let searchQuery = '';
	let searchType: 'device_id' | 'registration' = 'device_id';

	function formatDeviceId(deviceId: number): string {
		// Convert integer device_id to 6-digit hex string
		return deviceId.toString(16).toUpperCase().padStart(6, '0');
	}

	function parseDeviceId(hexString: string): number | null {
		// Convert 6-digit hex string to integer
		const cleaned = hexString.replace(/[^a-fA-F0-9]/g, '');
		if (cleaned.length !== 6) return null;
		
		const parsed = parseInt(cleaned, 16);
		return isNaN(parsed) ? null : parsed;
	}

	async function searchDevices() {
		if (!searchQuery.trim()) {
			devices = [];
			return;
		}

		loading = true;
		error = '';

		try {
			let endpoint = '/devices?';
			
			if (searchType === 'device_id') {
				const deviceId = parseDeviceId(searchQuery.trim());
				if (deviceId === null) {
					error = 'Invalid device ID format. Please enter a 6-digit hex code (e.g., AABBCC)';
					devices = [];
					loading = false;
					return;
				}
				endpoint += `device_id=${deviceId}`;
			} else {
				endpoint += `registration=${encodeURIComponent(searchQuery.trim())}`;
			}

			devices = await serverCall<Device[]>(endpoint);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search devices: ${errorMessage}`;
			console.error('Error searching devices:', err);
			devices = [];
		} finally {
			loading = false;
		}
	}

	async function loadAllDevices() {
		loading = true;
		error = '';

		try {
			devices = await serverCall<Device[]>('/devices?limit=50');
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load devices: ${errorMessage}`;
			console.error('Error loading devices:', err);
		} finally {
			loading = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			searchDevices();
		}
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	onMount(() => {
		loadAllDevices();
	});
</script>

<svelte:head>
	<title>Devices - Aircraft Tracking</title>
</svelte:head>

<div class="container mx-auto p-4 space-y-6 max-w-7xl">
	<!-- Header -->
	<header class="text-center space-y-2">
		<h1 class="h1 flex items-center justify-center gap-2">
			<Radio class="w-8 h-8" />
			Aircraft Devices
		</h1>
		<p class="text-surface-600-300-token">Search and manage aircraft tracking devices</p>
	</header>

	<!-- Search Section -->
	<section class="card p-6 space-y-4">
		<div class="flex flex-col md:flex-row gap-4">
			<!-- Search Type Toggle -->
			<div class="flex gap-2">
				<button
					class="btn btn-sm {searchType === 'device_id' ? 'variant-filled' : 'variant-soft'}"
					on:click={() => (searchType = 'device_id')}
				>
					<Radio class="w-4 h-4 mr-2" />
					Device ID
				</button>
				<button
					class="btn btn-sm {searchType === 'registration' ? 'variant-filled' : 'variant-soft'}"
					on:click={() => (searchType = 'registration')}
				>
					<Plane class="w-4 h-4 mr-2" />
					Registration
				</button>
			</div>

			<!-- Search Input -->
			<div class="flex-1 flex gap-2">
				<input
					bind:value={searchQuery}
					on:keydown={handleKeydown}
					class="input flex-1"
					type="text"
					placeholder={searchType === 'device_id' 
						? 'Enter device ID (e.g., AABBCC)' 
						: 'Enter registration number (e.g., N123AB)'}
				/>
				<button class="btn variant-filled" on:click={searchDevices}>
					<Search class="w-4 h-4 mr-2" />
					Search
				</button>
			</div>
		</div>

		<div class="flex justify-center">
			<button class="btn variant-soft" on:click={loadAllDevices}>
				Show Recent Devices
			</button>
		</div>
	</section>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Searching devices...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert variant-filled-error">
			<div class="alert-message">
				<h3 class="h3">Search Error</h3>
				<p>{error}</p>
			</div>
		</div>
	{/if}

	<!-- Results Table -->
	{#if !loading && !error && devices.length > 0}
		<section class="card">
			<header class="card-header">
				<h2 class="h2">Search Results</h2>
				<p class="text-surface-500-400-token">{devices.length} device{devices.length === 1 ? '' : 's'} found</p>
			</header>

			<div class="table-container">
				<table class="table table-hover">
					<thead>
						<tr>
							<th>Device ID</th>
							<th>Registration</th>
							<th>Aircraft Model</th>
							<th>Type</th>
							<th>Competition #</th>
							<th>Status</th>
							<th>Owner</th>
							<th>Updated</th>
						</tr>
					</thead>
					<tbody>
						{#each devices as device}
							<tr>
								<td>
									<a 
										href="/devices/{formatDeviceId(device.device_id)}" 
										class="anchor text-primary-500 hover:text-primary-600 font-mono"
									>
										{formatDeviceId(device.device_id)}
									</a>
								</td>
								<td class="font-semibold">{device.registration}</td>
								<td>{device.aircraft_model}</td>
								<td>
									<span class="badge variant-soft">
										{device.device_type}
									</span>
								</td>
								<td>{device.competition_number || '—'}</td>
								<td>
									<div class="flex flex-col gap-1">
										<span class="badge {device.tracked ? 'variant-filled-success' : 'variant-filled-surface'} text-xs">
											{device.tracked ? 'Tracked' : 'Not Tracked'}
										</span>
										<span class="badge {device.identified ? 'variant-filled-primary' : 'variant-filled-surface'} text-xs">
											{device.identified ? 'Identified' : 'Unidentified'}
										</span>
									</div>
								</td>
								<td>
									{#if device.user_id}
										<User class="w-4 h-4 inline mr-1" />
										<span class="text-xs">Assigned</span>
									{:else}
										<span class="text-surface-500 text-xs">Unassigned</span>
									{/if}
								</td>
								<td class="text-sm text-surface-600-300-token">
									{formatDate(device.updated_at)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{:else if !loading && !error && devices.length === 0 && searchQuery}
		<div class="card p-12 text-center space-y-4">
			<Search class="w-16 h-16 mx-auto text-surface-400 mb-4" />
			<div class="space-y-2">
				<h3 class="h3">No devices found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or search for a different device.
				</p>
			</div>
		</div>
	{/if}
</div>