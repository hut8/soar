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
		UserCheck,
		ExternalLink,
		Image,
		ClipboardList
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import type { ClubWithSoaring, User } from '$lib/types';
	import { getStatusCodeDescription, getAircraftTypeOgnDescription } from '$lib/formatters';

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
						<div class="flex flex-shrink-0 flex-col gap-2">
							<div
								class="inline-flex items-center gap-2 rounded-full bg-success-500 px-4 py-2 text-sm text-white"
							>
								<UserCheck class="h-4 w-4" />
								My Club
							</div>
							<a
								href={resolve(`/clubs/${clubId}/operations`)}
								class="btn preset-filled-secondary-500 btn-sm"
							>
								<ClipboardList class="mr-2 h-4 w-4" />
								Club Operations
							</a>
						</div>
					{/if}
				</div>
			</div>

			<!-- Home Base Airport Section -->
			{#if club.home_base_airport_id}
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<MapPin class="h-6 w-6" />
						Location
					</h2>

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
												{airport.municipality}{airport.iso_region ? `, ${airport.iso_region}` : ''}
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
																{runway.surface}{runway.lighted ? ' • Lighted' : ''}{runway.closed
																	? ' • Closed'
																	: ''}
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
				</div>
			{/if}

			<!-- Club Aircraft Section -->
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
					<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
						{#each aircraft as plane (plane.registration_number)}
							<div class="card p-4">
								<div class="mb-3 flex flex-wrap items-center gap-2">
									<h3 class="h3 font-semibold">{plane.registration_number}</h3>
									{#if plane.aircraft_type_ogn === 'tow_tug'}
										<span
											class="btn preset-filled-warning-500 btn-sm"
											title="This aircraft is a tow plane"
										>
											<Plane class="h-4 w-4" />
											Tow/Tug
										</span>
									{/if}
								</div>

								<div class="mb-3 flex flex-wrap gap-2">
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
								</div>

								<div class="space-y-2 text-sm">
									{#if plane.transponder_code}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">ICAO Code:</span>
											<span class="ml-2"
												>{plane.transponder_code.toString(16).toUpperCase().padStart(4, '0')}</span
											>
										</div>
									{/if}
									{#if plane.manufacturer_model_code}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Model:</span>
											<span class="ml-2">{plane.manufacturer_model_code}</span>
										</div>
									{/if}
									{#if plane.year_manufactured}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Year:</span>
											<span class="ml-2">{plane.year_manufactured}</span>
										</div>
									{/if}
									{#if plane.serial_number}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Serial:</span>
											<span class="ml-2">{plane.serial_number}</span>
										</div>
									{/if}
									{#if plane.registrant_name}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Owner:</span>
											<span class="ml-2">{plane.registrant_name}</span>
										</div>
									{/if}
									{#if plane.aircraft_type}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Type:</span>
											<span class="ml-2">{plane.aircraft_type}</span>
										</div>
									{/if}
									{#if plane.model?.manufacturer_name && plane.model?.model_name}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Make and Model:</span>
											<span class="ml-2"
												>{plane.model.manufacturer_name}
												{plane.model.model_name}</span
											>
										</div>
									{/if}
									{#if plane.aircraft_type_ogn}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium"
												>Aircraft Type (OGN):</span
											>
											<span class="ml-2"
												>{getAircraftTypeOgnDescription(plane.aircraft_type_ogn)}</span
											>
										</div>
									{/if}
									{#if plane.engine_manufacturer_model_code}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Engine:</span>
											<span class="ml-2">{plane.engine_manufacturer_model_code}</span>
										</div>
									{/if}
									{#if plane.status_code}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Status:</span>
											<span class="ml-2">{getStatusCodeDescription(plane.status_code)}</span>
										</div>
									{/if}
									{#if plane.airworthiness_class}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium"
												>Airworthiness Class:</span
											>
											<span class="ml-2">{plane.airworthiness_class}</span>
										</div>
									{/if}
									{#if plane.light_sport_type}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Light Sport Type:</span>
											<span class="ml-2">{plane.light_sport_type}</span>
										</div>
									{/if}
									{#if plane.model?.number_of_engines}
										<div class="border-surface-200-700-token border-b pb-2">
											<span class="text-surface-600-300-token font-medium">Number of Engines:</span>
											<span class="ml-2">{plane.model.number_of_engines}</span>
										</div>
									{/if}
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</div>

			<!-- Map Section - Shows home base airport if available -->
			{#if airport && airport.latitude_deg && airport.longitude_deg}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Airport Location Map
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
							title="Airport location map for {airport.name}"
						></iframe>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						<a
							href={`https://www.google.com/maps/search/?api=1&query=${airport.latitude_deg},${airport.longitude_deg}`}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-tonal-primary-500 btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						<a
							href={`https://www.google.com/maps/dir/?api=1&destination=${airport.latitude_deg},${airport.longitude_deg}`}
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
