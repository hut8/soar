<script lang="ts">
	import { onMount } from 'svelte';
	import { Search, Radio, Plane, User } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { resolve } from '$app/paths';
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

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Radio class="h-8 w-8" />
			Aircraft Devices
		</h1>
		<p class="text-surface-600-300-token">Search and manage aircraft tracking devices</p>
	</header>

	<!-- Search Section -->
	<section class="space-y-4 card p-6">
		<div class="flex flex-col gap-4 md:flex-row">
			<!-- Search Type Toggle -->
			<div class="flex gap-2">
				<button
					class="btn btn-sm {searchType === 'device_id' ? 'variant-filled' : 'variant-soft'}"
					on:click={() => (searchType = 'device_id')}
				>
					<Radio class="mr-2 h-4 w-4" />
					Device ID
				</button>
				<button
					class="btn btn-sm {searchType === 'registration' ? 'variant-filled' : 'variant-soft'}"
					on:click={() => (searchType = 'registration')}
				>
					<Plane class="mr-2 h-4 w-4" />
					Registration
				</button>
			</div>

			<!-- Search Input -->
			<div class="flex flex-1 gap-2">
				<input
					bind:value={searchQuery}
					on:keydown={handleKeydown}
					class="input flex-1"
					type="text"
					placeholder={searchType === 'device_id'
						? 'Enter device ID (e.g., AABBCC)'
						: 'Enter registration number (e.g., N123AB)'}
				/>
				<button class="variant-filled btn" on:click={searchDevices}>
					<Search class="mr-2 h-4 w-4" />
					Search
				</button>
			</div>
		</div>

		<div class="flex justify-center">
			<button class="variant-soft btn" on:click={loadAllDevices}> Show Recent Devices </button>
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
				<p class="text-surface-500-400-token">
					{devices.length} device{devices.length === 1 ? '' : 's'} found
				</p>
			</header>

			<div class="table-container">
				<table class="table-hover table">
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
						{#each devices as device (device.device_id)}
							<tr>
								<td>
									<a
										href={resolve(`/devices/${formatDeviceId(device.device_id)}`)}
										class="anchor font-mono text-primary-500 hover:text-primary-600"
									>
										{formatDeviceId(device.device_id)}
									</a>
								</td>
								<td class="font-semibold">{device.registration}</td>
								<td>{device.aircraft_model}</td>
								<td>
									<span class="variant-soft badge">
										{device.device_type}
									</span>
								</td>
								<td>{device.competition_number || '—'}</td>
								<td>
									<div class="flex flex-col gap-1">
										<span
											class="badge {device.tracked
												? 'variant-filled-success'
												: 'variant-filled-surface'} text-xs"
										>
											{device.tracked ? 'Tracked' : 'Not Tracked'}
										</span>
										<span
											class="badge {device.identified
												? 'variant-filled-primary'
												: 'variant-filled-surface'} text-xs"
										>
											{device.identified ? 'Identified' : 'Unidentified'}
										</span>
									</div>
								</td>
								<td>
									{#if device.user_id}
										<User class="mr-1 inline h-4 w-4" />
										<span class="text-xs">Assigned</span>
									{:else}
										<span class="text-xs text-surface-500">Unassigned</span>
									{/if}
								</td>
								<td class="text-surface-600-300-token text-sm">
									{formatDate(device.updated_at)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{:else if !loading && !error && devices.length === 0 && searchQuery}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No devices found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or search for a different device.
				</p>
			</div>
		</div>
	{/if}
</div>
