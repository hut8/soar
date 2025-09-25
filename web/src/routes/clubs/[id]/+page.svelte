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
		ExternalLink
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth, type User } from '$lib/stores/auth';
	import type { ClubWithSoaring } from '$lib/types';

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
	}

	let club: ClubWithSoaring | null = null;
	let aircraft: Aircraft[] = [];
	let loading = true;
	let loadingAircraft = false;
	let error = '';
	let aircraftError = '';
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
		<button class="variant-soft btn btn-sm" on:click={goBack}>
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
		<div class="alert variant-filled-error">
			<div class="alert-message">
				<h3 class="h3">Error Loading Club</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="variant-filled btn" on:click={loadClub}> Try Again </button>
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
							<Building class="text-primary-500 h-8 w-10" />
							<h1 class="h1">{club.name}</h1>
						</div>
						{#if club.is_soaring}
							<div
								class="bg-primary-500 inline-flex items-center gap-2 rounded-full px-3 py-1 text-sm text-white"
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
								class="variant-filled-primary btn"
								on:click={setAsMyClub}
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
								class="bg-success-500 inline-flex items-center gap-2 rounded-full px-4 py-2 text-sm text-white"
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
				<div class="card space-y-4 p-6">
					<h2 class="h2 flex items-center gap-2">
						<MapPin class="h-6 w-6" />
						Location
					</h2>

					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Info class="text-surface-500 mt-1 h-4 w-4" />
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
								<Navigation class="text-surface-500 mt-1 h-4 w-4" />
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
										class="variant-soft-primary btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Open in Google Maps
									</a>
									{#if club.location?.geolocation}
										<a
											href={`https://www.google.com/maps/dir/?api=1&destination=${club.location.geolocation.latitude},${club.location.geolocation.longitude}`}
											target="_blank"
											rel="noopener noreferrer"
											class="variant-soft-secondary btn btn-sm"
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
								<Plane class="text-surface-500 mt-1 h-4 w-4" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Home Base Airport</p>
									<p>Airport ID: {club.home_base_airport_id}</p>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Aircraft Information -->

				<div class="card space-y-4 p-6">
					<h2 class="h2 flex items-center gap-2">
						<Plane class="h-6 w-6" />
						Club Aircraft
					</h2>

					{#if loadingAircraft}
						<div class="flex items-center justify-center space-x-4 py-8">
							<ProgressRing size="w-6 h-6" />
							<span>Loading aircraft...</span>
						</div>
					{:else if aircraftError}
						<div class="alert variant-filled-error">
							<div class="alert-message">
								<p>{aircraftError}</p>
								<div class="alert-actions">
									<button class="variant-filled btn" on:click={loadAircraft}> Try Again </button>
								</div>
							</div>
						</div>
					{:else if aircraft.length === 0}
						<div class="text-surface-600-300-token py-8 text-center">
							<Plane class="text-surface-500 mx-auto mb-4 h-12 w-12" />
							<p>No aircraft registered to this club</p>
						</div>
					{:else}
						<div class="space-y-3">
							{#each aircraft as plane (plane.registration_number)}
								<div class="bg-surface-100-800-token rounded-lg p-4">
									<div class="flex items-start justify-between">
										<div class="flex-1">
											<div class="mb-2 flex items-center gap-2">
												<h3 class="h3 font-semibold">{plane.registration_number}</h3>
											</div>

											<div class="space-y-1 text-sm">
												{#if plane.transponder_code}
													<p>
														<span class="text-surface-600-300-token">ICAO Code:</span>
														{plane.transponder_code.toString(16).toUpperCase().padStart(4, '0')}
													</p>
												{/if}
												{#if plane.manufacturer_model_code}
													<p>
														<span class="text-surface-600-300-token">Model:</span>
														{plane.manufacturer_model_code}
													</p>
												{/if}
												{#if plane.year_manufactured}
													<p>
														<span class="text-surface-600-300-token">Year:</span>
														{plane.year_manufactured}
													</p>
												{/if}
												{#if plane.serial_number}
													<p>
														<span class="text-surface-600-300-token">Serial:</span>
														{plane.serial_number}
													</p>
												{/if}
												{#if plane.registrant_name}
													<p>
														<span class="text-surface-600-300-token">Owner:</span>
														{plane.registrant_name}
													</p>
												{/if}
											</div>
										</div>
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
					<h2 class="h2 mb-4 flex items-center gap-2">
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
							class="variant-ghost-primary btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						{#if club.location?.geolocation}
							<a
								href={`https://www.google.com/maps/dir/?api=1&destination=${club.location.geolocation.latitude},${club.location.geolocation.longitude}`}
								target="_blank"
								rel="noopener noreferrer"
								class="variant-ghost-secondary btn btn-sm"
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
