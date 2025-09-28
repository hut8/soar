<script lang="ts">
	import { Search, Radio, Plane, User, Antenna, Building2 } from '@lucide/svelte';
	import { Segment } from '@skeletonlabs/skeleton-svelte';
	import { resolve } from '$app/paths';
	import { serverCall } from '$lib/api/server';
	import ClubSelector from '$lib/components/ClubSelector.svelte';

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
	let searchType: 'registration' | 'device' | 'club' = 'registration';
	let deviceAddressType = 'I'; // ICAO, OGN, FLARM

	// Club search state
	let selectedClub: string[] = [];
	let clubDevices: Device[] = [];
	let clubSearchInProgress = false;
	let clubErrorMessage = '';

	function formatDeviceId(deviceId: number): string {
		// Convert integer device_id to 6-digit hex string
		return deviceId.toString(16).toUpperCase().padStart(6, '0');
	}

	async function searchDevices() {
		if (!searchQuery.trim()) {
			error = 'Please enter a search query';
			return;
		}

		loading = true;
		error = '';

		try {
			let endpoint = '/devices?';

			if (searchType === 'registration') {
				endpoint += `registration=${encodeURIComponent(searchQuery.trim())}`;
			} else {
				// Device address search
				const address = searchQuery.trim().toUpperCase();
				endpoint += `address=${encodeURIComponent(address)}&address-type=${encodeURIComponent(deviceAddressType)}`;
			}

			const response = await serverCall<{ devices: Device[] }>(endpoint);
			devices = response.devices || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search devices: ${errorMessage}`;
			console.error('Error searching devices:', err);
			devices = [];
		} finally {
			loading = false;
		}
	}

	// Clear club error message
	function clearClubError() {
		clubErrorMessage = '';
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
				// Set the main devices list to show club devices
				devices = clubDevices;
			}
		} catch (error) {
			console.warn(`Failed to fetch devices for club:`, error);
			// Only show error if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubErrorMessage = 'Failed to load club devices. Please try again.';
				clubDevices = [];
				devices = [];
			}
		} finally {
			clubSearchInProgress = false;
		}
	}

	// Handle club selection change
	function handleClubChange(e: { value: string[] }) {
		selectedClub = e.value;
		clearClubError();

		if (selectedClub.length > 0) {
			loadClubDevices();
		} else {
			clubDevices = [];
			devices = [];
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

	// Don't load devices automatically on mount - wait for user search
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
	</header>

	<!-- Search Section -->
	<section class="space-y-4 card p-6">
		<h3 class="mb-3 text-lg font-semibold flex items-center gap-2">
			<Search class="h-5 w-5" />
			Search Aircraft Devices
		</h3>
		<div class="space-y-3 rounded-lg border p-3">
			<!-- Search type selector -->
			<Segment
				name="search-type"
				value={searchType}
				class="flex flex-col sm:flex-row"
				onValueChange={(e) => {
					if (e.value && (e.value === 'registration' || e.value === 'device' || e.value === 'club')) {
						searchType = e.value;
						error = '';
						clubErrorMessage = '';
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
						<span class="ml-1">Device Address</span>
					</div>
				</Segment.Item>
				<Segment.Item value="club">
					<div class="flex flex-row items-center">
						<Building2 size={16} />
						<span class="ml-1">Club</span>
					</div>
				</Segment.Item>
			</Segment>

			{#if searchType === 'registration'}
				<input
					class="input"
					placeholder="Aircraft registration (e.g., N12345)"
					bind:value={searchQuery}
					onkeydown={handleKeydown}
					oninput={() => (error = '')}
				/>
			{:else if searchType === 'device'}
				<div class="grid grid-cols-2 gap-2">
					<Segment
						name="address-type"
						value={deviceAddressType}
						onValueChange={(e) => {
							if (e.value) {
								deviceAddressType = e.value;
								error = '';
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
						bind:value={searchQuery}
						onkeydown={handleKeydown}
						oninput={() => (error = '')}
					/>
				</div>
			{:else if searchType === 'club'}
				<div class="space-y-3">
					<ClubSelector
						bind:value={selectedClub}
						placeholder="Select a club..."
						onValueChange={handleClubChange}
					/>

					<!-- Club error message display -->
					{#if clubErrorMessage}
						<div class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600">
							{clubErrorMessage}
						</div>
					{/if}
				</div>
			{/if}

			{#if searchType !== 'club'}
				<button class="variant-filled-primary btn w-full" onclick={searchDevices} disabled={loading}>
					{#if loading}
						<div
							class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
						></div>
						Searching...
					{:else}
						<Search class="mr-2 h-4 w-4" />
						Search Devices
					{/if}
				</button>
			{/if}

			<!-- Error message display -->
			{#if error}
				<div class="rounded border border-red-200 bg-red-50 p-2 text-sm text-red-600">
					{error}
				</div>
			{/if}
		</div>
	</section>

	<!-- Results Table -->
	{#if !loading && devices.length > 0}
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
	{:else if !loading && devices.length === 0 && searchQuery}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No devices found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or search for a different device.
				</p>
			</div>
		</div>
	{:else if !loading && devices.length === 0 && searchType === 'club' && selectedClub.length > 0}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No devices found</h3>
				<p class="text-surface-500-400-token">
					No aircraft found for the selected club.
				</p>
			</div>
		</div>
	{/if}
</div>
