<script lang="ts">
	import {
		Plane,
		MapPin,
		Clock,
		ExternalLink,
		MoveUp,
		ChevronLeft,
		ChevronRight
	} from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { getAircraftTypeOgnDescription, getAircraftTypeColor } from '$lib/formatters';
	import FlightStateBadge from '$lib/components/FlightStateBadge.svelte';

	dayjs.extend(relativeTime);

	interface Flight {
		id: string;
		device_address: string;
		device_address_type: string;
		takeoff_time: string | null;
		landing_time: string | null;
		timed_out_at: string | null;
		state: 'active' | 'complete' | 'timed_out';
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
		latest_altitude_msl_feet: number | null;
		latest_altitude_agl_feet: number | null;
		duration_seconds: number | null;
	}

	interface FlightsListResponse {
		flights: Flight[];
		total_count: number;
	}

	let completedFlights: Flight[] = [];
	let activeFlights: Flight[] = [];
	let activeTotalCount = 0;
	let completedTotalCount = 0;
	let activeCurrentPage = 1;
	let completedCurrentPage = 1;
	const ITEMS_PER_PAGE = 50;
	let loading = true;
	let error = '';

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

	function calculateActiveDuration(takeoff: string | null): string {
		if (!takeoff) return '—';
		const takeoffTime = new Date(takeoff).getTime();
		const nowTime = new Date().getTime();
		const durationMs = nowTime - takeoffTime;
		const hours = Math.floor(durationMs / (1000 * 60 * 60));
		const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	}

	function formatAltitude(mslFeet: number | null, aglFeet: number | null): string {
		if (mslFeet === null && aglFeet === null) return '—';
		const parts: string[] = [];
		if (mslFeet !== null) {
			parts.push(`${mslFeet.toLocaleString()} ft MSL`);
		}
		if (aglFeet !== null) {
			parts.push(`${aglFeet.toLocaleString()} ft AGL`);
		}
		return parts.join(' / ');
	}

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

	async function loadFlights() {
		loading = true;
		error = '';

		try {
			const activeOffset = (activeCurrentPage - 1) * ITEMS_PER_PAGE;
			const completedOffset = (completedCurrentPage - 1) * ITEMS_PER_PAGE;

			// Load both active and completed flights in parallel
			const [activeResponse, completedResponse] = await Promise.all([
				serverCall<FlightsListResponse>(
					`/flights?completed=false&limit=${ITEMS_PER_PAGE}&offset=${activeOffset}`
				),
				serverCall<FlightsListResponse>(
					`/flights?completed=true&limit=${ITEMS_PER_PAGE}&offset=${completedOffset}`
				)
			]);

			activeFlights = activeResponse.flights || [];
			activeTotalCount = activeResponse.total_count || 0;
			completedFlights = completedResponse.flights || [];
			completedTotalCount = completedResponse.total_count || 0;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load flights: ${errorMessage}`;
			console.error('Error loading flights:', err);
			activeFlights = [];
			completedFlights = [];
			activeTotalCount = 0;
			completedTotalCount = 0;
		} finally {
			loading = false;
		}
	}

	function handleActivePageChange(newPage: number) {
		activeCurrentPage = newPage;
		loadFlights();
	}

	function handleCompletedPageChange(newPage: number) {
		completedCurrentPage = newPage;
		loadFlights();
	}

	$: activeTotalPages = Math.ceil(activeTotalCount / ITEMS_PER_PAGE);
	$: completedTotalPages = Math.ceil(completedTotalCount / ITEMS_PER_PAGE);

	onMount(() => {
		loadFlights();
	});
</script>

<svelte:head>
	<title>Flights - Aircraft Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Plane class="h-8 w-8" />
			Flights
		</h1>
		<p class="text-surface-500-400-token">Active flights and recent completed flights</p>
	</header>

	<!-- Loading State -->
	{#if loading}
		<div class="space-y-4 card p-12 text-center">
			<div
				class="mx-auto h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
			<p class="text-surface-500-400-token">Loading flights...</p>
		</div>
	{:else if error}
		<!-- Error State -->
		<div class="space-y-4 card p-12 text-center">
			<div class="rounded border border-red-200 bg-red-50 p-4 text-red-600">
				{error}
			</div>
			<button class="btn preset-filled-primary-500" onclick={loadFlights}> Try Again </button>
		</div>
	{:else if activeFlights.length === 0 && completedFlights.length === 0}
		<!-- Empty State -->
		<div class="space-y-4 card p-12 text-center">
			<Plane class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No flights found</h3>
				<p class="text-surface-500-400-token">No flights have been recorded yet.</p>
			</div>
		</div>
	{:else}
		<!-- Active Flights Table (Desktop) -->
		{#if activeFlights.length > 0}
			<section class="hidden card md:block">
				<header class="card-header">
					<h2 class="h2">Active Flights</h2>
					<p class="text-surface-500-400-token">
						{activeTotalCount} flight{activeTotalCount === 1 ? '' : 's'} in progress
					</p>
				</header>

				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Aircraft</th>
								<th>Type</th>
								<th>Recognized at</th>
								<th>Takeoff</th>
								<th>Duration</th>
								<th>Distance</th>
								<th>Altitude</th>
								<th>Tow</th>
								<th></th>
							</tr>
						</thead>
						<tbody>
							{#each activeFlights as flight (flight.id)}
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
												{formatRelativeTime(flight.created_at)}
											</div>
											{#if flight.created_at}
												<div class="text-surface-500-400-token text-xs">
													{formatLocalTime(flight.created_at)}
												</div>
											{/if}
										</div>
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
									<td class="font-semibold">
										{calculateActiveDuration(flight.takeoff_time)}
									</td>
									<td class="font-semibold">
										{formatDistance(flight.total_distance_meters)}
									</td>
									<td>
										<div class="text-sm">
											{formatAltitude(flight.latest_altitude_msl_feet, flight.latest_altitude_agl_feet)}
										</div>
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
										<a
											href={`/flights/${flight.id}`}
											target="_blank"
											rel="noopener noreferrer"
											class="preset-filled-primary btn flex items-center gap-1 btn-sm"
										>
											<ExternalLink class="h-3 w-3" />
											Open
										</a>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<!-- Active Flights Pagination -->
				{#if activeTotalPages > 1}
					<footer class="card-footer flex items-center justify-between">
						<div class="text-surface-500-400-token text-sm">
							Page {activeCurrentPage} of {activeTotalPages}
						</div>
						<div class="flex gap-2">
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={activeCurrentPage === 1}
								onclick={() => handleActivePageChange(activeCurrentPage - 1)}
							>
								<ChevronLeft class="h-4 w-4" />
								Previous
							</button>
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={activeCurrentPage === activeTotalPages}
								onclick={() => handleActivePageChange(activeCurrentPage + 1)}
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						</div>
					</footer>
				{/if}
			</section>
		{/if}

		<!-- Completed Flights Table (Desktop) -->
		{#if completedFlights.length > 0}
			<section class="hidden card md:block">
				<header class="card-header">
					<h2 class="h2">Completed Flights</h2>
					<p class="text-surface-500-400-token">
						{completedTotalCount} flight{completedTotalCount === 1 ? '' : 's'} found
					</p>
				</header>

				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Aircraft</th>
								<th>Type</th>
								<th>Status</th>
								<th>Takeoff</th>
								<th>Landing</th>
								<th>Duration</th>
								<th>Distance</th>
								<th>Tow</th>
								<th></th>
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
										<FlightStateBadge state={flight.state} />
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
										<a
											href={`/flights/${flight.id}`}
											target="_blank"
											rel="noopener noreferrer"
											class="preset-filled-primary btn flex items-center gap-1 btn-sm"
										>
											<ExternalLink class="h-3 w-3" />
											Open
										</a>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<!-- Completed Flights Pagination -->
				{#if completedTotalPages > 1}
					<footer class="card-footer flex items-center justify-between">
						<div class="text-surface-500-400-token text-sm">
							Page {completedCurrentPage} of {completedTotalPages}
						</div>
						<div class="flex gap-2">
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={completedCurrentPage === 1}
								onclick={() => handleCompletedPageChange(completedCurrentPage - 1)}
							>
								<ChevronLeft class="h-4 w-4" />
								Previous
							</button>
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={completedCurrentPage === completedTotalPages}
								onclick={() => handleCompletedPageChange(completedCurrentPage + 1)}
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						</div>
					</footer>
				{/if}
			</section>
		{/if}

		<!-- Active Flights Cards (Mobile) -->
		{#if activeFlights.length > 0}
			<div class="block space-y-4 md:hidden">
				<div class="card-header">
					<h2 class="h2">Active Flights</h2>
					<p class="text-surface-500-400-token">
						{activeTotalCount} flight{activeTotalCount === 1 ? '' : 's'} in progress
					</p>
				</div>

				{#each activeFlights as flight (flight.id)}
					<div class="relative card p-4 transition-all duration-200 hover:shadow-lg">
						<!-- Aircraft info -->
						<div
							class="border-surface-200-700-token mb-3 flex items-start justify-between border-b pb-3"
						>
							<div class="flex flex-wrap items-center gap-2">
								{#if flight.aircraft_model && flight.registration}
									{#if flight.device_id}
										<a
											href={`/devices/${flight.device_id}`}
											class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
										>
											{flight.aircraft_model} ({flight.registration})
										</a>
									{:else}
										<span class="font-semibold"
											>{flight.aircraft_model} ({flight.registration})</span
										>
									{/if}
								{:else if flight.registration}
									{#if flight.device_id}
										<a
											href={`/devices/${flight.device_id}`}
											class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
										>
											{flight.registration}
										</a>
									{:else}
										<span class="font-semibold">{flight.registration}</span>
									{/if}
								{:else if flight.device_id}
									<a
										href={`/devices/${flight.device_id}`}
										class="text-surface-500-400-token relative z-10 anchor font-mono text-sm hover:text-primary-500"
									>
										{formatDeviceAddress(flight.device_address, flight.device_address_type)}
									</a>
								{:else}
									<span class="text-surface-500-400-token font-mono text-sm">
										{formatDeviceAddress(flight.device_address, flight.device_address_type)}
									</span>
								{/if}
								{#if flight.aircraft_type_ogn}
									<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
										{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
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
							<a
								href={`/flights/${flight.id}`}
								class="relative z-10 flex-shrink-0"
								title="View flight details"
							>
								<ExternalLink class="h-4 w-4 text-surface-400 hover:text-primary-500" />
							</a>
						</div>

						<!-- Flight details -->
						<div class="text-surface-600-300-token space-y-2 text-sm">
							<div>
								<span class="text-surface-500-400-token text-xs">Recognized:</span>
								{formatLocalTime(flight.created_at)}
								<span class="text-surface-500-400-token text-xs">
									({formatRelativeTime(flight.created_at)})
								</span>
							</div>
							<div>
								<span class="text-surface-500-400-token text-xs">Takeoff:</span>
								{#if flight.departure_airport}
									<span class="font-medium"
										>{flight.departure_airport}{#if flight.takeoff_runway_ident}/{flight.takeoff_runway_ident}{/if}</span
									>
								{/if}
								{formatLocalTime(flight.takeoff_time)}
								<span class="text-surface-500-400-token text-xs">
									({formatRelativeTime(flight.takeoff_time)})
								</span>
							</div>
							<div class="flex gap-4">
								<div>
									<span class="text-surface-500-400-token text-xs">Duration:</span>
									<span class="font-semibold">{calculateActiveDuration(flight.takeoff_time)}</span>
								</div>
								<div>
									<span class="text-surface-500-400-token text-xs">Distance:</span>
									<span class="font-semibold">{formatDistance(flight.total_distance_meters)}</span>
								</div>
							</div>
							{#if flight.latest_altitude_msl_feet !== null || flight.latest_altitude_agl_feet !== null}
								<div>
									<span class="text-surface-500-400-token text-xs">Altitude:</span>
									{formatAltitude(flight.latest_altitude_msl_feet, flight.latest_altitude_agl_feet)}
								</div>
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/if}

		<!-- Completed Flights Cards (Mobile) -->
		{#if completedFlights.length > 0}
			<div class="block space-y-4 md:hidden">
				<div class="card-header">
					<h2 class="h2">Completed Flights</h2>
					<p class="text-surface-500-400-token">
						{completedTotalCount} flight{completedTotalCount === 1 ? '' : 's'} found
					</p>
				</div>

				{#each completedFlights as flight (flight.id)}
					<div class="relative card p-4 transition-all duration-200 hover:shadow-lg">
						<!-- Aircraft info -->
						<div
							class="border-surface-200-700-token mb-3 flex items-start justify-between border-b pb-3"
						>
							<div class="flex flex-wrap items-center gap-2">
								{#if flight.aircraft_model && flight.registration}
									{#if flight.device_id}
										<a
											href={`/devices/${flight.device_id}`}
											class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
										>
											{flight.aircraft_model} ({flight.registration})
										</a>
									{:else}
										<span class="font-semibold"
											>{flight.aircraft_model} ({flight.registration})</span
										>
									{/if}
								{:else if flight.registration}
									{#if flight.device_id}
										<a
											href={`/devices/${flight.device_id}`}
											class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
										>
											{flight.registration}
										</a>
									{:else}
										<span class="font-semibold">{flight.registration}</span>
									{/if}
								{:else if flight.device_id}
									<a
										href={`/devices/${flight.device_id}`}
										class="text-surface-500-400-token relative z-10 anchor font-mono text-sm hover:text-primary-500"
									>
										{formatDeviceAddress(flight.device_address, flight.device_address_type)}
									</a>
								{:else}
									<span class="text-surface-500-400-token font-mono text-sm">
										{formatDeviceAddress(flight.device_address, flight.device_address_type)}
									</span>
								{/if}
								<FlightStateBadge state={flight.state} />
								{#if flight.aircraft_type_ogn}
									<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
										{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
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
							<a
								href={`/flights/${flight.id}`}
								class="relative z-10 flex-shrink-0"
								title="View flight details"
							>
								<ExternalLink class="h-4 w-4 text-surface-400 hover:text-primary-500" />
							</a>
						</div>

						<!-- Flight details in compact single row -->
						<div class="text-surface-600-300-token text-sm">
							{#if flight.departure_airport}
								<span class="font-medium"
									>{flight.departure_airport}{#if flight.takeoff_runway_ident}/{flight.takeoff_runway_ident}{/if}</span
								>
							{/if}
							{formatLocalTime(flight.takeoff_time)}
							<span class="text-surface-500-400-token text-xs">
								({formatRelativeTime(flight.takeoff_time)})
							</span>
							<span class="mx-1">-</span>
							{#if flight.arrival_airport}
								<span class="font-medium"
									>{flight.arrival_airport}{#if flight.landing_runway_ident}/{flight.landing_runway_ident}{/if}</span
								>
							{/if}
							{formatLocalTime(flight.landing_time)}
							<span class="text-surface-500-400-token text-xs">
								({formatRelativeTime(flight.landing_time)})
							</span>
							<span class="mx-2 font-semibold">
								{calculateFlightDuration(flight.takeoff_time, flight.landing_time)}
							</span>
							<span class="font-semibold">
								{formatDistance(flight.total_distance_meters)}
							</span>
						</div>
					</div>
				{/each}
			</div>
		{/if}
	{/if}
</div>
