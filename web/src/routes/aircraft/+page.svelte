<script lang="ts">
	import {
		Search,
		Radio,
		Plane,
		Antenna,
		Building2,
		Activity,
		Filter,
		AlertTriangle
	} from '@lucide/svelte';
	import { SegmentedControl } from '@skeletonlabs/skeleton-svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import { serverCall } from '$lib/api/server';
	import ClubSelector from '$lib/components/ClubSelector.svelte';
	import AircraftTile from '$lib/components/AircraftTile.svelte';
	import { onMount } from 'svelte';
	import type { Aircraft } from '$lib/types';

	let aircraft = $state<Aircraft[]>([]);
	let loading = $state(false);
	let error = $state('');
	let searchQuery = $state('');
	let searchType = $state<'registration' | 'device' | 'club'>('registration');
	let aircraftAddressType = $state('I'); // ICAO, OGN, FLARM

	// Aircraft type filter state (for recently active aircraft only)
	let selectedAircraftTypes = new SvelteSet<string>();
	let filterExpanded = $state(false); // Track whether filter panel is expanded

	// Available aircraft types for filtering
	const aircraftTypes = [
		{ value: 'glider', label: 'Glider' },
		{ value: 'tow_tug', label: 'Tow/Tug' },
		{ value: 'recip_engine', label: 'Reciprocating Engine' },
		{ value: 'jet_turboprop', label: 'Jet/Turboprop' },
		{ value: 'helicopter_gyro', label: 'Helicopter/Gyro' },
		{ value: 'paraglider', label: 'Paraglider' },
		{ value: 'hang_glider', label: 'Hang Glider' },
		{ value: 'skydiver_parachute', label: 'Skydiver/Parachute' },
		{ value: 'drop_plane', label: 'Drop Plane' },
		{ value: 'balloon', label: 'Balloon' },
		{ value: 'airship', label: 'Airship' },
		{ value: 'uav', label: 'UAV' },
		{ value: 'static_obstacle', label: 'Static Obstacle' }
	];

	// Pagination state
	let currentPage = $state(0);
	let pageSize = 50;

	// Pagination - no filtering on frontend, backend handles it
	let totalPages = $derived(Math.ceil(aircraft.length / pageSize));
	let paginatedAircraft = $derived(
		aircraft.slice(currentPage * pageSize, (currentPage + 1) * pageSize)
	);

	// Club search state
	let selectedClub = $state<string[]>([]);
	let clubAircraft = $state<Aircraft[]>([]);
	let clubSearchInProgress = $state(false);
	let clubErrorMessage = $state('');

	async function loadRecentAircraft() {
		loading = true;
		error = '';
		currentPage = 0;

		try {
			// Build query parameters
			let endpoint = '/aircraft';
			if (selectedAircraftTypes.size > 0) {
				const typesParam = Array.from(selectedAircraftTypes).join(',');
				endpoint += `?aircraft-types=${encodeURIComponent(typesParam)}`;
			}

			const response = await serverCall<{ aircraft: Aircraft[] }>(endpoint);
			aircraft = response.aircraft || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load recent aircraft: ${errorMessage}`;
			console.error('Error loading recent aircraft:', err);
			aircraft = [];
		} finally {
			loading = false;
		}
	}

	async function searchAircraft() {
		if (!searchQuery.trim()) {
			error = 'Please enter a search query';
			return;
		}

		loading = true;
		error = '';
		currentPage = 0; // Reset to first page on new search

		try {
			let endpoint = '/aircraft?';

			if (searchType === 'registration') {
				endpoint += `registration=${encodeURIComponent(searchQuery.trim())}`;
			} else {
				// Aircraft address search
				const address = searchQuery.trim().toUpperCase();
				endpoint += `address=${encodeURIComponent(address)}&address-type=${encodeURIComponent(aircraftAddressType)}`;
			}

			const response = await serverCall<{ aircraft: Aircraft[] }>(endpoint);
			aircraft = response.aircraft || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to search aircraft: ${errorMessage}`;
			console.error('Error searching aircraft:', err);
			aircraft = [];
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadRecentAircraft();
	});

	// Clear club error message
	function clearClubError() {
		clubErrorMessage = '';
	}

	// Load aircraft for selected club
	async function loadClubDevices() {
		if (!selectedClub.length || clubSearchInProgress) return;

		const clubId = selectedClub[0];
		if (!clubId) return;

		clubSearchInProgress = true;
		clubErrorMessage = '';
		currentPage = 0; // Reset to first page on new search

		try {
			const response = await serverCall<{ aircraft: Aircraft[] }>(`/clubs/${clubId}/aircraft`);
			// Only update if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubAircraft = response.aircraft || [];
				// Set the main aircraft list to show club aircraft
				aircraft = clubAircraft;
			}
		} catch (err) {
			console.warn(`Failed to fetch aircraft for club:`, err);
			// Only show error if we're still looking at the same club
			if (selectedClub.length > 0 && selectedClub[0] === clubId) {
				clubErrorMessage = 'Failed to load club aircraft. Please try again.';
				clubAircraft = [];
				aircraft = [];
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
			clubAircraft = [];
			aircraft = [];
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			searchAircraft();
		}
	}

	function goToPage(page: number) {
		if (page >= 0 && page < totalPages) {
			currentPage = page;
		}
	}

	function toggleAircraftType(type: string) {
		if (selectedAircraftTypes.has(type)) {
			selectedAircraftTypes.delete(type);
		} else {
			selectedAircraftTypes.add(type);
		}
		currentPage = 0; // Reset to first page when filter changes
		loadRecentAircraft(); // Reload from backend with new filter
	}

	function toggleFilterExpanded() {
		if (filterExpanded) {
			// Collapsing - clear all filters and reload all aircraft
			filterExpanded = false;
			selectedAircraftTypes.clear();
			currentPage = 0;
			loadRecentAircraft();
		} else {
			// Expanding - show filter options (starting with nothing selected)
			filterExpanded = true;
			// If there were previously selected types, keep them
			// Otherwise the filter starts empty and shows no results until types are selected
		}
	}

	// Don't load aircraft automatically on mount - wait for user search
