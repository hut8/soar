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
		Clock,
		Users
	} from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';
	import type { Airport, Flight, Club, DataResponse, DataListResponse } from '$lib/types';

	const logger = getLogger(['soar', 'AirportDetailsPage']);

	let airport: Airport | null = null;
	let flights: Flight[] = [];
	let clubs: Club[] = [];
	let loading = true;
	let flightsLoading = false;
	let clubsLoading = false;
	let error = '';
	let flightsError = '';
	let clubsError = '';
	let airportId = '';

	$: airportId = $page.params.id || '';

	// Generate JSON-LD structured data for SEO (reactive to airport changes)
	$: jsonLdScript = (() => {
		const data = {
			'@context': 'https://schema.org',
			'@type': 'Airport',
			name: airport?.name || 'Airport',
			description: airport
				? `${airport.name}${airport.icaoCode ? ` (${airport.icaoCode})` : ''} - ${airport.municipality || 'Airport'}, ${airport.isoCountry || ''}`
				: 'View airport details including location, runway information, and flight activity.',
			url: `https://glider.flights/airports/${airportId}`,
			iataCode: airport?.iataCode || undefined,
			icaoCode: airport?.icaoCode || undefined,
			...(airport?.latitudeDeg &&
				airport?.longitudeDeg && {
					geo: {
						'@type': 'GeoCoordinates',
						latitude: airport.latitudeDeg,
						longitude: airport.longitudeDeg
					}
				})
		};
		return '<script type="application/ld+json">' + JSON.stringify(data) + '</' + 'script>';
	})();

	onMount(async () => {
		if (airportId) {
			await Promise.all([loadAirport(), loadFlights(), loadClubs()]);
		}
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

	async function loadAirport() {
		loading = true;
		error = '';

		try {
			const response = await serverCall<DataResponse<Airport>>(`/airports/${airportId}`);
			airport = response.data;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load airport: ${errorMessage}`;
			logger.error('Error loading airport: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	async function loadFlights() {
		flightsLoading = true;
		flightsError = '';

		try {
			const response = await serverCall<DataListResponse<Flight>>(`/airports/${airportId}/flights`);
			flights = response.data;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			flightsError = `Failed to load flights: ${errorMessage}`;
			logger.error('Error loading flights: {error}', { error: err });
		} finally {
			flightsLoading = false;
		}
	}

	async function loadClubs() {
		clubsLoading = true;
		clubsError = '';

		try {
			const response = await serverCall<DataListResponse<Club>>(`/airports/${airportId}/clubs`);
			clubs = response.data;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			clubsError = `Failed to load clubs: ${errorMessage}`;
			logger.error('Error loading clubs: {error}', { error: err });
		} finally {
			clubsLoading = false;
		}
	}

	function formatCoordinates(lat: string | null, lng: string | null): string {
		if (!lat || !lng) return 'Not available';
		return `${parseFloat(lat).toFixed(6)}, ${parseFloat(lng).toFixed(6)}`;
	}

	function generateGoogleMapsUrl(airport: Airport): string {
		if (airport.latitudeDeg && airport.longitudeDeg) {
			return `https://www.google.com/maps/search/?api=1&query=${airport.latitudeDeg},${airport.longitudeDeg}`;
		}
		return '';
	}

	function goBack() {
		goto(resolve('/airports'));
	}

	function getAirportCode(airport: Airport): string {
		return (
			airport.icaoCode || airport.iataCode || airport.gpsCode || airport.localCode || airport.ident
		);
	}

	function formatAirportType(type: string): string {
		return type
			.split('_')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
			.join(' ');
	}

	function formatDateTime(dateStr: string | null | undefined): string {
		if (!dateStr) return '—';
		const date = new Date(dateStr);
		return date.toLocaleString('en-US', {
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function getFlightStatus(flight: Flight): string {
		if (!flight.landingTime) return 'In Progress';
		return 'Completed';
	}

	function getFlightType(flight: Flight, currentAirportId: string): string {
		const isDeparture = flight.departureAirport === currentAirportId;
		const isArrival = flight.arrivalAirport === currentAirportId;

		if (isDeparture && isArrival) return 'Local';
		if (isDeparture) return 'Departure';
		if (isArrival) return 'Arrival';
		return 'Unknown';
	}
</script>

<svelte:head>
	<title
		>{airport?.name || 'Airport Details'}{airport?.icaoCode ? ` (${airport.icaoCode})` : ''} - Airports
		- SOAR</title
	>
	<meta
		name="description"
		content={airport
			? `${airport.name}${airport.icaoCode ? ` (${airport.icaoCode})` : ''} in ${airport.municipality || 'Unknown'}, ${airport.isoCountry || ''}. View location, flight activity, and soaring clubs.`
			: 'View airport details including location, coordinates, flight activity, and nearby soaring clubs on SOAR.'}
	/>
	<link rel="canonical" href="https://glider.flights/airports/{airportId}" />

	<!-- JSON-LD structured data for SEO -->
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html jsonLdScript}
</svelte:head>

<div class="max-w-8xl container mx-auto space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn preset-tonal btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Search
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<Progress class="h-8 w-8" />
				<span class="text-lg">Loading airport details...</span>
			</div>

			<!-- SEO fallback content - visible during loading for crawlers -->
			<div class="text-surface-600-300-token mt-6 space-y-4">
				<h1 class="h2">Airport Details</h1>
				<p>
					This page displays detailed information about an airport, including location coordinates,
					runway information, recent flight activity, and nearby soaring clubs.
				</p>
				<p>
					SOAR tracks flights at airports worldwide using Open Glider Network and ADS-B data. View
					<a href="/airports" class="anchor">all airports</a>
					or <a href="/flights" class="anchor">recent flights</a>.
				</p>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert preset-filled-error-500">
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
							<span class="preset-tonal-primary-500 badge font-mono text-lg">
								{getAirportCode(airport)}
							</span>
							<span class="badge preset-tonal">
								{formatAirportType(airport.airportType)}
							</span>
							{#if airport.scheduledService}
								<span class="badge preset-filled-success-500"> Scheduled Service </span>
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
						{#if airport.municipality || airport.isoRegion}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Municipality</p>
									<p>
										{airport.municipality || '—'}
										{#if airport.isoRegion}
											<span class="text-surface-500">, {airport.isoRegion}</span>
										{/if}
									</p>
									{#if airport.isoCountry}
										<p class="text-sm text-surface-500">{airport.isoCountry}</p>
									{/if}
								</div>
							</div>
						{/if}

						{#if airport.latitudeDeg && airport.longitudeDeg}
							<div class="flex items-start gap-3">
								<Navigation class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Coordinates</p>
									<p class="font-mono text-sm">
										{formatCoordinates(airport.latitudeDeg, airport.longitudeDeg)}
									</p>
								</div>
							</div>
						{/if}

						{#if airport.elevationFt !== null}
							<div class="flex items-start gap-3">
								<Compass class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Elevation</p>
									<p>{airport.elevationFt} ft MSL</p>
								</div>
							</div>
						{/if}

						<!-- External Links -->
						{#if airport.latitudeDeg && airport.longitudeDeg}
							<div class="border-surface-200-700-token border-t pt-3">
								<div class="flex flex-wrap gap-2">
									<a
										href={generateGoogleMapsUrl(airport)}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-primary-500 btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Open in Google Maps
									</a>
									<a
										href={`https://www.google.com/maps/dir/?api=1&destination=${airport.latitudeDeg},${airport.longitudeDeg}`}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-secondary-500 btn btn-sm"
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
						{#if airport.icaoCode}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">ICAO Code:</span>
								<span class="font-mono font-semibold">{airport.icaoCode}</span>
							</div>
						{/if}
						{#if airport.iataCode}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">IATA Code:</span>
								<span class="font-mono font-semibold">{airport.iataCode}</span>
							</div>
						{/if}
						{#if airport.gpsCode}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">GPS Code:</span>
								<span class="font-mono font-semibold">{airport.gpsCode}</span>
							</div>
						{/if}
						{#if airport.localCode}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">Local Code:</span>
								<span class="font-mono font-semibold">{airport.localCode}</span>
							</div>
						{/if}
					</div>

					{#if airport.homeLink || airport.wikipediaLink}
						<div class="border-surface-200-700-token border-t pt-3">
							<div class="flex flex-wrap gap-2">
								{#if airport.homeLink}
									<a
										href={airport.homeLink}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-primary-500 btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Website
									</a>
								{/if}
								{#if airport.wikipediaLink}
									<a
										href={airport.wikipediaLink}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-secondary-500 btn btn-sm"
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
											<span class="badge preset-filled-success-500">
												<svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20">
													<path
														d="M11 3a1 1 0 10-2 0v1a1 1 0 102 0V3zM15.657 5.757a1 1 0 00-1.414-1.414l-.707.707a1 1 0 001.414 1.414l.707-.707zM18 10a1 1 0 01-1 1h-1a1 1 0 110-2h1a1 1 0 011 1zM5.05 6.464A1 1 0 106.464 5.05l-.707-.707a1 1 0 00-1.414 1.414l.707.707zM5 10a1 1 0 01-1 1H3a1 1 0 110-2h1a1 1 0 011 1zM8 16v-1h4v1a2 2 0 11-4 0zM12 14c.015-.34.208-.646.477-.859a4 4 0 10-4.954 0c.27.213.462.519.476.859h4.002z"
													/>
												</svg>
												Lighted
											</span>
										{/if}
										{#if runway.closed}
											<span class="badge preset-filled-error-500">
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

								<!-- Runway Details -->
								<!-- Desktop: Table -->
								<div class="mb-4 hidden md:block">
									<div class="table-container">
										<table class="table-compact table-hover table">
											<tbody>
												{#if runway.lengthFt}
													<tr>
														<td class="w-1/3 font-medium">Length</td>
														<td>{runway.lengthFt.toLocaleString()} ft</td>
													</tr>
												{/if}
												{#if runway.widthFt}
													<tr>
														<td class="w-1/3 font-medium">Width</td>
														<td>{runway.widthFt} ft</td>
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
								</div>

								<!-- Mobile: Definition List -->
								<dl class="mb-4 space-y-2 md:hidden">
									{#if runway.lengthFt}
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token font-medium">Length</dt>
											<dd class="font-semibold">{runway.lengthFt.toLocaleString()} ft</dd>
										</div>
									{/if}
									{#if runway.widthFt}
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token font-medium">Width</dt>
											<dd class="font-semibold">{runway.widthFt} ft</dd>
										</div>
									{/if}
									{#if runway.surface}
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token font-medium">Surface</dt>
											<dd class="font-semibold">{runway.surface}</dd>
										</div>
									{/if}
								</dl>

								<!-- Runway Ends Details -->
								<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
									<!-- Low End -->
									<div>
										<h4 class="mb-2 font-semibold text-primary-500">
											{runway.low.ident || 'Low End'}
										</h4>

										<!-- Desktop: Table -->
										<div class="hidden md:block">
											<div class="table-container">
												<table class="table-compact table-hover table">
													<tbody>
														{#if runway.low.headingDegt !== null}
															<tr>
																<td class="w-2/3 text-sm">True Heading</td>
																<td class="text-sm font-medium">{runway.low.headingDegt}°</td>
															</tr>
														{/if}
														{#if runway.low.elevationFt !== null}
															<tr>
																<td class="w-2/3 text-sm">Elevation</td>
																<td class="text-sm font-medium">{runway.low.elevationFt} ft</td>
															</tr>
														{/if}
														{#if runway.low.displacedThresholdFt !== null && runway.low.displacedThresholdFt > 0}
															<tr>
																<td class="w-2/3 text-sm">Displaced Threshold</td>
																<td class="text-sm font-medium"
																	>{runway.low.displacedThresholdFt} ft</td
																>
															</tr>
														{/if}
													</tbody>
												</table>
											</div>
										</div>

										<!-- Mobile: Definition List -->
										<dl class="space-y-2 text-sm md:hidden">
											{#if runway.low.headingDegt !== null}
												<div class="flex justify-between gap-4">
													<dt class="text-surface-600-300-token">True Heading</dt>
													<dd class="font-medium">{runway.low.headingDegt}°</dd>
												</div>
											{/if}
											{#if runway.low.elevationFt !== null}
												<div class="flex justify-between gap-4">
													<dt class="text-surface-600-300-token">Elevation</dt>
													<dd class="font-medium">{runway.low.elevationFt} ft</dd>
												</div>
											{/if}
											{#if runway.low.displacedThresholdFt !== null && runway.low.displacedThresholdFt > 0}
												<div class="flex justify-between gap-4">
													<dt class="text-surface-600-300-token">Displaced Threshold</dt>
													<dd class="font-medium">{runway.low.displacedThresholdFt} ft</dd>
												</div>
											{/if}
										</dl>
									</div>

									<!-- High End -->
									<div>
										<h4 class="mb-2 font-semibold text-primary-500">
											{runway.high.ident || 'High End'}
										</h4>

										<!-- Desktop: Table -->
										<div class="hidden md:block">
											<div class="table-container">
												<table class="table-compact table-hover table">
													<tbody>
														{#if runway.high.headingDegt !== null}
															<tr>
																<td class="w-2/3 text-sm">True Heading</td>
																<td class="text-sm font-medium">{runway.high.headingDegt}°</td>
															</tr>
														{/if}
														{#if runway.high.elevationFt !== null}
															<tr>
																<td class="w-2/3 text-sm">Elevation</td>
																<td class="text-sm font-medium">{runway.high.elevationFt} ft</td>
															</tr>
														{/if}
														{#if runway.high.displacedThresholdFt !== null && runway.high.displacedThresholdFt > 0}
															<tr>
																<td class="w-2/3 text-sm">Displaced Threshold</td>
																<td class="text-sm font-medium"
																	>{runway.high.displacedThresholdFt} ft</td
																>
															</tr>
														{/if}
													</tbody>
												</table>
											</div>
										</div>

										<!-- Mobile: Definition List -->
										<dl class="space-y-2 text-sm md:hidden">
											{#if runway.high.headingDegt !== null}
												<div class="flex justify-between gap-4">
													<dt class="text-surface-600-300-token">True Heading</dt>
													<dd class="font-medium">{runway.high.headingDegt}°</dd>
												</div>
											{/if}
											{#if runway.high.elevationFt !== null}
												<div class="flex justify-between gap-4">
													<dt class="text-surface-600-300-token">Elevation</dt>
													<dd class="font-medium">{runway.high.elevationFt} ft</dd>
												</div>
											{/if}
											{#if runway.high.displacedThresholdFt !== null && runway.high.displacedThresholdFt > 0}
												<div class="flex justify-between gap-4">
													<dt class="text-surface-600-300-token">Displaced Threshold</dt>
													<dd class="font-medium">{runway.high.displacedThresholdFt} ft</dd>
												</div>
											{/if}
										</dl>
									</div>
								</div>
							</div>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Clubs Based at Airport -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Users class="h-6 w-6" />
					Clubs Based at This Airport
				</h2>

				<!-- Clubs Loading State -->
				{#if clubsLoading}
					<div class="flex items-center justify-center space-x-4 p-8">
						<Progress class="h-6 w-6" />
						<span>Loading clubs...</span>
					</div>
				{/if}

				<!-- Clubs Error State -->
				{#if clubsError}
					<div class="alert mb-4 preset-filled-error-500">
						<div class="alert-message">
							<p>{clubsError}</p>
						</div>
					</div>
				{/if}

				<!-- Clubs List -->
				{#if !clubsLoading && !clubsError}
					{#if clubs.length === 0}
						<p class="text-surface-500">No clubs are based at this airport.</p>
					{:else}
						<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
							{#each clubs as club (club.id)}
								<a
									href={resolve(`/clubs/${club.id}`)}
									class="card preset-tonal p-4 transition-all hover:preset-filled-primary-500"
								>
									<div class="flex items-center gap-3">
										<Users class="h-5 w-5 text-primary-500" />
										<div class="flex-1">
											<h3 class="font-semibold">{club.name}</h3>
										</div>
									</div>
								</a>
							{/each}
						</div>
					{/if}
				{/if}
			</div>

			<!-- Recent Flights Section (Last 24 Hours) -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Clock class="h-6 w-6" />
					Recent Flights (Last 24 Hours)
				</h2>

				<!-- Flights Loading State -->
				{#if flightsLoading}
					<div class="flex items-center justify-center space-x-4 p-8">
						<Progress class="h-6 w-6" />
						<span>Loading flights...</span>
					</div>
				{/if}

				<!-- Flights Error State -->
				{#if flightsError}
					<div class="alert mb-4 preset-filled-error-500">
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
						<!-- Desktop: Table -->
						<div class="hidden md:block">
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
										{#each flights as flight (flight.id)}
											<tr>
												<td>
													{#if flight.registration}
														<div class="flex flex-col">
															<span class="font-mono font-semibold">{flight.registration}</span>
														</div>
													{:else}
														<span class="font-mono">{flight.deviceAddress}</span>
													{/if}
												</td>
												<td>
													<span
														class="badge preset-tonal"
														class:preset-tonal-primary-500={getFlightType(
															flight,
															airport?.ident || ''
														) === 'Departure'}
														class:preset-tonal-success-500={getFlightType(
															flight,
															airport?.ident || ''
														) === 'Arrival'}
														class:preset-tonal-secondary-500={getFlightType(
															flight,
															airport?.ident || ''
														) === 'Local'}
													>
														{getFlightType(flight, airport?.ident || '')}
													</span>
												</td>
												<td>
													<span class="font-mono text-sm">
														{flight.departureAirport || '—'}
													</span>
												</td>
												<td>
													<span class="font-mono text-sm">
														{flight.arrivalAirport || '—'}
													</span>
												</td>
												<td>{formatDateTime(flight.takeoffTime)}</td>
												<td>{formatDateTime(flight.landingTime)}</td>
												<td>
													{#if getFlightStatus(flight) === 'In Progress'}
														<span class="badge preset-filled-warning-500">In Progress</span>
													{:else}
														<span class="badge preset-filled-success-500">Completed</span>
													{/if}
												</td>
												<td>
													<a
														href={resolve(`/flights/${flight.id}`)}
														class="preset-tonal-primary-500 btn btn-sm"
													>
														View
													</a>
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							</div>
						</div>

						<!-- Mobile: Cards -->
						<div class="space-y-4 md:hidden">
							{#each flights as flight (flight.id)}
								<div class="card p-4">
									<div class="mb-3 flex items-start justify-between gap-2">
										<div>
											{#if flight.registration}
												<div class="font-mono font-semibold">
													{flight.registration}
												</div>
											{:else}
												<div class="font-mono">{flight.deviceAddress}</div>
											{/if}
										</div>
										<div class="flex flex-col items-end gap-2">
											<span
												class="badge preset-tonal text-xs"
												class:preset-tonal-primary-500={getFlightType(
													flight,
													airport?.ident || ''
												) === 'Departure'}
												class:preset-tonal-success-500={getFlightType(
													flight,
													airport?.ident || ''
												) === 'Arrival'}
												class:preset-tonal-secondary-500={getFlightType(
													flight,
													airport?.ident || ''
												) === 'Local'}
											>
												{getFlightType(flight, airport?.ident || '')}
											</span>
											{#if getFlightStatus(flight) === 'In Progress'}
												<span class="badge preset-filled-warning-500 text-xs">In Progress</span>
											{:else}
												<span class="badge preset-filled-success-500 text-xs">Completed</span>
											{/if}
										</div>
									</div>

									<dl class="mb-4 space-y-2 text-sm">
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token">Departure</dt>
											<dd class="font-mono font-medium">
												{flight.departureAirport || '—'}
											</dd>
										</div>
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token">Arrival</dt>
											<dd class="font-mono font-medium">
												{flight.arrivalAirport || '—'}
											</dd>
										</div>
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token">Takeoff</dt>
											<dd class="font-medium">{formatDateTime(flight.takeoffTime)}</dd>
										</div>
										<div class="flex justify-between gap-4">
											<dt class="text-surface-600-300-token">Landing</dt>
											<dd class="font-medium">{formatDateTime(flight.landingTime)}</dd>
										</div>
									</dl>

									<a
										href={resolve(`/flights/${flight.id}`)}
										class="preset-tonal-primary-500 btn w-full btn-sm"
									>
										View Flight Details
									</a>
								</div>
							{/each}
						</div>
					{/if}
				{/if}
			</div>

			<!-- Map Section -->
			{#if airport.latitudeDeg && airport.longitudeDeg}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Location Map
					</h2>
					<div class="border-surface-300-600-token overflow-hidden rounded-lg border">
						<!-- Embedded Google Map -->
						<iframe
							src={`https://maps.google.com/maps?q=${airport.latitudeDeg},${airport.longitudeDeg}&output=embed`}
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
							class="preset-tonal-primary-500 btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						<a
							href={`https://www.google.com/maps/dir/?api=1&destination=${airport.latitudeDeg},${airport.longitudeDeg}`}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-tonal-secondary-500 btn btn-sm"
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
