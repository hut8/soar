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
		ClipboardList,
		Users
	} from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall, ServerError } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import AircraftTile from '$lib/components/AircraftTile.svelte';
	import { getLogger } from '$lib/logging';
	import { toaster } from '$lib/toaster';
	import type {
		ClubWithSoaring,
		Aircraft,
		Airport,
		ClubJoinRequestView,
		DataResponse,
		DataListResponse
	} from '$lib/types';

	const logger = getLogger(['soar', 'ClubDetailsPage']);

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
	let cancellingRequest = false;
	let pendingJoinRequest: ClubJoinRequestView | null = null;
	let requestingToJoin = false;

	$: clubId = $page.params.id || '';
	$: isCurrentClub = $auth.user?.clubId === clubId;

	// Generate JSON-LD structured data for SEO (reactive to club changes)
	$: jsonLdScript = (() => {
		const data = {
			'@context': 'https://schema.org',
			'@type': 'SportsClub',
			name: club?.name || 'Soaring Club',
			description: club
				? `${club.name} is a soaring club${club.homeBaseAirportIdent ? ` based at ${club.homeBaseAirportIdent}` : ''}.`
				: 'View soaring club details including aircraft fleet, home base, and membership information.',
			url: `https://glider.flights/clubs/${clubId}`,
			sport: 'Gliding',
			...(club?.homeBaseAirportIdent && {
				location: { '@type': 'Place', name: club.homeBaseAirportIdent }
			})
		};
		return '<script type="application/ld+json">' + JSON.stringify(data) + '</' + 'script>';
	})();

	onMount(async () => {
		if (clubId) {
			await loadClub();
			await loadAircraft();
			if ($auth.isAuthenticated && !isCurrentClub) {
				await loadMyJoinRequest();
			}
		}
	});

	$: if (club?.homeBaseAirportId) {
		loadAirport(club.homeBaseAirportId);
	}

	async function loadClub() {
		loading = true;
		error = '';

		try {
			const response = await serverCall<DataResponse<ClubWithSoaring>>(`/clubs/${clubId}`);
			club = response.data;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load club: ${errorMessage}`;
			logger.error('Error loading club: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	async function loadAircraft() {
		if (!clubId) return;

		loadingAircraft = true;
		aircraftError = '';

		try {
			const response = await serverCall<DataListResponse<Aircraft>>(`/clubs/${clubId}/aircraft`);
			// Deduplicate by id to prevent Svelte each_key_duplicate errors
			const byId: Record<string, Aircraft> = {};
			for (const a of response.data || []) {
				if (a.id && !(a.id in byId)) byId[a.id] = a;
			}
			aircraft = Object.values(byId);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			aircraftError = `Failed to load aircraft: ${errorMessage}`;
			logger.error('Error loading aircraft: {error}', { error: err });
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
			const response = await serverCall<DataResponse<Airport>>(`/airports/${airportId}`);
			airport = response.data;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			airportError = `Failed to load airport: ${errorMessage}`;
			logger.error('Error loading airport: {error}', { error: err });
		} finally {
			loadingAirport = false;
		}
	}

	async function loadMyJoinRequest() {
		if (!$auth.isAuthenticated || !clubId) return;
		try {
			const request = await serverCall<ClubJoinRequestView>(`/clubs/${clubId}/join-requests/my`);
			pendingJoinRequest = request;
		} catch (err) {
			// 404 means no pending request - that's expected
			if (err instanceof ServerError && err.status === 404) {
				pendingJoinRequest = null;
				return;
			}
			// Log real errors but don't block the page
			logger.error('Error loading join request status: {error}', { error: err });
			pendingJoinRequest = null;
		}
	}

	async function requestToJoin() {
		if (!$auth.isAuthenticated || !club) return;

		requestingToJoin = true;
		try {
			const request = await serverCall<ClubJoinRequestView>(`/clubs/${club.id}/join-requests`, {
				method: 'POST',
				body: JSON.stringify({ message: null })
			});
			pendingJoinRequest = request;
			toaster.success({ title: `Join request sent to ${club.name}` });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			logger.error('Error requesting to join club: {error}', { error: err });
			toaster.error({ title: 'Failed to send join request', description: errorMessage });
		} finally {
			requestingToJoin = false;
		}
	}

	async function cancelJoinRequest() {
		if (!pendingJoinRequest || !club) return;

		cancellingRequest = true;
		try {
			await serverCall(`/clubs/${club.id}/join-requests/${pendingJoinRequest.id}`, {
				method: 'DELETE'
			});
			pendingJoinRequest = null;
			toaster.success({ title: 'Join request cancelled' });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			logger.error('Error cancelling join request: {error}', { error: err });
			toaster.error({ title: 'Failed to cancel request', description: errorMessage });
		} finally {
			cancellingRequest = false;
		}
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club Details'} - Soaring Clubs - SOAR</title>
	<meta
		name="description"
		content={club
			? `${club.name} soaring club${club.homeBaseAirportIdent ? ` based at ${club.homeBaseAirportIdent}` : ''}. View fleet, members, and club information.`
			: 'View soaring club details including aircraft fleet, home base airport, and membership information on SOAR.'}
	/>
	<link rel="canonical" href="https://glider.flights/clubs/{clubId}" />

	<!-- JSON-LD structured data for SEO -->
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html jsonLdScript}
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
				<Progress class="h-8 w-8" />
				<span class="text-lg">Loading club details...</span>
			</div>

			<!-- SEO fallback content - visible during loading for crawlers -->
			<div class="text-surface-600-300-token mt-6 space-y-4">
				<h1 class="h2">Soaring Club Details</h1>
				<p>
					This page displays detailed information about a soaring club on SOAR, including the club's
					aircraft fleet, home base airport, and membership information.
				</p>
				<p>
					SOAR connects glider pilots and soaring enthusiasts with clubs worldwide. View
					<a href="/clubs" class="anchor">all soaring clubs</a>
					or <a href="/aircraft" class="anchor">browse tracked aircraft</a>.
				</p>
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
						{#if club.isSoaring}
							<div
								class="inline-flex items-center gap-2 rounded-full bg-primary-500 px-3 py-1 text-sm text-white"
							>
								<Plane class="h-4 w-4" />
								Soaring Club
							</div>
						{/if}
					</div>

					<!-- Join / Pending / My Club actions -->
					{#if $auth.isAuthenticated && !isCurrentClub}
						<div class="flex-shrink-0">
							{#if pendingJoinRequest}
								<div class="flex flex-col gap-2">
									<div
										class="inline-flex items-center gap-2 rounded-full bg-warning-500 px-4 py-2 text-sm text-white"
									>
										<UserCheck class="h-4 w-4" />
										Request Pending
									</div>
									<button
										class="btn preset-tonal btn-sm"
										onclick={cancelJoinRequest}
										disabled={cancellingRequest}
									>
										{cancellingRequest ? 'Cancelling...' : 'Cancel Request'}
									</button>
								</div>
							{:else}
								<button
									class="btn preset-filled-primary-500"
									onclick={requestToJoin}
									disabled={requestingToJoin}
								>
									{#if requestingToJoin}
										<Progress class="h-4 w-4" />
									{:else}
										<UserCheck class="mr-2 h-4 w-4" />
									{/if}
									{requestingToJoin ? 'Sending...' : 'Request to Join'}
								</button>
							{/if}
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
							<a
								href={resolve(`/clubs/${clubId}/pilots`)}
								class="btn preset-filled-secondary-500 btn-sm"
							>
								<Users class="mr-2 h-4 w-4" />
								Pilots
							</a>
							<a
								href={resolve(`/clubs/${clubId}/admin/join-requests`)}
								class="btn preset-filled-secondary-500 btn-sm"
							>
								<UserCheck class="mr-2 h-4 w-4" />
								Join Requests
							</a>
						</div>
					{/if}
				</div>
			</div>

			<!-- Home Base Airport Section -->
			{#if club.homeBaseAirportId}
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
									<Progress class="h-4 w-4" />
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
												{airport.municipality}{airport.isoRegion ? `, ${airport.isoRegion}` : ''}
											</p>
										{/if}
									</div>

									{#if airport.elevationFt}
										<p class="text-sm">
											<span class="text-surface-600-300-token">Elevation:</span>
											{airport.elevationFt} ft
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
															{#if runway.lengthFt}
																<span class="text-surface-600-300-token">
																	{runway.lengthFt}' × {runway.widthFt || 0}'
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
									Airport ID: {club.homeBaseAirportId}
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
						<Progress class="h-6 w-6" />
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
					<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
						{#each aircraft as plane (plane.id)}
							<AircraftTile aircraft={plane} />
						{/each}
					</div>
				{/if}
			</div>

			<!-- Map Section - Shows home base airport if available -->
			{#if airport && airport.latitudeDeg && airport.longitudeDeg}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Airport Location Map
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
							title="Airport location map for {airport.name}"
						></iframe>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						<a
							href={`https://www.google.com/maps/search/?api=1&query=${airport.latitudeDeg},${airport.longitudeDeg}`}
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
