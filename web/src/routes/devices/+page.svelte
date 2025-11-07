<script lang="ts">
	import { Search, Radio, Plane, Antenna, Building2, Check, X, Activity } from '@lucide/svelte';
	import { SegmentedControl } from '@skeletonlabs/skeleton-svelte';
	import { resolve } from '$app/paths';
	import { serverCall } from '$lib/api/server';
	import ClubSelector from '$lib/components/ClubSelector.svelte';
	import { onMount } from 'svelte';
	import {
		formatDeviceAddress,
		getAircraftTypeOgnDescription,
		getAircraftTypeColor
	} from '$lib/formatters';

	interface Device {
		id?: string;
		device_address: string;
		address_type: string;
		address: string;
		aircraft_model: string;
		registration: string;
		competition_number: string;
		tracked: boolean;
		identified: boolean;
		from_ddb?: boolean;
		aircraft_type_ogn?: string;
		created_at?: string;
		updated_at?: string;
	}

	let devices: Device[] = [];
	let loading = false;
	let error = '';
	let searchQuery = '';
	let searchType: 'registration' | 'device' | 'club' = 'registration';
	let deviceAddressType = 'I'; // ICAO, OGN, FLARM

	// Pagination state
	let currentPage = 0;
	let pageSize = 50;
	$: totalPages = Math.ceil(devices.length / pageSize);
	$: paginatedDevices = devices.slice(currentPage * pageSize, (currentPage + 1) * pageSize);

	// Club search state
	let selectedClub: string[] = [];
	let clubDevices: Device[] = [];
	let clubSearchInProgress = false;
	let clubErrorMessage = '';

	async function loadRecentDevices() {
		loading = true;
		error = '';
		currentPage = 0;

		try {
			const response = await serverCall<{ devices: Device[] }>('/devices');
			devices = response.devices || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load recent devices: ${errorMessage}`;
			console.error('Error loading recent devices:', err);
			devices = [];
		} finally {
			loading = false;
		}
	}

	async function searchDevices() {
		if (!searchQuery.trim()) {
			error = 'Please enter a search query';
			return;
		}

		loading = true;
		error = '';
		currentPage = 0; // Reset to first page on new search

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

	onMount(() => {
		loadRecentDevices();
	});

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
		currentPage = 0; // Reset to first page on new search

		try {
			const response = await serverCall<{ devices: Device[] }>(`/clubs/${clubId}/devices`);
			// Only update if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubDevices = response.devices || [];
				// Set the main devices list to show club devices
				devices = clubDevices;
			}
		} catch (err) {
			console.warn(`Failed to fetch devices for club:`, err);
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
	function handleClubChange(event: { value: string[] }) {
		selectedClub = event.value;
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

	function goToPage(page: number) {
		if (page >= 0 && page < totalPages) {
			currentPage = page;
		}
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
		<h3 class="mb-3 flex items-center gap-2 text-lg font-semibold">
			<Search class="h-5 w-5" />
			Search Aircraft Devices
		</h3>
		<div class="space-y-3 rounded-lg border p-3">
			<!-- Mobile: Vertical layout (segment above inputs) -->
			<div class="space-y-3 md:hidden">
				<!-- Search type selector -->
				<SegmentedControl
					name="search-type"
					value={searchType}
					orientation="vertical"
					onValueChange={(event: { value: string | null }) => {
						if (
							event.value &&
							(event.value === 'registration' || event.value === 'device' || event.value === 'club')
						) {
							searchType = event.value;
							error = '';
							clubErrorMessage = '';
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
									<Antenna size={16} />
									<span class="ml-1">Device Address</span>
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

				{#if searchType === 'registration'}
					<input
						class="input"
						placeholder="Aircraft registration (e.g., N12345)"
						bind:value={searchQuery}
						onkeydown={handleKeydown}
						oninput={() => (error = '')}
					/>
				{:else if searchType === 'device'}
					<div class="space-y-3">
						<SegmentedControl
							name="address-type"
							value={deviceAddressType}
							orientation="vertical"
							onValueChange={(event: { value: string | null }) => {
								if (event.value) {
									deviceAddressType = event.value;
									error = '';
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
			</div>

			<!-- Desktop: Horizontal layout (segment to the left of inputs) -->
			<div class="hidden md:block">
				<div class="grid grid-cols-[200px_1fr] items-start gap-4">
					<!-- Search type selector -->
					<SegmentedControl
						name="search-type-desktop"
						value={searchType}
						orientation="vertical"
						onValueChange={(event: { value: string | null }) => {
							if (
								event.value &&
								(event.value === 'registration' ||
									event.value === 'device' ||
									event.value === 'club')
							) {
								searchType = event.value;
								error = '';
								clubErrorMessage = '';
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
										<Antenna size={16} />
										<span class="ml-1">Device Address</span>
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
						{#if searchType === 'registration'}
							<input
								class="input"
								placeholder="Aircraft registration (e.g., N12345)"
								bind:value={searchQuery}
								onkeydown={handleKeydown}
								oninput={() => (error = '')}
							/>
						{:else if searchType === 'device'}
							<div class="flex items-start gap-3">
								<SegmentedControl
									name="address-type-desktop"
									value={deviceAddressType}
									orientation="vertical"
									onValueChange={(event: { value: string | null }) => {
										if (event.value) {
											deviceAddressType = event.value;
											error = '';
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
									class="input flex-1"
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
					</div>
				</div>
			</div>

			{#if searchType !== 'club'}
				<button
					class="btn w-full preset-filled-primary-500"
					onclick={searchDevices}
					disabled={loading}
				>
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

	<!-- Results Cards -->
	{#if !loading && devices.length > 0}
		<section class="space-y-4">
			<!-- Results Header -->
			<div class="flex items-center justify-between">
				<div>
					<h2 class="h2">
						{#if !searchQuery && searchType !== 'club'}
							<div class="flex items-center gap-2">
								<Activity class="h-6 w-6" />
								Recently Active Devices
							</div>
						{:else}
							Search Results
						{/if}
					</h2>
					<p class="text-surface-500-400-token">
						{#if !searchQuery && searchType !== 'club'}
							{devices.length} device{devices.length === 1 ? '' : 's'} heard from recently
						{:else}
							{devices.length} device{devices.length === 1 ? '' : 's'} found
						{/if}
						{#if totalPages > 1}
							(showing {currentPage * pageSize + 1}-{Math.min(
								(currentPage + 1) * pageSize,
								devices.length
							)})
						{/if}
					</p>
				</div>
			</div>

			<!-- Device Cards Grid -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each paginatedDevices as device (device.id || device.address)}
					<a
						href={resolve(`/devices/${device.id}`)}
						class="group card p-5 card-hover transition-all hover:scale-[1.02]"
					>
						<!-- Header Section -->
						<div class="mb-4 flex items-start justify-between">
							<div class="flex items-center gap-2">
								<Radio class="h-5 w-5 text-primary-500" />
								<div>
									<h3 class="font-mono text-lg font-bold group-hover:text-primary-500">
										{device.device_address}
									</h3>
									<p class="text-surface-600-300-token text-xs">
										{formatDeviceAddress(device.address_type, device.address)}
									</p>
								</div>
							</div>
						</div>

						<!-- Registration and Model -->
						<div class="mb-4 space-y-2">
							<div class="flex items-center gap-2">
								<Plane class="h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token text-xs">Registration</p>
									<p class="text-sm font-semibold">
										{device.registration || 'Unknown'}
									</p>
								</div>
							</div>
							<div class="flex items-center gap-2">
								<Antenna class="h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token text-xs">Aircraft Model</p>
									<p class="text-sm">{device.aircraft_model || 'Unknown'}</p>
								</div>
							</div>
							{#if device.competition_number}
								<div class="flex items-center gap-2">
									<Activity class="h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token text-xs">Competition Number</p>
										<p class="font-mono text-sm">{device.competition_number}</p>
									</div>
								</div>
							{/if}
						</div>

						<!-- Status Badges -->
						<div class="flex flex-wrap gap-2">
							<span
								class="badge text-xs {device.tracked
									? 'preset-filled-success-500'
									: 'preset-filled-surface-500'}"
							>
								{#if device.tracked}
									<Check class="mr-1 h-3 w-3" />
								{:else}
									<X class="mr-1 h-3 w-3" />
								{/if}
								{device.tracked ? 'Tracked' : 'Not Tracked'}
							</span>
							<span
								class="badge text-xs {device.identified
									? 'preset-filled-primary-500'
									: 'preset-filled-surface-500'}"
							>
								{#if device.identified}
									<Check class="mr-1 h-3 w-3" />
								{:else}
									<X class="mr-1 h-3 w-3" />
								{/if}
								{device.identified ? 'Identified' : 'Unidentified'}
							</span>
							{#if device.from_ddb}
								<span class="badge preset-filled-success-500 text-xs">
									<Check class="mr-1 h-3 w-3" />
									OGN DB
								</span>
							{/if}
							{#if device.aircraft_type_ogn}
								<span class="badge {getAircraftTypeColor(device.aircraft_type_ogn)} text-xs">
									{getAircraftTypeOgnDescription(device.aircraft_type_ogn)}
								</span>
							{/if}
						</div>
					</a>
				{/each}
			</div>

			<!-- Pagination Controls -->
			{#if totalPages > 1}
				<div class="flex items-center justify-between card p-4">
					<div class="text-surface-500-400-token text-sm">
						Page {currentPage + 1} of {totalPages}
					</div>
					<div class="flex gap-2">
						<button
							class="btn preset-filled-surface-500 btn-sm"
							onclick={() => goToPage(0)}
							disabled={currentPage === 0}
						>
							First
						</button>
						<button
							class="btn preset-filled-surface-500 btn-sm"
							onclick={() => goToPage(currentPage - 1)}
							disabled={currentPage === 0}
						>
							Previous
						</button>
						<button
							class="btn preset-filled-surface-500 btn-sm"
							onclick={() => goToPage(currentPage + 1)}
							disabled={currentPage >= totalPages - 1}
						>
							Next
						</button>
						<button
							class="btn preset-filled-surface-500 btn-sm"
							onclick={() => goToPage(totalPages - 1)}
							disabled={currentPage >= totalPages - 1}
						>
							Last
						</button>
					</div>
				</div>
			{/if}
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
				<p class="text-surface-500-400-token">No aircraft found for the selected club.</p>
			</div>
		</div>
	{/if}
</div>
