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
		UserPlus,
		ChevronLeft,
		ChevronRight,
		Users
	} from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { getAircraftCategoryDescription, getAircraftCategoryColor } from '$lib/formatters';
	import { auth } from '$lib/stores/auth';
	import { getLogger } from '$lib/logging';
	import PilotSelectionModal from '$lib/components/PilotSelectionModal.svelte';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';
	import AircraftLink from '$lib/components/AircraftLink.svelte';
	import type { Flight, FlightDetails, Aircraft, DataResponse } from '$lib/types';

	dayjs.extend(relativeTime);

	const logger = getLogger(['soar', 'ClubOperationsPage']);

	interface Club {
		id: string;
		name: string;
	}

	let club = $state<Club | null>(null);
	let selectedDate = $state(dayjs().format('YYYY-MM-DD'));
	let flightsInProgress = $state<Flight[]>([]);
	let completedFlights = $state<Flight[]>([]);
	let loadingClub = $state(true);
	let loadingFlights = $state(true);
	let error = $state('');
	let flightsError = $state('');

	// Enriched flight details with aircraft data
	let flightsInProgressDetails = $state<FlightDetails[]>([]);
	let completedFlightsDetails = $state<FlightDetails[]>([]);

	// Pilot modal state
	let pilotModalOpen = $state(false);
	let selectedFlightId = $state('');

	let clubId = $derived($page.params.id || '');

	// Check if user belongs to this club
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);

	// Check if selected date is today
	let isToday = $derived(selectedDate === dayjs().format('YYYY-MM-DD'));

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

	// Fetch aircraft data whenever flights change
	$effect(() => {
		async function fetchAircraftData() {
			// Combine all flights to get unique aircraft IDs
			const allFlights = [...flightsInProgress, ...completedFlights];
			const aircraftIds = Array.from(
				new Set(allFlights.filter((f) => f.aircraftId).map((f) => f.aircraftId!))
			);

			if (aircraftIds.length === 0) {
				// No aircraft IDs, just create FlightDetails with null aircraft
				flightsInProgressDetails = flightsInProgress.map((flight) => ({ flight, aircraft: null }));
				completedFlightsDetails = completedFlights.map((flight) => ({ flight, aircraft: null }));
				return;
			}

			try {
				// Fetch bulk aircraft data
				const response = await serverCall<DataResponse<Record<string, Aircraft>>>(
					`/aircraft/bulk?ids=${aircraftIds.join(',')}`
				);

				const aircraftMap = response.data;

				// Decorate both flight arrays
				flightsInProgressDetails = flightsInProgress.map((flight) => ({
					flight,
					aircraft: flight.aircraftId ? aircraftMap[flight.aircraftId] || null : null
				}));

				completedFlightsDetails = completedFlights.map((flight) => ({
					flight,
					aircraft: flight.aircraftId ? aircraftMap[flight.aircraftId] || null : null
				}));
			} catch (err) {
				logger.error('Failed to fetch aircraft data: {error}', { error: err });
				// On error, create FlightDetails with null aircraft
				flightsInProgressDetails = flightsInProgress.map((flight) => ({ flight, aircraft: null }));
				completedFlightsDetails = completedFlights.map((flight) => ({ flight, aircraft: null }));
			}
		}

		fetchAircraftData();
	});

	function extractErrorMessage(err: unknown): string {
		if (err instanceof Error) {
			// Try to parse the error message as JSON
			try {
				const parsed = JSON.parse(err.message);
				if (parsed && typeof parsed === 'object' && 'errors' in parsed) {
					return String(parsed.errors);
				}
			} catch {
				// Not JSON, return the original message
			}
			return err.message;
		}
		return 'Unknown error';
	}

	async function loadClub() {
		loadingClub = true;
		error = '';

		try {
			club = await serverCall<Club>(`/clubs/${clubId}`);
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load club: ${errorMessage}`;
			logger.error('Error loading club: {error}', { error: err });
		} finally {
			loadingClub = false;
		}
	}

	async function loadFlights() {
		loadingFlights = true;
		flightsError = '';

		try {
			// Format date as YYYYMMDD for the API
			const dateParam = dayjs(selectedDate).format('YYYYMMDD');

			// Fetch completed flights for this club and date
			const completedResponse = await serverCall<Flight[]>(
				`/clubs/${clubId}/flights?date=${dateParam}&completed=true`
			);
			completedFlights = completedResponse || [];

			// Only fetch flights in progress if viewing today's date
			if (selectedDate === dayjs().format('YYYY-MM-DD')) {
				const inProgressResponse = await serverCall<Flight[]>(
					`/clubs/${clubId}/flights?completed=false`
				);
				flightsInProgress = inProgressResponse || [];
			} else {
				// Clear in-progress flights when viewing historical dates
				flightsInProgress = [];
			}
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			flightsError = `Failed to load flights: ${errorMessage}`;
			logger.error('Error loading flights: {error}', { error: err });
		} finally {
			loadingFlights = false;
		}
	}

	function formatAircraftAddress(address: string, addressType: string): string {
		const typePrefix = addressType === 'Flarm' ? 'F' : addressType === 'Ogn' ? 'O' : 'I';
		return `${typePrefix}-${address}`;
	}

	function formatRelativeTime(dateString: string | null | undefined): string {
		if (!dateString) return '—';
		return dayjs(dateString).fromNow();
	}

	function formatLocalTime(dateString: string | null | undefined): string {
		if (!dateString) return '';
		return dayjs(dateString).format('HH:mm');
	}

	function calculateFlightDuration(
		takeoff: string | null | undefined,
		landing: string | null | undefined
	): string {
		if (!takeoff || !landing) return '—';
		const takeoffTime = new Date(takeoff).getTime();
		const landingTime = new Date(landing).getTime();
		const durationMs = landingTime - takeoffTime;
		const hours = Math.floor(durationMs / (1000 * 60 * 60));
		const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	}

	function formatDistance(meters: number | null | undefined): string {
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

	function goToPreviousDay() {
		selectedDate = dayjs(selectedDate).subtract(1, 'day').format('YYYY-MM-DD');
	}

	function goToNextDay() {
		const today = dayjs().format('YYYY-MM-DD');
		const nextDate = dayjs(selectedDate).add(1, 'day').format('YYYY-MM-DD');
		// Don't allow going beyond today
		if (nextDate <= today) {
			selectedDate = nextDate;
		}
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
		{#if userBelongsToClub}
			<a href={resolve(`/clubs/${clubId}/pilots`)} class="btn preset-filled-secondary-500 btn-sm">
				<Users class="mr-2 h-4 w-4" />
				Pilots
			</a>
		{/if}
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
					<div class="flex items-center gap-1">
						<button
							onclick={goToPreviousDay}
							class="preset-tonal-surface-500 btn p-2 btn-sm"
							title="Previous day"
						>
							<ChevronLeft class="h-4 w-4" />
						</button>
						<input
							type="date"
							bind:value={selectedDate}
							class="input px-3 py-2"
							max={dayjs().format('YYYY-MM-DD')}
						/>
						<button
							onclick={goToNextDay}
							class="preset-tonal-surface-500 btn p-2 btn-sm"
							title="Next day"
							disabled={selectedDate >= dayjs().format('YYYY-MM-DD')}
						>
							<ChevronRight class="h-4 w-4" />
						</button>
					</div>
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
		<!-- Flights In Progress Section (only show for today) -->
		{#if isToday}
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
					<!-- Desktop: Table -->
					<div class="hidden md:block">
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
									{#each flightsInProgressDetails as { flight, aircraft } (flight.id)}
										<tr>
											<td>
												<div class="flex flex-col gap-1">
													{#if aircraft}
														<AircraftLink {aircraft} size="md" />
													{:else}
														<span class="font-medium">
															{flight.registration ||
																formatAircraftAddress(
																	flight.deviceAddress,
																	flight.deviceAddressType
																)}
														</span>
													{/if}
												</div>
											</td>
											<td>
												{#if flight.aircraftCategory}
													<span
														class="badge {getAircraftCategoryColor(
															flight.aircraftCategory
														)} text-xs"
													>
														{getAircraftCategoryDescription(flight.aircraftCategory)}
													</span>
												{:else}
													<span class="text-surface-500">—</span>
												{/if}
											</td>
											<td>
												<div class="flex flex-col gap-1">
													<div class="flex items-center gap-1 text-sm">
														<Clock class="h-3 w-3" />
														{formatRelativeTime(flight.takeoffTime)}
													</div>
													{#if flight.takeoffTime}
														<div class="text-surface-500-400-token text-xs">
															{formatLocalTime(flight.takeoffTime)}
														</div>
													{/if}
												</div>
											</td>
											<td class="font-semibold">
												{calculateFlightDuration(flight.takeoffTime, dayjs().toISOString())}
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
					</div>

					<!-- Mobile: Cards -->
					<div class="space-y-4 md:hidden">
						{#each flightsInProgressDetails as { flight, aircraft } (flight.id)}
							<div class="card p-4">
								<div class="mb-3 flex items-start justify-between gap-2">
									<div class="flex-1">
										{#if aircraft}
											<AircraftLink {aircraft} size="md" />
										{:else}
											<div class="font-medium">
												{flight.registration ||
													formatAircraftAddress(flight.deviceAddress, flight.deviceAddressType)}
											</div>
										{/if}
									</div>
									{#if flight.aircraftCategory}
										<span class="badge {getAircraftCategoryColor(flight.aircraftCategory)} text-xs">
											{getAircraftCategoryDescription(flight.aircraftCategory)}
										</span>
									{/if}
								</div>

								<dl class="mb-4 space-y-2 text-sm">
									<div class="flex justify-between gap-4">
										<dt class="text-surface-600-300-token">Takeoff</dt>
										<dd class="text-right">
											<div class="flex items-center gap-1">
												<Clock class="h-3 w-3" />
												{formatRelativeTime(flight.takeoffTime)}
											</div>
											{#if flight.takeoffTime}
												<div class="text-surface-500-400-token text-xs">
													{formatLocalTime(flight.takeoffTime)}
												</div>
											{/if}
										</dd>
									</div>
									<div class="flex justify-between gap-4">
										<dt class="text-surface-600-300-token">Duration</dt>
										<dd class="font-semibold">
											{calculateFlightDuration(flight.takeoffTime, dayjs().toISOString())}
										</dd>
									</div>
								</dl>

								<div class="flex flex-col gap-2">
									{#if userBelongsToClub}
										<button
											onclick={() => openPilotModal(flight.id)}
											class="preset-tonal-primary-500 btn flex w-full items-center justify-center gap-1 btn-sm"
										>
											<UserPlus class="h-3 w-3" />
											Add Pilot
										</button>
									{/if}
									<a
										href={`/flights/${flight.id}`}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-surface-500 btn flex w-full items-center justify-center gap-1 btn-sm"
									>
										<ExternalLink class="h-3 w-3" />
										Open Flight
									</a>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>
		{/if}

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
				<!-- Desktop: Table -->
				<div class="hidden md:block">
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
								{#each completedFlightsDetails as { flight, aircraft } (flight.id)}
									<tr>
										<td>
											<div class="flex flex-col gap-1">
												<div class="flex items-center gap-2">
													{#if aircraft}
														<AircraftLink {aircraft} size="md" />
													{:else}
														<span class="font-medium">
															{flight.registration ||
																formatAircraftAddress(
																	flight.deviceAddress,
																	flight.deviceAddressType
																)}
														</span>
													{/if}
													{#if flight.towedByAircraftId}
														<span
															class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
															title="This aircraft was towed"
														>
															<MoveUp class="h-3 w-3" />
															Towed
														</span>
													{/if}
												</div>
											</div>
										</td>
										<td>
											{#if flight.aircraftCategory}
												<span
													class="badge {getAircraftCategoryColor(flight.aircraftCategory)} text-xs"
												>
													{getAircraftCategoryDescription(flight.aircraftCategory)}
												</span>
											{:else}
												<span class="text-surface-500">—</span>
											{/if}
										</td>
										<td>
											<div class="flex flex-col gap-1">
												<div class="flex items-center gap-1 text-sm">
													<Clock class="h-3 w-3" />
													{formatRelativeTime(flight.takeoffTime)}
												</div>
												{#if flight.takeoffTime}
													<div class="text-surface-500-400-token text-xs">
														{formatLocalTime(flight.takeoffTime)}
													</div>
												{/if}
												{#if flight.departureAirport}
													<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
														<MapPin class="h-3 w-3" />
														{flight.departureAirport}
													</div>
												{/if}
											</div>
										</td>
										<td>
											<div class="flex flex-col gap-1">
												<div class="flex items-center gap-1 text-sm">
													<Clock class="h-3 w-3" />
													{formatRelativeTime(flight.landingTime)}
												</div>
												{#if flight.landingTime}
													<div class="text-surface-500-400-token text-xs">
														{formatLocalTime(flight.landingTime)}
													</div>
												{/if}
												{#if flight.arrivalAirport}
													<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
														<MapPin class="h-3 w-3" />
														{flight.arrivalAirport}
													</div>
												{/if}
											</div>
										</td>
										<td class="font-semibold">
											{calculateFlightDuration(flight.takeoffTime, flight.landingTime)}
										</td>
										<td class="font-semibold">
											{formatDistance(flight.totalDistanceMeters)}
										</td>
										<td>
											{#if flight.towedByAircraftId}
												<TowAircraftLink aircraftId={flight.towedByAircraftId} size="sm" />
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
				</div>

				<!-- Mobile: Cards -->
				<div class="space-y-4 md:hidden">
					{#each completedFlightsDetails as { flight, aircraft } (flight.id)}
						<div class="card p-4">
							<div class="mb-3 flex items-start justify-between gap-2">
								<div class="flex-1">
									{#if aircraft}
										<AircraftLink {aircraft} size="md" />
									{:else}
										<div class="font-medium">
											{flight.registration ||
												formatAircraftAddress(flight.deviceAddress, flight.deviceAddressType)}
										</div>
									{/if}
									{#if flight.towedByAircraftId}
										<span
											class="mt-1 badge flex inline-flex items-center gap-1 preset-filled-primary-500 text-xs"
											title="This aircraft was towed"
										>
											<MoveUp class="h-3 w-3" />
											Towed
										</span>
									{/if}
								</div>
								<div>
									{#if flight.aircraftCategory}
										<span class="badge {getAircraftCategoryColor(flight.aircraftCategory)} text-xs">
											{getAircraftCategoryDescription(flight.aircraftCategory)}
										</span>
									{/if}
								</div>
							</div>

							<dl class="mb-4 space-y-2 text-sm">
								<div class="flex justify-between gap-4">
									<dt class="text-surface-600-300-token">Takeoff</dt>
									<dd class="text-right">
										<div class="flex items-center gap-1">
											<Clock class="h-3 w-3" />
											{formatRelativeTime(flight.takeoffTime)}
										</div>
										{#if flight.takeoffTime}
											<div class="text-surface-500-400-token text-xs">
												{formatLocalTime(flight.takeoffTime)}
											</div>
										{/if}
										{#if flight.departureAirport}
											<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
												<MapPin class="h-3 w-3" />
												{flight.departureAirport}
											</div>
										{/if}
									</dd>
								</div>
								<div class="flex justify-between gap-4">
									<dt class="text-surface-600-300-token">Landing</dt>
									<dd class="text-right">
										<div class="flex items-center gap-1">
											<Clock class="h-3 w-3" />
											{formatRelativeTime(flight.landingTime)}
										</div>
										{#if flight.landingTime}
											<div class="text-surface-500-400-token text-xs">
												{formatLocalTime(flight.landingTime)}
											</div>
										{/if}
										{#if flight.arrivalAirport}
											<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
												<MapPin class="h-3 w-3" />
												{flight.arrivalAirport}
											</div>
										{/if}
									</dd>
								</div>
								<div class="flex justify-between gap-4">
									<dt class="text-surface-600-300-token">Duration</dt>
									<dd class="font-semibold">
										{calculateFlightDuration(flight.takeoffTime, flight.landingTime)}
									</dd>
								</div>
								<div class="flex justify-between gap-4">
									<dt class="text-surface-600-300-token">Distance</dt>
									<dd class="font-semibold">{formatDistance(flight.totalDistanceMeters)}</dd>
								</div>
								{#if flight.towedByAircraftId}
									<div class="flex justify-between gap-4">
										<dt class="text-surface-600-300-token">Tow Aircraft</dt>
										<dd>
											<TowAircraftLink aircraftId={flight.towedByAircraftId} size="sm" />
										</dd>
									</div>
								{/if}
							</dl>

							<div class="flex flex-col gap-2">
								{#if userBelongsToClub}
									<button
										onclick={() => openPilotModal(flight.id)}
										class="preset-tonal-primary-500 btn flex w-full items-center justify-center gap-1 btn-sm"
									>
										<UserPlus class="h-3 w-3" />
										Add Pilot
									</button>
								{/if}
								<a
									href={`/flights/${flight.id}`}
									target="_blank"
									rel="noopener noreferrer"
									class="preset-tonal-surface-500 btn flex w-full items-center justify-center gap-1 btn-sm"
								>
									<ExternalLink class="h-3 w-3" />
									Open Flight
								</a>
							</div>
						</div>
					{/each}
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
