<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Building,
		MapPin,
		Plane,
		Navigation,
		Info,
		UserCheck,
		ExternalLink,
		Image
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth, type User } from '$lib/stores/auth';
	import type { ClubWithSoaring } from '$lib/types';
	import { getStatusCodeDescription, formatSnakeCase } from '$lib/formatters';

	interface Aircraft {
		registration_number: string;
		serial_number: string;
		manufacturer_model_code?: string;
		engine_manufacturer_model_code?: string;
		year_manufactured?: number;
		registrant_type?: string;
		registrant_name?: string;
		aircraft_type?: string;
		engine_type?: number;
		status_code?: string;
		transponder_code?: number;
		airworthiness_class?: string;
		airworthiness_date?: string;
		certificate_issue_date?: string;
		expiration_date?: string;
		club_id?: string;
		home_base_airport_id?: string;
		kit_manufacturer_name?: string;
		kit_model_name?: string;
		other_names: string[];
		light_sport_type?: string;
		device_id?: string;
		aircraft_type_ogn?: string;
		model?: {
			manufacturer_name?: string;
			model_name?: string;
			number_of_engines?: number;
		};
	}

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
		runways: Runway[];
	}

	let club: ClubWithSoaring | null = null;
	let aircraft: Aircraft[] = [];
	let airport: Airport | null = null;
	let loading = true;
	let loadingAircraft = false;
	let loadingAirport = false;
	let error = '';
	let aircraftError = '';
	let airportError = '';
	let clubId = '';
	let settingClub = false;

	$: clubId = $page.params.id || '';
	$: isCurrentClub = $auth.user?.club_id === clubId;

	onMount(async () => {
		if (clubId) {
			await loadClub();
			await loadAircraft();
		}
	});

	$: if (club?.home_base_airport_id) {
		loadAirport(club.home_base_airport_id);
	}

	async function loadClub() {
		loading = true;
		error = '';

		try {
			club = await serverCall<ClubWithSoaring>(`/clubs/${clubId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load club: ${errorMessage}`;
			console.error('Error loading club:', err);
		} finally {
			loading = false;
		}
	}

	async function loadAircraft() {
		if (!clubId) return;

		loadingAircraft = true;
		aircraftError = '';

		try {
			aircraft = await serverCall<Aircraft[]>(`/clubs/${clubId}/aircraft`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			aircraftError = `Failed to load aircraft: ${errorMessage}`;
			console.error('Error loading aircraft:', err);
		} finally {
			loadingAircraft = false;
		}
	}

	function formatCoordinates(point: import('$lib/types').Point): string {
		return `${point.latitude.toFixed(6)}, ${point.longitude.toFixed(6)}`;
	}

	function generateGoogleMapsUrl(club: ClubWithSoaring): string {
		if (club.location?.geolocation) {
			const { latitude, longitude } = club.location.geolocation;
			return `https://www.google.com/maps/search/?api=1&query=${latitude},${longitude}`;
		} else if (club.location) {
			// Fallback to address search if no coordinates
			const address = [
				club.location.street1,
				club.location.street2,
				club.location.city,
				club.location.state,
				club.location.zip_code
			]
				.filter(Boolean)
				.join(', ');
			return `https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(address)}`;
		}
		return '';
	}

	function goBack() {
		goto(resolve('/clubs'));
	}

	async function loadAirport(airportId: number) {
		loadingAirport = true;
		airportError = '';

		try {
			airport = await serverCall<Airport>(`/airports/${airportId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			airportError = `Failed to load airport: ${errorMessage}`;
			console.error('Error loading airport:', err);
		} finally {
			loadingAirport = false;
		}
	}

	async function setAsMyClub() {
		if (!$auth.isAuthenticated || !club) return;

		settingClub = true;
		try {
			const updatedUser = await serverCall<User>(`/users/set-club`, {
				method: 'PUT',
				body: JSON.stringify({ club_id: club.id })
			});

			// Update the auth store with the new user data
			auth.updateUser(updatedUser);

			// Load aircraft now that user is part of the club
			await loadAircraft();
		} catch (err) {
			console.error('Error setting club:', err);
			// You could add a toast notification here
		} finally {
			settingClub = false;
		}
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club Details'} - Soaring Clubs</title>
</svelte:head>

<div class="max-w-8xl container mx-auto space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn preset-tonal btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Clubs
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Loading club details...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert preset-filled-error-500">
			<div class="alert-message">
				<h3 class="h3">Error Loading Club</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn preset-filled" onclick={loadClub}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Club Details -->
	{#if !loading && !error && club}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Building class="h-8 w-10 text-primary-500" />
							<h1 class="h1">{club.name}</h1>
						</div>
						{#if club.is_soaring}
							<div
								class="inline-flex items-center gap-2 rounded-full bg-primary-500 px-3 py-1 text-sm text-white"
							>
								<Plane class="h-4 w-4" />
								Soaring Club
							</div>
						{/if}
					</div>

					<!-- Set as My Club Button -->
					{#if $auth.isAuthenticated && !isCurrentClub}
						<div class="flex-shrink-0">
							<button
								class="btn preset-filled-primary-500"
								onclick={setAsMyClub}
								disabled={settingClub}
							>
								{#if settingClub}
									<ProgressRing size="w-4 h-4" />
								{:else}
									<UserCheck class="mr-2 h-4 w-4" />
								{/if}
								{settingClub ? 'Setting...' : 'Set as My Club'}
							</button>
						</div>
					{:else if $auth.isAuthenticated && isCurrentClub}
						<div class="flex-shrink-0">
							<div
								class="inline-flex items-center gap-2 rounded-full bg-success-500 px-4 py-2 text-sm text-white"
							>
								<UserCheck class="h-4 w-4" />
								My Club
							</div>
						</div>
					{/if}
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
						<div class="flex items-start gap-3">
							<Info class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Address</p>
								{#if club.location}
									<p>
										{club.location.street1 || ''}<br />
										{club.location.street2 && `${club.location.street2}<br />`}
										{club.location.city && `${club.location.city}, `}
										{club.location.state && `${club.location.state}`}
										{club.location.zip_code && ` ${club.location.zip_code}`}
										<br />{club.location.country_mail_code && `${club.location.country_mail_code}`}
									</p>
								{:else}
									<p class="text-surface-500">Address not available</p>
								{/if}
							</div>
						</div>

						{#if club.location?.geolocation}
							<div class="flex items-start gap-3">
								<Navigation class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Coordinates</p>
									<p class="font-mono text-sm">{formatCoordinates(club.location.geolocation)}</p>
								</div>
							</div>
						{/if}

						<!-- Google Maps Links -->
						{#if club.location && generateGoogleMapsUrl(club)}
							<div class="border-surface-200-700-token border-t pt-3">
								<div class="flex flex-wrap gap-2">
									<a
										href={generateGoogleMapsUrl(club)}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-primary-500 btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Open in Google Maps
									</a>
									{#if club.location?.geolocation}
										<a
											href={`https://www.google.com/maps/dir/?api=1&destination=${club.location.geolocation.latitude},${club.location.geolocation.longitude}`}
											target="_blank"
											rel="noopener noreferrer"
											class="preset-tonal-secondary-500 btn btn-sm"
										>
											<Navigation class="mr-2 h-4 w-4" />
											Get Directions
										</a>
									{/if}
								</div>
							</div>
						{/if}

						{#if club.home_base_airport_id}
							<div class="flex items-start gap-3">
								<Plane class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-2 text-sm">Home Base Airport</p>
									{#if loadingAirport}
										<div class="flex items-center gap-2">
											<ProgressRing size="w-4 h-4" />
											<span class="text-sm">Loading airport...</span>
										</div>
									{:else if airportError}
										<div class="text-sm text-error-500">
											{airportError}
										</div>
									{:else if airport}
										<div class="space-y-2">
											<div>
												<p class="font-semibold">{airport.name} ({airport.ident})</p>
												{#if airport.municipality}
													<p class="text-surface-600-300-token text-sm">
														{airport.municipality}{airport.iso_region
															? `, ${airport.iso_region}`
															: ''}
													</p>
												{/if}
											</div>

											{#if airport.elevation_ft}
												<p class="text-sm">
													<span class="text-surface-600-300-token">Elevation:</span>
													{airport.elevation_ft} ft
												</p>
											{/if}

											{#if airport.runways.length > 0}
												<div>
													<p class="text-surface-600-300-token mb-1 text-sm">Runways:</p>
													<div class="space-y-1">
														{#each airport.runways as runway (runway.id)}
															<div class="bg-surface-50-900-token rounded p-2 text-sm">
																<div class="flex items-center justify-between">
																	<span class="font-medium">
																		{runway.low.ident || 'N/A'}/{runway.high.ident || 'N/A'}
																	</span>
																	{#if runway.length_ft}
																		<span class="text-surface-600-300-token">
																			{runway.length_ft}' × {runway.width_ft || 0}'
																		</span>
																	{/if}
																</div>
																{#if runway.surface}
																	<p class="text-surface-600-300-token text-xs">
																		{runway.surface}{runway.lighted
																			? ' • Lighted'
																			: ''}{runway.closed ? ' • Closed' : ''}
																	</p>
																{/if}
															</div>
														{/each}
													</div>
												</div>
											{/if}
										</div>
									{:else}
										<p class="text-surface-600-300-token text-sm">
											Airport ID: {club.home_base_airport_id}
										</p>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Aircraft Information -->

				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Plane class="h-6 w-6" />
						Club Aircraft
					</h2>

					{#if loadingAircraft}
						<div class="flex items-center justify-center space-x-4 py-8">
							<ProgressRing size="w-6 h-6" />
							<span>Loading aircraft...</span>
						</div>
					{:else if aircraftError}
						<div class="alert preset-filled-error-500">
							<div class="alert-message">
								<p>{aircraftError}</p>
								<div class="alert-actions">
									<button class="btn preset-filled" onclick={loadAircraft}> Try Again </button>
								</div>
							</div>
						</div>
					{:else if aircraft.length === 0}
						<div class="text-surface-600-300-token py-8 text-center">
							<Plane class="mx-auto mb-4 h-12 w-12 text-surface-500" />
							<p>No aircraft registered to this club</p>
						</div>
					{:else}
						<div class="space-y-4">
							{#each aircraft as plane (plane.registration_number)}
								<div class="bg-surface-100-800-token rounded-lg p-4">
									<div class="mb-3 flex flex-wrap items-center gap-3">
										<h3 class="h3 font-semibold">{plane.registration_number}</h3>
										{#if plane.device_id}
											<a
												href={`/devices/${plane.device_id}`}
												target="_blank"
												rel="noopener noreferrer"
												class="preset-tonal-primary-500 btn btn-sm"
												title="View device details"
											>
												<Plane class="h-4 w-4" />
												Device
											</a>
										{/if}
										<a
											href={`https://www.flightaware.com/photos/aircraft/${plane.registration_number}`}
											target="_blank"
											rel="noopener noreferrer"
											class="preset-tonal-primary-500 btn btn-sm"
											title="View photos on FlightAware"
										>
											<Image class="h-4 w-4" />
											Photos
										</a>
										{#if plane.aircraft_type_ogn === 'TowTug'}
											<span
												class="btn preset-filled-warning-500 btn-sm"
												title="This aircraft is a tow plane"
											>
												<Plane class="h-4 w-4" />
												Tow/Tug
											</span>
										{/if}
									</div>

									<div class="overflow-x-auto">
										<table class="w-full">
											<tbody class="space-y-1">
												{#if plane.transponder_code}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															ICAO Code:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.transponder_code.toString(16).toUpperCase().padStart(4, '0')}
														</td>
													</tr>
												{/if}
												{#if plane.manufacturer_model_code}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Model:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.manufacturer_model_code}
														</td>
													</tr>
												{/if}
												{#if plane.year_manufactured}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Year:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.year_manufactured}
														</td>
													</tr>
												{/if}
												{#if plane.serial_number}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Serial:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.serial_number}
														</td>
													</tr>
												{/if}
												{#if plane.registrant_name}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Owner:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.registrant_name}
														</td>
													</tr>
												{/if}
												{#if plane.aircraft_type}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Type:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.aircraft_type}
														</td>
													</tr>
												{/if}
												{#if plane.model?.manufacturer_name && plane.model?.model_name}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Make and Model:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.model.manufacturer_name}
															{plane.model.model_name}
														</td>
													</tr>
												{/if}
												{#if plane.aircraft_type_ogn}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Aircraft Type (OGN):
														</td>
														<td class="py-2 text-left text-sm">
															{formatSnakeCase(plane.aircraft_type_ogn)}
														</td>
													</tr>
												{/if}
												{#if plane.engine_manufacturer_model_code}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Engine:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.engine_manufacturer_model_code}
														</td>
													</tr>
												{/if}
												{#if plane.status_code}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Status:
														</td>
														<td class="py-2 text-left text-sm">
															{getStatusCodeDescription(plane.status_code)}
														</td>
													</tr>
												{/if}
												{#if plane.airworthiness_class}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Airworthiness Class:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.airworthiness_class}
														</td>
													</tr>
												{/if}
												{#if plane.light_sport_type}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Light Sport Type:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.light_sport_type}
														</td>
													</tr>
												{/if}
												{#if plane.model?.number_of_engines}
													<tr class="border-surface-200-700-token border-b">
														<td
															class="text-surface-600-300-token py-2 pr-4 text-right text-sm font-medium"
														>
															Number of Engines:
														</td>
														<td class="py-2 text-left text-sm">
															{plane.model.number_of_engines}
														</td>
													</tr>
												{/if}
											</tbody>
										</table>
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>

			<!-- Map Section -->
			{#if club.location}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Location Map
					</h2>
					<div class="border-surface-300-600-token overflow-hidden rounded-lg border">
						<!-- Embedded Google Map -->
						<iframe
							src={`https://maps.google.com/maps?q=${encodeURIComponent([club.location.street1, club.location.street2, club.location.city, club.location.state, club.location.zip_code].filter(Boolean).join(', '))}&output=embed`}
							width="100%"
							height="500"
							style="border:0;"
							allowfullscreen
							loading="lazy"
							referrerpolicy="no-referrer-when-downgrade"
							title="Location map for {club.name}"
						></iframe>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						<a
							href={generateGoogleMapsUrl(club)}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-tonal-primary-500 btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						{#if club.location?.geolocation}
							<a
								href={`https://www.google.com/maps/dir/?api=1&destination=${club.location.geolocation.latitude},${club.location.geolocation.longitude}`}
								target="_blank"
								rel="noopener noreferrer"
								class="preset-tonal-secondary-500 btn btn-sm"
							>
								<Navigation class="mr-2 h-4 w-4" />
								Get Directions
							</a>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
