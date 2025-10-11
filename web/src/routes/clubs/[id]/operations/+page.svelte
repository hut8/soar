<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		Plane,
		Clock,
		MapPin,
		ExternalLink,
		MoveUp,
		ArrowLeft,
		Calendar,
		UserPlus
	} from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { getAircraftTypeOgnDescription, getAircraftTypeColor } from '$lib/formatters';
	import { auth } from '$lib/stores/auth';
	import PilotSelectionModal from '$lib/components/PilotSelectionModal.svelte';

	dayjs.extend(relativeTime);

	interface Flight {
		id: string;
		device_address: string;
		device_address_type: string;
		takeoff_time: string | null;
		landing_time: string | null;
		departure_airport: string | null;
		departure_airport_country: string | null;
		arrival_airport: string | null;
		arrival_airport_country: string | null;
		takeoff_runway_ident: string | null;
		landing_runway_ident: string | null;
		tow_aircraft_id: string | null;
		tow_release_height_msl: number | null;
		takeoff_altitude_offset_ft: number | null;
		landing_altitude_offset_ft: number | null;
		total_distance_meters: number | null;
		device_id: string | null;
		aircraft_model: string | null;
		registration: string | null;
		aircraft_type_ogn: string | null;
		created_at: string;
		updated_at: string;
	}

	interface Club {
		id: string;
		name: string;
	}

	let club: Club | null = null;
	let selectedDate = dayjs().format('YYYY-MM-DD');
	let flightsInProgress: Flight[] = [];
	let completedFlights: Flight[] = [];
	let loadingClub = true;
	let loadingFlights = true;
	let error = '';
	let flightsError = '';

	// Pilot modal state
	let pilotModalOpen = $state(false);
	let selectedFlightId = $state('');

	let clubId = $derived($page.params.id || '');

	// Check if user belongs to this club
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.club_id === clubId);

	onMount(async () => {
		if (clubId) {
			await loadClub();
			await loadFlights();
		}
	});

	// Reload flights when date changes
	$effect(() => {
		if (selectedDate && clubId) {
			loadFlights();
		}
	});

	async function loadClub() {
		loadingClub = true;
		error = '';

		try {
			club = await serverCall<Club>(`/clubs/${clubId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load club: ${errorMessage}`;
			console.error('Error loading club:', err);
		} finally {
			loadingClub = false;
		}
	}

	async function loadFlights() {
		loadingFlights = true;
		flightsError = '';

		try {
			// TODO: Replace with actual API endpoints for club-specific flights
			// For now, fetch all flights and filter by date
			const allCompleted = await serverCall<Flight[]>('/flights?completed=true&limit=100');

			// Filter flights for the selected date
			const selectedDateStart = dayjs(selectedDate).startOf('day');
			const selectedDateEnd = dayjs(selectedDate).endOf('day');

			completedFlights = (allCompleted || []).filter((flight) => {
				if (!flight.takeoff_time) return false;
				const takeoffDate = dayjs(flight.takeoff_time);
				return takeoffDate.isAfter(selectedDateStart) && takeoffDate.isBefore(selectedDateEnd);
			});

			// TODO: Add API endpoint for flights in progress
			flightsInProgress = [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			flightsError = `Failed to load flights: ${errorMessage}`;
			console.error('Error loading flights:', err);
		} finally {
			loadingFlights = false;
		}
	}

	function formatDeviceAddress(address: string, addressType: string): string {
		const typePrefix = addressType === 'Flarm' ? 'F' : addressType === 'Ogn' ? 'O' : 'I';
		return `${typePrefix}-${address}`;
	}

	function formatRelativeTime(dateString: string | null): string {
		if (!dateString) return '—';
		return dayjs(dateString).fromNow();
	}

	function formatLocalTime(dateString: string | null): string {
		if (!dateString) return '';
		return dayjs(dateString).format('HH:mm');
	}

	function calculateFlightDuration(takeoff: string | null, landing: string | null): string {
		if (!takeoff || !landing) return '—';
		const takeoffTime = new Date(takeoff).getTime();
		const landingTime = new Date(landing).getTime();
		const durationMs = landingTime - takeoffTime;
		const hours = Math.floor(durationMs / (1000 * 60 * 60));
		const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	}

	function formatDistance(meters: number | null): string {
		if (meters === null || meters === undefined) return '—';
		const nm = meters / 1852;
		const km = meters / 1000;
		if (nm >= 1) {
			return `${nm.toFixed(1)} nm`;
		} else {
			return `${km.toFixed(1)} km`;
		}
	}

	function goBack() {
		goto(resolve(`/clubs/${clubId}`));
	}

	function openPilotModal(flightId: string) {
		selectedFlightId = flightId;
		pilotModalOpen = true;
	}

	function handlePilotAdded() {
		// Reload flights to show updated pilot information
		loadFlights();
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club'} Operations - Aircraft Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn preset-tonal btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Club
		</button>
	</div>

	<!-- Header -->
	<header class="card p-6">
		<div class="space-y-4">
			<div class="flex flex-wrap items-center justify-between gap-4">
				<div>
					<h1 class="flex items-center gap-2 h1">
						<Plane class="h-8 w-8" />
						{club?.name || 'Club'} Operations
					</h1>
					{#if loadingClub}
						<p class="text-surface-500-400-token">Loading...</p>
					{:else if error}
						<p class="text-error-500">{error}</p>
					{/if}
				</div>
			</div>

			<!-- Date Picker -->
			<div class="flex items-center gap-3">
				<Calendar class="h-5 w-5 text-surface-500" />
				<label class="flex items-center gap-2">
					<span class="text-surface-600-300-token text-sm font-medium">Date:</span>
					<input
						type="date"
						bind:value={selectedDate}
						class="input px-3 py-2"
						max={dayjs().format('YYYY-MM-DD')}
					/>
				</label>
			</div>
		</div>
	</header>

	<!-- Loading State -->
	{#if loadingFlights}
		<div class="space-y-4 card p-12 text-center">
			<div
				class="mx-auto h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
			<p class="text-surface-500-400-token">Loading flights...</p>
		</div>
	{:else if flightsError}
		<!-- Error State -->
		<div class="space-y-4 card p-12 text-center">
			<div class="rounded border border-red-200 bg-red-50 p-4 text-red-600">
				{flightsError}
			</div>
			<button class="btn preset-filled-primary-500" onclick={loadFlights}> Try Again </button>
		</div>
	{:else}
		<!-- Flights In Progress Section -->
		<section class="card">
			<header class="card-header">
				<h2 class="h2">Flights In Progress</h2>
				<p class="text-surface-500-400-token">
					{flightsInProgress.length} flight{flightsInProgress.length === 1 ? '' : 's'} currently active
				</p>
			</header>

			{#if flightsInProgress.length === 0}
				<div class="space-y-4 p-12 text-center">
					<Plane class="mx-auto mb-4 h-16 w-16 text-surface-400" />
					<div class="space-y-2">
						<h3 class="h3">No flights in progress</h3>
						<p class="text-surface-500-400-token">There are currently no active flights.</p>
					</div>
				</div>
			{:else}
				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Aircraft</th>
								<th>Type</th>
								<th>Takeoff</th>
								<th>Duration</th>
								<th>Actions</th>
							</tr>
						</thead>
						<tbody>
							{#each flightsInProgress as flight (flight.id)}
								<tr>
									<td>
										<div class="flex flex-col gap-1">
											{#if flight.aircraft_model && flight.registration}
												<div class="flex items-center gap-2">
													{#if flight.device_id}
														<a
															href={`/devices/${flight.device_id}`}
															class="anchor font-medium text-primary-500 hover:text-primary-600"
														>
															{flight.aircraft_model}
															<span class="text-surface-500-400-token text-sm font-normal"
																>({flight.registration})</span
															>
														</a>
													{:else}
														<span class="font-medium"
															>{flight.aircraft_model}
															<span class="text-surface-500-400-token text-sm font-normal"
																>({flight.registration})</span
															></span
														>
													{/if}
												</div>
											{:else if flight.registration}
												{#if flight.device_id}
													<a
														href={`/devices/${flight.device_id}`}
														class="anchor font-medium text-primary-500 hover:text-primary-600"
													>
														{flight.registration}
													</a>
												{:else}
													<span class="font-medium">{flight.registration}</span>
												{/if}
											{:else if flight.device_id}
												<a
													href={`/devices/${flight.device_id}`}
													class="text-surface-500-400-token anchor font-mono text-sm hover:text-primary-500"
												>
													{formatDeviceAddress(flight.device_address, flight.device_address_type)}
												</a>
											{:else}
												<span class="text-surface-500-400-token font-mono text-sm">
													{formatDeviceAddress(flight.device_address, flight.device_address_type)}
												</span>
											{/if}
										</div>
									</td>
									<td>
										{#if flight.aircraft_type_ogn}
											<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
												{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
											</span>
										{:else}
											<span class="text-surface-500">—</span>
										{/if}
									</td>
									<td>
										<div class="flex flex-col gap-1">
											<div class="flex items-center gap-1 text-sm">
												<Clock class="h-3 w-3" />
												{formatRelativeTime(flight.takeoff_time)}
											</div>
											{#if flight.takeoff_time}
												<div class="text-surface-500-400-token text-xs">
													{formatLocalTime(flight.takeoff_time)}
												</div>
											{/if}
										</div>
									</td>
									<td class="font-semibold">
										{calculateFlightDuration(flight.takeoff_time, dayjs().toISOString())}
									</td>
									<td>
										<div class="flex items-center gap-2">
											{#if userBelongsToClub}
												<button
													onclick={() => openPilotModal(flight.id)}
													class="preset-tonal-primary-500 btn flex items-center gap-1 btn-sm"
													title="Add pilot to flight"
												>
													<UserPlus class="h-3 w-3" />
													Add Pilot
												</button>
											{/if}
											<a
												href={`/flights/${flight.id}`}
												target="_blank"
												rel="noopener noreferrer"
												class="preset-tonal-surface-500 btn flex items-center gap-1 btn-sm"
											>
												<ExternalLink class="h-3 w-3" />
												Open
											</a>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>

		<!-- Completed Flights Section -->
		<section class="card">
			<header class="card-header">
				<h2 class="h2">Completed Flights</h2>
				<p class="text-surface-500-400-token">
					{completedFlights.length} flight{completedFlights.length === 1 ? '' : 's'} completed on
					{dayjs(selectedDate).format('MMMM D, YYYY')}
				</p>
			</header>

			{#if completedFlights.length === 0}
				<div class="space-y-4 p-12 text-center">
					<Plane class="mx-auto mb-4 h-16 w-16 text-surface-400" />
					<div class="space-y-2">
						<h3 class="h3">No completed flights</h3>
						<p class="text-surface-500-400-token">
							No flights were completed on {dayjs(selectedDate).format('MMMM D, YYYY')}.
						</p>
					</div>
				</div>
			{:else}
				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Aircraft</th>
								<th>Type</th>
								<th>Takeoff</th>
								<th>Landing</th>
								<th>Duration</th>
								<th>Distance</th>
								<th>Tow</th>
								<th>Actions</th>
							</tr>
						</thead>
						<tbody>
							{#each completedFlights as flight (flight.id)}
								<tr>
									<td>
										<div class="flex flex-col gap-1">
											{#if flight.aircraft_model && flight.registration}
												<div class="flex items-center gap-2">
													{#if flight.device_id}
														<a
															href={`/devices/${flight.device_id}`}
															class="anchor font-medium text-primary-500 hover:text-primary-600"
														>
															{flight.aircraft_model}
															<span class="text-surface-500-400-token text-sm font-normal"
																>({flight.registration})</span
															>
														</a>
													{:else}
														<span class="font-medium"
															>{flight.aircraft_model}
															<span class="text-surface-500-400-token text-sm font-normal"
																>({flight.registration})</span
															></span
														>
													{/if}
													{#if flight.tow_aircraft_id}
														<span
															class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
															title="This aircraft was towed by {flight.tow_aircraft_id}"
														>
															<MoveUp class="h-3 w-3" />
															Towed
														</span>
													{/if}
												</div>
											{:else if flight.registration}
												<div class="flex items-center gap-2">
													{#if flight.device_id}
														<a
															href={`/devices/${flight.device_id}`}
															class="anchor font-medium text-primary-500 hover:text-primary-600"
														>
															{flight.registration}
														</a>
													{:else}
														<span class="font-medium">{flight.registration}</span>
													{/if}
													{#if flight.tow_aircraft_id}
														<span
															class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
															title="This aircraft was towed by {flight.tow_aircraft_id}"
														>
															<MoveUp class="h-3 w-3" />
															Towed
														</span>
													{/if}
												</div>
											{:else}
												<div class="flex items-center gap-2">
													{#if flight.device_id}
														<a
															href={`/devices/${flight.device_id}`}
															class="text-surface-500-400-token anchor font-mono text-sm hover:text-primary-500"
														>
															{formatDeviceAddress(
																flight.device_address,
																flight.device_address_type
															)}
														</a>
													{:else}
														<span class="text-surface-500-400-token font-mono text-sm">
															{formatDeviceAddress(
																flight.device_address,
																flight.device_address_type
															)}
														</span>
													{/if}
													{#if flight.tow_aircraft_id}
														<span
															class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
															title="This aircraft was towed by {flight.tow_aircraft_id}"
														>
															<MoveUp class="h-3 w-3" />
															Towed
														</span>
													{/if}
												</div>
											{/if}
										</div>
									</td>
									<td>
										{#if flight.aircraft_type_ogn}
											<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
												{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
											</span>
										{:else}
											<span class="text-surface-500">—</span>
										{/if}
									</td>
									<td>
										<div class="flex flex-col gap-1">
											<div class="flex items-center gap-1 text-sm">
												<Clock class="h-3 w-3" />
												{formatRelativeTime(flight.takeoff_time)}
											</div>
											{#if flight.takeoff_time}
												<div class="text-surface-500-400-token text-xs">
													{formatLocalTime(flight.takeoff_time)}
												</div>
											{/if}
											{#if flight.departure_airport}
												<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
													<MapPin class="h-3 w-3" />
													{flight.departure_airport}
												</div>
											{/if}
										</div>
									</td>
									<td>
										<div class="flex flex-col gap-1">
											<div class="flex items-center gap-1 text-sm">
												<Clock class="h-3 w-3" />
												{formatRelativeTime(flight.landing_time)}
											</div>
											{#if flight.landing_time}
												<div class="text-surface-500-400-token text-xs">
													{formatLocalTime(flight.landing_time)}
												</div>
											{/if}
											{#if flight.arrival_airport}
												<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
													<MapPin class="h-3 w-3" />
													{flight.arrival_airport}
												</div>
											{/if}
										</div>
									</td>
									<td class="font-semibold">
										{calculateFlightDuration(flight.takeoff_time, flight.landing_time)}
									</td>
									<td class="font-semibold">
										{formatDistance(flight.total_distance_meters)}
									</td>
									<td>
										{#if flight.tow_aircraft_id}
											<div class="flex flex-col gap-1">
												<span class="text-xs">{flight.tow_aircraft_id}</span>
												{#if flight.tow_release_height_msl}
													<span class="text-surface-500-400-token text-xs">
														{flight.tow_release_height_msl}m MSL
													</span>
												{/if}
											</div>
										{:else}
											<span class="text-surface-500">—</span>
										{/if}
									</td>
									<td>
										<div class="flex items-center gap-2">
											{#if userBelongsToClub}
												<button
													onclick={() => openPilotModal(flight.id)}
													class="preset-tonal-primary-500 btn flex items-center gap-1 btn-sm"
													title="Add pilot to flight"
												>
													<UserPlus class="h-3 w-3" />
													Add Pilot
												</button>
											{/if}
											<a
												href={`/flights/${flight.id}`}
												target="_blank"
												rel="noopener noreferrer"
												class="preset-tonal-surface-500 btn flex items-center gap-1 btn-sm"
											>
												<ExternalLink class="h-3 w-3" />
												Open
											</a>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>
	{/if}
</div>

<!-- Pilot Selection Modal -->
<PilotSelectionModal
	bind:isOpen={pilotModalOpen}
	{clubId}
	flightId={selectedFlightId}
	onSuccess={handlePilotAdded}
/>
