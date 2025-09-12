<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ArrowLeft, Building, MapPin, Plane, Navigation, Info, UserCheck } from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth, type User } from '$lib/stores/auth';

	interface Point {
		latitude: number;
		longitude: number;
	}

	interface Club {
		id: string;
		name: string;
		is_soaring?: boolean;
		home_base_airport_id?: number;
		location_id?: string;
		street1?: string;
		street2?: string;
		city?: string;
		state?: string;
		zip_code?: string;
		region_code?: string;
		county_mail_code?: string;
		country_mail_code?: string;
		base_location?: Point;
		created_at: string;
		updated_at: string;
	}

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

	let club: Club | null = null;
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
			if ($auth.isAuthenticated) {
				await loadAircraft();
			}
		}
	});

	async function loadClub() {
		loading = true;
		error = '';

		try {
			club = await serverCall<Club>(`/clubs/${clubId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load club: ${errorMessage}`;
			console.error('Error loading club:', err);
		} finally {
			loading = false;
		}
	}

	async function loadAircraft() {
		if (!$auth.isAuthenticated || !clubId) return;

		loadingAircraft = true;
		aircraftError = '';

		try {
			aircraft = await serverCall<Aircraft[]>(`/aircraft/club/${clubId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			aircraftError = `Failed to load aircraft: ${errorMessage}`;
			console.error('Error loading aircraft:', err);
		} finally {
			loadingAircraft = false;
		}
	}

	function formatCoordinates(point: Point): string {
		return `${point.latitude.toFixed(6)}, ${point.longitude.toFixed(6)}`;
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
								<p>
									{club.street1}<br />
									{club.street2 && `${club.street2}<br />`}
									{club.city && `${club.city}, `}
									{club.state && `${club.state}`}
									{club.zip_code && `${club.zip_code}`}
									<br />{club.country_mail_code && `${club.country_mail_code}`}
								</p>
							</div>
						</div>

						{#if club.base_location}
							<div class="flex items-start gap-3">
								<Navigation class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Coordinates</p>
									<p class="font-mono text-sm">{formatCoordinates(club.base_location)}</p>
								</div>
							</div>
						{/if}

						{#if club.home_base_airport_id}
							<div class="flex items-start gap-3">
								<Plane class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Home Base Airport</p>
									<p>Airport ID: {club.home_base_airport_id}</p>
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
							<Plane class="mx-auto mb-4 h-12 w-12 text-surface-500" />
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
												{#if plane.transponder_code}
													<span
														class="bg-surface-200-700-token rounded px-2 py-1 font-mono text-xs"
													>
														{plane.transponder_code.toString(16).toUpperCase().padStart(4, '0')}
													</span>
												{/if}
											</div>

											<div class="space-y-1 text-sm">
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

			<!-- Map Section (placeholder for future implementation) -->
			{#if club.base_location}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Location Map
					</h2>
					<div class="bg-surface-100-800-token rounded-lg p-8 text-center">
						<MapPin class="mx-auto mb-4 h-12 w-12 text-surface-500" />
						<p class="text-surface-600-300-token">Map integration coming soon</p>
						<p class="mt-2 text-sm text-surface-500">
							{formatCoordinates(club.base_location)}
						</p>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
