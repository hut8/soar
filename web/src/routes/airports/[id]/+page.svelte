<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Plane,
		MapPin,
		Navigation,
		Info,
		ExternalLink,
		Compass,
		Clock
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';

	interface RunwayEnd {
		ident: string | null;
		latitude_deg: number | null;
		longitude_deg: number | null;
		elevation_ft: number | null;
		heading_degt: number | null;
		displaced_threshold_ft: number | null;
	}

	interface Runway {
		id: number;
		length_ft: number | null;
		width_ft: number | null;
		surface: string | null;
		lighted: boolean;
		closed: boolean;
		low: RunwayEnd;
		high: RunwayEnd;
	}

	interface Airport {
		id: number;
		ident: string;
		airport_type: string;
		name: string;
		latitude_deg: string | null;
		longitude_deg: string | null;
		elevation_ft: number | null;
		continent: string | null;
		iso_country: string | null;
		iso_region: string | null;
		municipality: string | null;
		scheduled_service: boolean;
		icao_code: string | null;
		iata_code: string | null;
		gps_code: string | null;
		local_code: string | null;
		home_link: string | null;
		wikipedia_link: string | null;
		keywords: string | null;
		runways: Runway[];
	}

	interface FlightView {
		id: string;
		device_address: string;
		takeoff_time: string | null;
		landing_time: string | null;
		departure_airport_ident: string | null;
		arrival_airport_ident: string | null;
		created_at: string;
		updated_at: string;
	}

	interface Device {
		id: string;
		registration: string;
		competition_number: string;
	}

	interface FlightResponse {
		flight: FlightView;
		device: Device | null;
	}

	let airport: Airport | null = null;
	let flights: FlightResponse[] = [];
	let loading = true;
	let flightsLoading = false;
	let error = '';
	let flightsError = '';
	let airportId = '';

	$: airportId = $page.params.id || '';

	onMount(async () => {
		if (airportId) {
			await Promise.all([loadAirport(), loadFlights()]);
		}
	});

	async function loadAirport() {
		loading = true;
		error = '';

		try {
			airport = await serverCall<Airport>(`/airports/${airportId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load airport: ${errorMessage}`;
			console.error('Error loading airport:', err);
		} finally {
			loading = false;
		}
	}

	async function loadFlights() {
		flightsLoading = true;
		flightsError = '';

		try {
			flights = await serverCall<FlightResponse[]>(`/airports/${airportId}/flights`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			flightsError = `Failed to load flights: ${errorMessage}`;
			console.error('Error loading flights:', err);
		} finally {
			flightsLoading = false;
		}
	}

	function formatCoordinates(lat: string | null, lng: string | null): string {
		if (!lat || !lng) return 'Not available';
		return `${parseFloat(lat).toFixed(6)}, ${parseFloat(lng).toFixed(6)}`;
	}

	function generateGoogleMapsUrl(airport: Airport): string {
		if (airport.latitude_deg && airport.longitude_deg) {
			return `https://www.google.com/maps/search/?api=1&query=${airport.latitude_deg},${airport.longitude_deg}`;
		}
		return '';
	}

	function goBack() {
		goto(resolve('/airports'));
	}

	function getAirportCode(airport: Airport): string {
		return (
			airport.icao_code ||
			airport.iata_code ||
			airport.gps_code ||
			airport.local_code ||
			airport.ident
		);
	}

	function formatAirportType(type: string): string {
		return type
			.split('_')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
			.join(' ');
	}

	function formatDateTime(dateStr: string | null): string {
		if (!dateStr) return '—';
		const date = new Date(dateStr);
		return date.toLocaleString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function getFlightStatus(flight: FlightView): string {
		if (!flight.landing_time) return 'In Progress';
		return 'Completed';
	}

	function getFlightType(flight: FlightView, currentAirportId: string): string {
		const isDeparture = flight.departure_airport_ident === currentAirportId;
		const isArrival = flight.arrival_airport_ident === currentAirportId;

		if (isDeparture && isArrival) return 'Local';
		if (isDeparture) return 'Departure';
		if (isArrival) return 'Arrival';
		return 'Unknown';
	}
</script>

<svelte:head>
	<title>{airport?.name || 'Airport Details'} - Airports</title>
</svelte:head>

<div class="max-w-8xl container mx-auto space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="preset-soft btn btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Search
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Loading airport details...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert preset-filled-error">
			<div class="alert-message">
				<h3 class="h3">Error Loading Airport</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn preset-filled" onclick={loadAirport}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Airport Details -->
	{#if !loading && !error && airport}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Plane class="h-8 w-10 text-primary-500" />
							<h1 class="h1">{airport.name}</h1>
						</div>
						<div class="flex flex-wrap items-center gap-2">
							<span class="preset-soft-primary badge font-mono text-lg">
								{getAirportCode(airport)}
							</span>
							<span class="preset-soft badge">
								{formatAirportType(airport.airport_type)}
							</span>
							{#if airport.scheduled_service}
								<span class="preset-filled-success badge"> Scheduled Service </span>
							{/if}
						</div>
					</div>
				</div>
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				<!-- Location Information -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<MapPin class="h-6 w-6" />
						Location
					</h2>

					<div class="space-y-3">
						{#if airport.municipality || airport.iso_region}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Municipality</p>
									<p>
										{airport.municipality || '—'}
										{#if airport.iso_region}
											<span class="text-surface-500">, {airport.iso_region}</span>
										{/if}
									</p>
									{#if airport.iso_country}
										<p class="text-sm text-surface-500">{airport.iso_country}</p>
									{/if}
								</div>
							</div>
						{/if}

						{#if airport.latitude_deg && airport.longitude_deg}
							<div class="flex items-start gap-3">
								<Navigation class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Coordinates</p>
									<p class="font-mono text-sm">
										{formatCoordinates(airport.latitude_deg, airport.longitude_deg)}
									</p>
								</div>
							</div>
						{/if}

						{#if airport.elevation_ft !== null}
							<div class="flex items-start gap-3">
								<Compass class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Elevation</p>
									<p>{airport.elevation_ft} ft MSL</p>
								</div>
							</div>
						{/if}

						<!-- External Links -->
						{#if airport.latitude_deg && airport.longitude_deg}
							<div class="border-surface-200-700-token border-t pt-3">
								<div class="flex flex-wrap gap-2">
									<a
										href={generateGoogleMapsUrl(airport)}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-soft-primary btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Open in Google Maps
									</a>
									<a
										href={`https://www.google.com/maps/dir/?api=1&destination=${airport.latitude_deg},${airport.longitude_deg}`}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-soft-secondary btn btn-sm"
									>
										<Navigation class="mr-2 h-4 w-4" />
										Get Directions
									</a>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Airport Codes & Info -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Info class="h-6 w-6" />
						Airport Codes
					</h2>

					<div class="space-y-2">
						{#if airport.icao_code}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">ICAO Code:</span>
								<span class="font-mono font-semibold">{airport.icao_code}</span>
							</div>
						{/if}
						{#if airport.iata_code}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">IATA Code:</span>
								<span class="font-mono font-semibold">{airport.iata_code}</span>
							</div>
						{/if}
						{#if airport.gps_code}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">GPS Code:</span>
								<span class="font-mono font-semibold">{airport.gps_code}</span>
							</div>
						{/if}
						{#if airport.local_code}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">Local Code:</span>
								<span class="font-mono font-semibold">{airport.local_code}</span>
							</div>
						{/if}
					</div>

					{#if airport.home_link || airport.wikipedia_link}
						<div class="border-surface-200-700-token border-t pt-3">
							<div class="flex flex-wrap gap-2">
								{#if airport.home_link}
									<a
										href={airport.home_link}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-soft-primary btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Website
									</a>
								{/if}
								{#if airport.wikipedia_link}
									<a
										href={airport.wikipedia_link}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-soft-secondary btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Wikipedia
									</a>
								{/if}
							</div>
						</div>
					{/if}
				</div>
			</div>

			<!-- Runways Section -->
			{#if airport.runways && airport.runways.length > 0}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Plane class="h-6 w-6" />
						Runways
					</h2>

					<div class="space-y-4">
						{#each airport.runways as runway (runway.id)}
							<div class="bg-surface-50-900-token rounded-lg p-4">
								<div class="mb-3 flex flex-wrap items-center justify-between gap-2">
									<h3 class="h3 font-mono font-semibold">
										{runway.low.ident || 'N/A'} / {runway.high.ident || 'N/A'}
									</h3>
									<div class="flex flex-wrap gap-2">
										{#if runway.lighted}
											<span class="preset-filled-success badge">
												<svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
													<path
														d="M11 3a1 1 0 10-2 0v1a1 1 0 102 0V3zM15.657 5.757a1 1 0 00-1.414-1.414l-.707.707a1 1 0 001.414 1.414l.707-.707zM18 10a1 1 0 01-1 1h-1a1 1 0 110-2h1a1 1 0 011 1zM5.05 6.464A1 1 0 106.464 5.05l-.707-.707a1 1 0 00-1.414 1.414l.707.707zM5 10a1 1 0 01-1 1H3a1 1 0 110-2h1a1 1 0 011 1zM8 16v-1h4v1a2 2 0 11-4 0zM12 14c.015-.34.208-.646.477-.859a4 4 0 10-4.954 0c.27.213.462.519.476.859h4.002z"
													/>
												</svg>
												Lighted
											</span>
										{/if}
										{#if runway.closed}
											<span class="preset-filled-error badge">
												<svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
													<path
														fill-rule="evenodd"
														d="M13.477 14.89A6 6 0 015.11 6.524l8.367 8.368zm1.414-1.414L6.524 5.11a6 6 0 018.367 8.367zM18 10a8 8 0 11-16 0 8 8 0 0116 0z"
														clip-rule="evenodd"
													/>
												</svg>
												Closed
											</span>
										{/if}
									</div>
								</div>

								<!-- Runway Details Table -->
								<div class="table-container mb-4">
									<table class="table-compact table-hover table">
										<tbody>
											{#if runway.length_ft}
												<tr>
													<td class="w-1/3 font-medium">Length</td>
													<td>{runway.length_ft.toLocaleString()} ft</td>
												</tr>
											{/if}
											{#if runway.width_ft}
												<tr>
													<td class="w-1/3 font-medium">Width</td>
													<td>{runway.width_ft} ft</td>
												</tr>
											{/if}
											{#if runway.surface}
												<tr>
													<td class="w-1/3 font-medium">Surface</td>
													<td>{runway.surface}</td>
												</tr>
											{/if}
										</tbody>
									</table>
								</div>

								<!-- Runway Ends Details -->
								<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
									<!-- Low End -->
									<div>
										<h4 class="mb-2 font-semibold text-primary-500">
											{runway.low.ident || 'Low End'}
										</h4>
										<div class="table-container">
											<table class="table-compact table-hover table">
												<tbody>
													{#if runway.low.heading_degt !== null}
														<tr>
															<td class="w-2/3 text-sm">True Heading</td>
															<td class="text-sm font-medium">{runway.low.heading_degt}°</td>
														</tr>
													{/if}
													{#if runway.low.elevation_ft !== null}
														<tr>
															<td class="w-2/3 text-sm">Elevation</td>
															<td class="text-sm font-medium">{runway.low.elevation_ft} ft</td>
														</tr>
													{/if}
													{#if runway.low.displaced_threshold_ft !== null && runway.low.displaced_threshold_ft > 0}
														<tr>
															<td class="w-2/3 text-sm">Displaced Threshold</td>
															<td class="text-sm font-medium"
																>{runway.low.displaced_threshold_ft} ft</td
															>
														</tr>
													{/if}
												</tbody>
											</table>
										</div>
									</div>

									<!-- High End -->
									<div>
										<h4 class="mb-2 font-semibold text-primary-500">
											{runway.high.ident || 'High End'}
										</h4>
										<div class="table-container">
											<table class="table-compact table-hover table">
												<tbody>
													{#if runway.high.heading_degt !== null}
														<tr>
															<td class="w-2/3 text-sm">True Heading</td>
															<td class="text-sm font-medium">{runway.high.heading_degt}°</td>
														</tr>
													{/if}
													{#if runway.high.elevation_ft !== null}
														<tr>
															<td class="w-2/3 text-sm">Elevation</td>
															<td class="text-sm font-medium">{runway.high.elevation_ft} ft</td>
														</tr>
													{/if}
													{#if runway.high.displaced_threshold_ft !== null && runway.high.displaced_threshold_ft > 0}
														<tr>
															<td class="w-2/3 text-sm">Displaced Threshold</td>
															<td class="text-sm font-medium"
																>{runway.high.displaced_threshold_ft} ft</td
															>
														</tr>
													{/if}
												</tbody>
											</table>
										</div>
									</div>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Recent Flights Section (Last 24 Hours) -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Clock class="h-6 w-6" />
					Recent Flights (Last 24 Hours)
				</h2>

				<!-- Flights Loading State -->
				{#if flightsLoading}
					<div class="flex items-center justify-center space-x-4 p-8">
						<ProgressRing size="w-6 h-6" />
						<span>Loading flights...</span>
					</div>
				{/if}

				<!-- Flights Error State -->
				{#if flightsError}
					<div class="alert preset-filled-error mb-4">
						<div class="alert-message">
							<p>{flightsError}</p>
						</div>
					</div>
				{/if}

				<!-- Flights List -->
				{#if !flightsLoading && !flightsError}
					{#if flights.length === 0}
						<p class="text-surface-500">No flights in the last 24 hours.</p>
					{:else}
						<div class="table-container">
							<table class="table-hover table">
								<thead>
									<tr>
										<th>Aircraft</th>
										<th>Type</th>
										<th>Departure</th>
										<th>Arrival</th>
										<th>Takeoff</th>
										<th>Landing</th>
										<th>Status</th>
										<th>Actions</th>
									</tr>
								</thead>
								<tbody>
									{#each flights as flightData (flightData.flight.id)}
										<tr>
											<td>
												{#if flightData.device}
													<div class="flex flex-col">
														<span class="font-mono font-semibold"
															>{flightData.device.registration}</span
														>
														{#if flightData.device.competition_number}
															<span class="text-sm text-surface-500"
																>{flightData.device.competition_number}</span
															>
														{/if}
													</div>
												{:else}
													<span class="font-mono">{flightData.flight.device_address}</span>
												{/if}
											</td>
											<td>
												<span
													class="preset-soft badge"
													class:preset-soft-primary={getFlightType(
														flightData.flight,
														airport?.ident || ''
													) === 'Departure'}
													class:preset-soft-success={getFlightType(
														flightData.flight,
														airport?.ident || ''
													) === 'Arrival'}
													class:preset-soft-secondary={getFlightType(
														flightData.flight,
														airport?.ident || ''
													) === 'Local'}
												>
													{getFlightType(flightData.flight, airport?.ident || '')}
												</span>
											</td>
											<td>
												<span class="font-mono text-sm">
													{flightData.flight.departure_airport_ident || '—'}
												</span>
											</td>
											<td>
												<span class="font-mono text-sm">
													{flightData.flight.arrival_airport_ident || '—'}
												</span>
											</td>
											<td>{formatDateTime(flightData.flight.takeoff_time)}</td>
											<td>{formatDateTime(flightData.flight.landing_time)}</td>
											<td>
												{#if getFlightStatus(flightData.flight) === 'In Progress'}
													<span class="preset-filled-warning badge">In Progress</span>
												{:else}
													<span class="preset-filled-success badge">Completed</span>
												{/if}
											</td>
											<td>
												<a
													href={resolve(`/flights/${flightData.flight.id}`)}
													class="preset-ghost-primary btn btn-sm"
												>
													View
												</a>
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>
					{/if}
				{/if}
			</div>

			<!-- Map Section -->
			{#if airport.latitude_deg && airport.longitude_deg}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Location Map
					</h2>
					<div class="border-surface-300-600-token overflow-hidden rounded-lg border">
						<!-- Embedded Google Map -->
						<iframe
							src={`https://maps.google.com/maps?q=${airport.latitude_deg},${airport.longitude_deg}&output=embed`}
							width="100%"
							height="500"
							style="border:0;"
							allowfullscreen
							loading="lazy"
							referrerpolicy="no-referrer-when-downgrade"
							title="Location map for {airport.name}"
						></iframe>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						<a
							href={generateGoogleMapsUrl(airport)}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-ghost-primary btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						<a
							href={`https://www.google.com/maps/dir/?api=1&destination=${airport.latitude_deg},${airport.longitude_deg}`}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-ghost-secondary btn btn-sm"
						>
							<Navigation class="mr-2 h-4 w-4" />
							Get Directions
						</a>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