</script>

<svelte:head>
	<title>Aircraft - SOAR Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Radio class="h-8 w-8" />
			Aircraft
		</h1>
	</header>

	<!-- Issues Button -->
	<div class="flex justify-center">
		<a href="/aircraft/issues" class="btn preset-filled-warning-500">
			<AlertTriangle class="h-5 w-5" />
			View Aircraft Issues
		</a>
	</div>

	<!-- Search Section -->
	<section class="space-y-4 card p-6">
		<h3 class="mb-3 flex items-center gap-2 text-lg font-semibold">
			<Search class="h-5 w-5" />
			Search Aircraft
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
									<span class="ml-1">Aircraft Address</span>
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
							value={aircraftAddressType}
							orientation="vertical"
							onValueChange={(event: { value: string | null }) => {
								if (event.value) {
									aircraftAddressType = event.value;
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
										<span class="ml-1">Aircraft Address</span>
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
									value={aircraftAddressType}
									orientation="vertical"
									onValueChange={(event: { value: string | null }) => {
										if (event.value) {
											aircraftAddressType = event.value;
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
					onclick={searchAircraft}
					disabled={loading}
				>
					{#if loading}
						<div
							class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
						></div>
						Searching...
					{:else}
						<Search class="mr-2 h-4 w-4" />
						Search Aircraft
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

	<!-- Aircraft Type Filter (only for recently active aircraft) -->
	{#if !searchQuery && searchType !== 'club' && !loading}
		<section class="space-y-4 card p-6">
			<!-- Filter toggle button -->
			<button
				class="btn w-full {filterExpanded
					? 'preset-filled-primary-500'
					: 'preset-filled-surface-500'}"
				onclick={toggleFilterExpanded}
			>
				<Filter class="h-5 w-5" />
				Filter by Aircraft Type
			</button>

			<!-- Aircraft type selection (shown when expanded) -->
			{#if filterExpanded}
				<div class="grid grid-cols-2 gap-2 md:grid-cols-7">
					{#each aircraftTypes as type (type.value)}
						<button
							class="badge text-xs transition-all {selectedAircraftTypes.has(type.value)
								? 'preset-filled-primary-500'
								: 'preset-filled-error-500'}"
							onclick={() => toggleAircraftType(type.value)}
						>
							{type.label}
						</button>
					{/each}
				</div>
			{/if}
		</section>
	{/if}

	<!-- Results Cards -->
	{#if !loading && aircraft.length > 0}
		<section class="space-y-4">
			<!-- Results Header -->
			<div class="flex items-center justify-between">
				<h2 class="h2">
					{#if !searchQuery && searchType !== 'club'}
						<div class="flex items-center gap-2">
							<Activity class="h-6 w-6" />
							Recently Active Aircraft
						</div>
					{:else}
						Search Results
					{/if}
				</h2>
			</div>

			<!-- Aircraft Cards Grid -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each paginatedAircraft as aircraft (aircraft.id || aircraft.address)}
					<AircraftTile {aircraft} />
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
	{:else if !loading && aircraft.length === 0 && searchQuery}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No aircraft found</h3>
				<p class="text-surface-500-400-token">
					Try adjusting your search criteria or search for a different aircraft.
				</p>
			</div>
		</div>
	{:else if !loading && aircraft.length === 0 && searchType === 'club' && selectedClub.length > 0}
		<div class="space-y-4 card p-12 text-center">
			<Search class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No aircraft found</h3>
				<p class="text-surface-500-400-token">No aircraft found for the selected club.</p>
			</div>
		</div>
	{/if}
</div>
