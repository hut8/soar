<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Plane,
		Activity,
		Building2,
		Save,
		Eye,
		EyeOff,
		HelpCircle,
		Image,
		History,
		FileText,
		Cog,
		ChevronDown,
		ChevronUp
	} from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { watchlist } from '$lib/stores/watchlist';
	import { getLogger } from '$lib/logging';
	import type {
		Aircraft,
		AircraftRegistration,
		AircraftModel,
		Fix,
		Flight,
		Club,
		DataResponse,
		DataListResponse,
		PaginatedDataResponse
	} from '$lib/types';

	const logger = getLogger(['soar', 'AircraftDetailsPage']);
	import FlightsList from '$lib/components/FlightsList.svelte';
	import FixesList from '$lib/components/FixesList.svelte';
	import {
		formatTitleCase,
		getAllAddresses,
		getStatusCodeDescription,
		getAircraftCategoryDescription,
		getAircraftCategoryColor,
		formatTransponderCode,
		getCountryName,
		getFlagPath
	} from '$lib/formatters';
	import {
		getEmitterCategoryLabel,
		getEmitterCategoryDescription
	} from '$lib/constants/adsbEmitterCategories';
	import { getEngineTypeLabel } from '$lib/constants/faaEngineTypes';
	import { toaster } from '$lib/toaster';
	import dayjs from 'dayjs';
	import utc from 'dayjs/plugin/utc';
	import relativeTime from 'dayjs/plugin/relativeTime';

	// Extend dayjs with plugins
	dayjs.extend(utc);
	dayjs.extend(relativeTime);

	// Local interfaces match backend paginated responses
	// Backend returns: { data: Fix[], metadata: { page, totalPages, totalCount } }
	type FixesResponse = PaginatedDataResponse<Fix>;
	type FlightsResponse = PaginatedDataResponse<Flight>;

	// Aircraft images interfaces
	interface AircraftImage {
		source: 'airport_data' | 'planespotters';
		pageUrl: string;
		thumbnailUrl: string;
		imageUrl?: string;
		photographer?: string;
	}

	interface AircraftImageCollection {
		images: AircraftImage[];
		lastFetched: Record<string, string>;
	}

	let aircraft: Aircraft | null = null;
	let aircraftRegistration: AircraftRegistration | null = null;
	let aircraftModel: AircraftModel | null = null;
	let fixes: Fix[] = [];
	let flights: Flight[] = [];
	let loading = true;
	let loadingFixes = true;
	let loadingFlights = true;
	let error = '';
	let fixesPage = 1;
	let flightsPage = 1;
	let fixesTotalPages = 1;
	let flightsTotalPages = 1;
	let hideInactiveFixes = false;
	let clubs: Club[] = [];
	let selectedClubId: string = '';
	let savingClub = false;
	let clubName: string | null = null;
	let aircraftImages: AircraftImage[] = [];
	let loadingImages = true;
	let isFixesCollapsed = true;

	$: aircraftId = $page.params.id || '';
	$: isAdmin = $auth.user?.isAdmin === true;
	$: userClubId = $auth.user?.clubId;
	$: isInWatchlist = watchlist.has(aircraftId);

	// Generate JSON-LD structured data for SEO (reactive to aircraft and aircraftId changes)
	$: jsonLdScript = (() => {
		const data = {
			'@context': 'https://schema.org',
			'@type': 'WebPage',
			name: aircraft?.registration
				? `${aircraft.registration} - Aircraft Details`
				: 'Aircraft Details',
			description: aircraft
				? `Aircraft details for ${aircraft.registration || 'unknown'}${aircraft.aircraftModel ? `, a ${aircraft.aircraftModel}` : ''}`
				: 'View aircraft details including flight history and registration information',
			url: `https://glider.flights/aircraft/${aircraftId}`,
			mainEntity: aircraft
				? {
						'@type': 'Vehicle',
						name: aircraft.registration || undefined,
						model: aircraft.aircraftModel || undefined,
						vehicleIdentificationNumber:
							aircraft.icaoAddress ||
							aircraft.flarmAddress ||
							aircraft.ognAddress ||
							aircraft.otherAddress ||
							undefined,
						category: aircraft.aircraftCategory || 'Aircraft'
					}
				: undefined
		};
		return '<script type="application/ld+json">' + JSON.stringify(data) + '</' + 'script>';
	})();

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

	onMount(async () => {
		if (aircraftId) {
			await loadAircraft();
			// Load fixes, flights, images, and clubs in parallel after aircraft data is ready
			await Promise.all([
				loadFixes(),
				loadFlights(),
				loadImages(),
				...(isAdmin ? [loadClubs()] : [])
			]);
		}
	});

	async function loadAircraft() {
		loading = true;
		error = '';

		try {
			// Load aircraft data
			const aircraftResponse = await serverCall<DataResponse<Aircraft>>(`/aircraft/${aircraftId}`);
			aircraft = aircraftResponse.data;

			// Initialize selected club ID if aircraft has one
			if (aircraft.clubId) {
				selectedClubId = aircraft.clubId;
			}

			// Load aircraft registration, model, and club data in parallel
			const [registrationResponse, modelResponse, clubResponse] = await Promise.all([
				serverCall<DataResponse<AircraftRegistration>>(
					`/aircraft/${aircraftId}/registration`
				).catch(() => null),
				serverCall<DataResponse<AircraftModel>>(`/aircraft/${aircraftId}/model`).catch(() => null),
				aircraft.clubId
					? serverCall<DataResponse<Club>>(`/clubs/${aircraft.clubId}`).catch(() => null)
					: Promise.resolve(null)
			]);

			aircraftRegistration = registrationResponse?.data || null;
			aircraftModel = modelResponse?.data || null;
			clubName = clubResponse?.data?.name || null;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load aircraft: ${errorMessage}`;
			logger.error('Error loading aircraft: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	async function loadFixes(page: number = 1) {
		loadingFixes = true;
		try {
			// Calculate timestamp for 24 hours ago in ISO 8601 UTC format
			const twentyFourHoursAgo = dayjs().utc().subtract(24, 'hour');
			const after = twentyFourHoursAgo.toISOString();

			const response = await serverCall<FixesResponse>(`/aircraft/${aircraftId}/fixes`, {
				params: {
					page,
					perPage: 5,
					after,
					...(hideInactiveFixes && { active: true })
				}
			});
			fixes = response.data;
			fixesPage = response.metadata.page;
			fixesTotalPages = response.metadata.totalPages;
		} catch (err) {
			logger.error('Failed to load fixes: {error}', { error: err });
		} finally {
			loadingFixes = false;
		}
	}

	function handleHideInactiveChange(value: boolean) {
		hideInactiveFixes = value;
		loadFixes(1); // Reset to page 1 when filter changes
	}

	async function loadFlights(page: number = 1) {
		loadingFlights = true;
		try {
			const response = await serverCall<FlightsResponse>(
				`/aircraft/${aircraftId}/flights?page=${page}&perPage=5`
			);
			flights = response.data || [];
			// Sort by takeoffTime if available, otherwise createdAt (most recent first)
			// This matches the backend COALESCE(takeoff_time, created_at) sorting
			flights.sort((a, b) => {
				const timeA = a.takeoffTime
					? new Date(a.takeoffTime).getTime()
					: new Date(a.createdAt).getTime();
				const timeB = b.takeoffTime
					? new Date(b.takeoffTime).getTime()
					: new Date(b.createdAt).getTime();
				return timeB - timeA;
			});
			flightsPage = response.metadata.page || 1;
			flightsTotalPages = response.metadata.totalPages || 1;
		} catch (err) {
			logger.error('Failed to load flights: {error}', { error: err });
			flights = [];
		} finally {
			loadingFlights = false;
		}
	}

	async function loadImages() {
		loadingImages = true;
		try {
			const response = await serverCall<DataResponse<AircraftImageCollection>>(
				`/aircraft/${aircraftId}/images`
			);
			aircraftImages = response.data.images || [];
		} catch (err) {
			logger.error('Failed to load aircraft images: {error}', { error: err });
			aircraftImages = [];
		} finally {
			loadingImages = false;
		}
	}

	function goBack() {
		goto(resolve('/aircraft'));
	}

	async function loadClubs() {
		try {
			const response = await serverCall<DataListResponse<Club>>('/clubs');
			clubs = response.data || [];
		} catch (err) {
			logger.error('Failed to load clubs: {error}', { error: err });
		}
	}

	async function assignToMyClub() {
		if (!userClubId) return;
		selectedClubId = userClubId;
		await updateDeviceClub();
	}

	async function updateDeviceClub() {
		if (!aircraftId || savingClub) return;

		savingClub = true;
		try {
			await serverCall(`/aircraft/${aircraftId}/club`, {
				method: 'PUT',
				body: JSON.stringify({ clubId: selectedClubId || null })
			});

			toaster.success({ title: 'Aircraft club assignment updated successfully' });

			// Reload aircraft to get updated data
			await loadAircraft();
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			toaster.error({ title: `Failed to update club assignment: ${errorMessage}` });
			logger.error('Error updating aircraft club: {error}', { error: err });
		} finally {
			savingClub = false;
		}
	}

	async function toggleWatchlist() {
		if (!$auth.isAuthenticated) {
			toaster.warning({ title: 'Please log in to use watchlist' });
			return;
		}

		try {
			if (isInWatchlist) {
				await watchlist.remove(aircraftId);
				toaster.success({ title: 'Removed from watchlist' });
			} else {
				await watchlist.add(aircraftId, false);
				toaster.success({ title: 'Added to watchlist' });
			}
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			toaster.error({ title: 'Failed to update watchlist', description: errorMessage });
		}
	}
</script>

<svelte:head>
	<title>{aircraft?.registration || 'Aircraft'} - Aircraft Details - SOAR</title>
	<meta
		name="description"
		content={aircraft
			? `View details for ${aircraft.registration || 'aircraft'}${aircraft.aircraftModel ? ` (${aircraft.aircraftModel})` : ''} including flight history, registration info, and real-time tracking data.`
			: 'View aircraft details including flight history, registration information, photos, and real-time position tracking on SOAR.'}
	/>
	<link rel="canonical" href="https://glider.flights/aircraft/{aircraftId}" />

	<!-- JSON-LD structured data for SEO -->
	<!-- eslint-disable-next-line svelte/no-at-html-tags -->
	{@html jsonLdScript}
</svelte:head>

<div class="container mx-auto max-w-6xl space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn preset-tonal btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Aircraft
		</button>

		{#if $auth.isAuthenticated}
			<button
				class="btn {isInWatchlist
					? 'preset-filled-warning-500'
					: 'preset-tonal-primary-500'} btn-sm"
				onclick={toggleWatchlist}
			>
				{#if isInWatchlist}
					<Eye class="h-4 w-4" />
					Watching
				{:else}
					<EyeOff class="h-4 w-4" />
					Watch Aircraft
				{/if}
			</button>
		{/if}
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<Progress class="h-8 w-8" />
				<span class="text-lg">Loading aircraft details...</span>
			</div>

			<!-- SEO fallback content - visible during loading for crawlers -->
			<div class="text-surface-600-300-token mt-6 space-y-4">
				<h1 class="h2">Aircraft Details</h1>
				<p>
					This page displays detailed information about a tracked aircraft on SOAR (Soaring
					Observation And Records), including aircraft registration, model details, flight history,
					real-time position tracking, and photos.
				</p>
				<p>
					SOAR tracks gliders, sailplanes, and other aircraft using data from the Open Glider
					Network (OGN) and ADS-B receivers worldwide. View <a href="/aircraft" class="anchor"
						>all tracked aircraft</a
					>
					or <a href="/flights" class="anchor">recent flights</a>.
				</p>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert preset-filled-error-500">
			<div class="alert-message">
				<h3 class="h3">Error Loading Aircraft</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn preset-filled" onclick={loadAircraft}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Aircraft Details -->
	{#if !loading && !error && aircraft}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Plane class="h-8 w-8 text-primary-500" />
							<div>
								<h1 class="h1">
									{aircraft.registration || 'Unknown'}
									{#if aircraft.competitionNumber}
										<span class="text-surface-600-300-token">({aircraft.competitionNumber})</span>
									{/if}
								</h1>
								{#if aircraft.aircraftModel}
									<p class="text-lg">{aircraft.aircraftModel}</p>
								{/if}
								{#if aircraft.icaoModelCode}
									<p class="text-surface-600-300-token text-sm">
										ICAO Model Code: <span class="font-mono">{aircraft.icaoModelCode}</span>
									</p>
								{/if}
								{#if aircraft.modelData}
									<div class="text-surface-600-300-token mt-1 text-sm">
										{#if aircraft.modelData.manufacturer}
											<span>{aircraft.modelData.manufacturer}</span>
											{#if aircraft.modelData.description}
												<span class="mx-1">&middot;</span>
											{/if}
										{/if}
										{#if aircraft.modelData.description}
											<span>{aircraft.modelData.description}</span>
										{/if}
										{#if aircraft.modelData.wingType}
											<span class="mx-1">&middot;</span>
											<span>{aircraft.modelData.wingType}</span>
										{/if}
										{#if aircraft.modelData.aircraftCategory}
											<span class="mx-1">&middot;</span>
											<span>{aircraft.modelData.aircraftCategory}</span>
										{/if}
									</div>
								{/if}
								{#each getAllAddresses(aircraft) as addr (addr.label)}
									<p class="text-surface-600-300-token font-mono text-sm">
										{addr.label}: {addr.hex}
									</p>
								{/each}
							</div>
						</div>

						<div class="mt-3 flex flex-wrap gap-2">
							{#if aircraft.fromOgnDdb}
								<span
									class="badge {aircraft.tracked
										? 'preset-filled-success-500'
										: 'preset-filled-surface-500'}"
								>
									{aircraft.tracked ? 'Tracked' : 'Not Tracked'}
								</span>
								<span
									class="badge {aircraft.identified
										? 'preset-filled-primary-500'
										: 'preset-filled-surface-500'}"
								>
									{aircraft.identified ? 'Identified' : 'Unidentified'}
								</span>
							{/if}
							<span
								class="badge {aircraft.fromOgnDdb
									? 'preset-filled-success-500'
									: 'preset-filled-secondary-500'}"
							>
								{aircraft.fromOgnDdb ? 'In Unified DDB' : 'Not in Unified DDB'}
							</span>
							<span
								class="badge {aircraft.fromAdsbxDdb
									? 'preset-filled-success-500'
									: 'preset-filled-secondary-500'}"
							>
								{aircraft.fromAdsbxDdb ? 'In ADSB Exchange DB' : 'Not in ADSB Exchange DB'}
							</span>
							{#if aircraft.aircraftCategory}
								<span class="badge {getAircraftCategoryColor(aircraft.aircraftCategory)} text-xs">
									{getAircraftCategoryDescription(aircraft.aircraftCategory)}
								</span>
							{/if}
							{#if aircraft.trackerDeviceType}
								<span class="badge preset-filled-tertiary-500 text-xs">
									OGN Tracker Device Type: {aircraft.trackerDeviceType}
								</span>
							{/if}
							{#if aircraft.countryCode}
								{@const countryName = getCountryName(aircraft.countryCode)}
								{@const flagPath = getFlagPath(aircraft.countryCode)}
								<span class="badge flex items-center gap-1.5 preset-filled-tertiary-500 text-xs">
									{#if flagPath}
										<img
											src={flagPath}
											alt="{aircraft.countryCode} flag"
											class="h-3 w-4 object-cover"
										/>
									{/if}
									{countryName ? `${countryName} (${aircraft.countryCode})` : aircraft.countryCode}
								</span>
							{/if}
							{#if aircraft.adsbEmitterCategory}
								<span
									class="badge flex cursor-help items-center gap-1.5 preset-filled-primary-500 text-xs"
									title="{aircraft.adsbEmitterCategory.toUpperCase()} - {getEmitterCategoryDescription(
										aircraft.adsbEmitterCategory
									)}"
								>
									ADS-B: {getEmitterCategoryLabel(aircraft.adsbEmitterCategory)}
									<HelpCircle class="h-3 w-3" />
								</span>
							{/if}
						</div>
					</div>
				</div>
			</div>

			<!-- Aircraft Images Section -->
			{#if !loadingImages && aircraftImages.length > 0}
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Image class="h-6 w-6" />
						Aircraft Photos
					</h2>

					<div class="flex gap-4 overflow-x-auto pb-2">
						{#each aircraftImages as image (image.thumbnailUrl)}
							<a
								href={image.pageUrl}
								target="_blank"
								rel="noopener noreferrer"
								class="group relative flex-shrink-0"
							>
								<img
									src={image.thumbnailUrl}
									alt="Aircraft photo{image.photographer ? ` by ${image.photographer}` : ''}"
									class="border-surface-300-600-token h-48 rounded-lg border object-cover transition-all group-hover:border-primary-500 group-hover:shadow-lg"
									loading="lazy"
								/>
								{#if image.photographer}
									<p class="text-surface-600-300-token mt-1 text-center text-xs">
										© {image.photographer}
									</p>
								{/if}
								<p class="text-surface-600-300-token mt-0.5 text-center text-xs capitalize">
									{image.source === 'airport_data' ? 'Airport Data' : 'Planespotters'}
								</p>
							</a>
						{/each}
					</div>
				</div>
			{/if}

			<!-- Flights Section -->
			<div class="space-y-4 card p-6">
				<h2 class="flex items-center gap-2 h2">
					<History class="h-6 w-6" />
					Flight History
				</h2>

				{#if loadingFlights}
					<div class="space-y-3">
						{#each [0, 1, 2] as i (i)}
							<div class="animate-pulse card preset-tonal p-4">
								<div class="flex items-center gap-4">
									<div class="h-10 w-10 rounded-full bg-surface-300 dark:bg-surface-600"></div>
									<div class="flex-1 space-y-2">
										<div class="h-4 w-1/3 rounded bg-surface-300 dark:bg-surface-600"></div>
										<div class="h-3 w-2/3 rounded bg-surface-300 dark:bg-surface-600"></div>
									</div>
									<div class="h-4 w-16 rounded bg-surface-300 dark:bg-surface-600"></div>
								</div>
							</div>
						{/each}
					</div>
				{:else if flights.length === 0}
					<div class="text-surface-600-300-token py-8 text-center">
						<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
						<p>Flight history will appear here once this aircraft is tracked in flight.</p>
					</div>
				{:else}
					<FlightsList {flights} showEnd={true} showAircraft={false} />

					<!-- Pagination for flights -->
					{#if flightsTotalPages > 1}
						<div class="flex items-center justify-between pt-4">
							<p class="text-surface-600-300-token text-sm">
								Page {flightsPage} of {flightsTotalPages}
							</p>
							<div class="flex gap-2">
								<button
									class="btn preset-tonal btn-sm"
									disabled={flightsPage <= 1}
									onclick={() => loadFlights(flightsPage - 1)}
								>
									Previous
								</button>
								<button
									class="btn preset-tonal btn-sm"
									disabled={flightsPage >= flightsTotalPages}
									onclick={() => loadFlights(flightsPage + 1)}
								>
									Next
								</button>
							</div>
						</div>
					{/if}
				{/if}
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				<!-- Club Assignment (Admin Only) -->
				{#if isAdmin}
					<div class="space-y-4 card p-6">
						<h2 class="flex items-center gap-2 h2">
							<Building2 class="h-6 w-6" />
							Club Assignment
						</h2>

						<div class="space-y-4">
							<div>
								<label for="club-select" class="text-surface-600-300-token mb-2 block text-sm">
									Assign Aircraft to Club
								</label>
								<select
									id="club-select"
									class="select"
									bind:value={selectedClubId}
									disabled={savingClub}
								>
									<option value="">No club assigned</option>
									{#each clubs as club (club.id)}
										<option value={club.id}>{club.name}</option>
									{/each}
								</select>
							</div>

							<div class="flex gap-2">
								<button
									class="btn preset-filled-primary-500"
									onclick={updateDeviceClub}
									disabled={savingClub}
								>
									{#if savingClub}
										<Progress class="h-4 w-4" />
										<span>Saving...</span>
									{:else}
										<Save class="h-4 w-4" />
										<span>Save Assignment</span>
									{/if}
								</button>

								{#if userClubId}
									<button
										class="btn preset-filled-secondary-500"
										onclick={assignToMyClub}
										disabled={savingClub}
									>
										<Building2 class="h-4 w-4" />
										<span>Assign to My Club</span>
									</button>
								{/if}
							</div>
						</div>
					</div>
				{/if}
			</div>

			<!-- Aircraft Registration & Model Grid -->
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				<!-- Aircraft Model Details -->
				{#if aircraftModel}
					<div class="order-2 space-y-4 card p-6">
						<h2 class="flex items-center gap-2 h2">
							<Cog class="h-6 w-6" />
							FAA Aircraft Model Details
						</h2>

						<!-- Consolidated Model Details -->
						<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
							<div>
								<dt class="text-surface-600-300-token mb-1 font-medium">Manufacturer</dt>
								<dd class="font-semibold">{aircraftModel.manufacturerName}</dd>
							</div>
							<div>
								<dt class="text-surface-600-300-token mb-1 font-medium">Model</dt>
								<dd class="font-semibold">{aircraftModel.modelName}</dd>
							</div>
							{#if aircraftModel.aircraftType}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Aircraft Type</dt>
									<dd>{formatTitleCase(aircraftModel.aircraftType)}</dd>
								</div>
							{/if}
							{#if aircraftModel.aircraftCategory}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Category</dt>
									<dd>{formatTitleCase(aircraftModel.aircraftCategory)}</dd>
								</div>
							{/if}
							{#if aircraftModel.engineType}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Engine Type</dt>
									<dd>{formatTitleCase(aircraftModel.engineType)}</dd>
								</div>
							{/if}
							{#if aircraftModel.numberOfEngines !== null && aircraftModel.numberOfEngines !== undefined}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Engines</dt>
									<dd>{aircraftModel.numberOfEngines}</dd>
								</div>
							{/if}
							{#if aircraftModel.builderCertification}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Builder Certification</dt>
									<dd>{formatTitleCase(aircraftModel.builderCertification)}</dd>
								</div>
							{/if}
							{#if aircraftModel.weightClass}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Weight Class</dt>
									<dd>{formatTitleCase(aircraftModel.weightClass)}</dd>
								</div>
							{/if}
							{#if aircraftModel.numberOfSeats}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Seats</dt>
									<dd>{aircraftModel.numberOfSeats}</dd>
								</div>
							{/if}
							{#if aircraftModel.cruisingSpeed && aircraftModel.cruisingSpeed > 0}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Cruising Speed</dt>
									<dd>{aircraftModel.cruisingSpeed} kts</dd>
								</div>
							{/if}
							{#if aircraftModel.typeCertificateDataSheet}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">
										Type Certificate Data Sheet
									</dt>
									<dd>{aircraftModel.typeCertificateDataSheet}</dd>
								</div>
							{/if}
							{#if aircraftModel.typeCertificateDataHolder}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">
										Type Certificate Data Holder
									</dt>
									<dd>{aircraftModel.typeCertificateDataHolder}</dd>
								</div>
							{/if}
						</dl>
					</div>
				{/if}

				<!-- Aircraft Registration -->
				<div class="order-1 space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<FileText class="h-6 w-6" />
						Aircraft Registration
					</h2>

					{#if aircraftRegistration}
						<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
							<!-- Registration Number -->
							<div>
								<dt class="text-surface-600-300-token mb-1 font-medium">Registration Number</dt>
								<dd class="font-mono font-semibold">
									{aircraft.registration || aircraftRegistration.registrationNumber || 'Unknown'}
								</dd>
							</div>

							<!-- Serial Number -->
							<div>
								<dt class="text-surface-600-300-token mb-1 font-medium">Serial Number</dt>
								<dd class="font-mono">{aircraftRegistration.serialNumber}</dd>
							</div>

							<!-- Year Manufactured -->
							{#if aircraftRegistration.yearManufactured}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Year Manufactured</dt>
									<dd>{aircraftRegistration.yearManufactured}</dd>
								</div>
							{/if}

							<!-- Aircraft Type -->
							{#if aircraftRegistration.aircraftType}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Aircraft Type</dt>
									<dd>{aircraftRegistration.aircraftType}</dd>
								</div>
							{/if}

							<!-- Manufacturer/Model/Series Codes -->
							{#if aircraftRegistration.manufacturerCode || aircraftRegistration.modelCode || aircraftRegistration.seriesCode}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Mfr/Model/Series</dt>
									<dd class="font-mono">
										{[
											aircraftRegistration.manufacturerCode,
											aircraftRegistration.modelCode,
											aircraftRegistration.seriesCode
										]
											.filter(Boolean)
											.join('-')}
									</dd>
								</div>
							{/if}

							<!-- Engine Type -->
							{#if aircraftRegistration.engineType !== null && aircraftRegistration.engineType !== undefined}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Engine Type</dt>
									<dd>{getEngineTypeLabel(aircraftRegistration.engineType)}</dd>
								</div>
							{/if}

							<!-- Engine Codes -->
							{#if aircraftRegistration.engineManufacturerCode || aircraftRegistration.engineModelCode}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Engine Mfr/Model</dt>
									<dd class="font-mono">
										{[
											aircraftRegistration.engineManufacturerCode,
											aircraftRegistration.engineModelCode
										]
											.filter(Boolean)
											.join('-')}
									</dd>
								</div>
							{/if}

							<!-- Registrant Name -->
							{#if aircraftRegistration.registrantName}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Owner</dt>
									<dd>{aircraftRegistration.registrantName}</dd>
								</div>
							{/if}

							<!-- Registrant Type -->
							{#if aircraftRegistration.registrantType}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Registrant Type</dt>
									<dd>{aircraftRegistration.registrantType}</dd>
								</div>
							{/if}

							<!-- Club -->
							{#if aircraft?.clubId && clubName}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Club</dt>
									<dd>
										<a href={resolve(`/clubs/${aircraft.clubId}`)} class="anchor">
											{clubName}
										</a>
									</dd>
								</div>
							{/if}

							<!-- Home Base Airport -->
							{#if aircraft?.homeBaseAirportIdent}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Home Base Airport</dt>
									<dd>{aircraft.homeBaseAirportIdent}</dd>
								</div>
							{/if}

							<!-- Status Code -->
							{#if aircraftRegistration.statusCode}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Status</dt>
									<dd>
										{getStatusCodeDescription(aircraftRegistration.statusCode)}
										<span class="ml-1 text-xs text-surface-500"
											>({aircraftRegistration.statusCode})</span
										>
									</dd>
								</div>
							{/if}

							<!-- Transponder Code -->
							{#if aircraftRegistration.transponderCode}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Transponder Code</dt>
									<dd class="font-mono">
										{formatTransponderCode(aircraftRegistration.transponderCode)}
									</dd>
								</div>
							{/if}

							<!-- Airworthiness Class -->
							{#if aircraftRegistration.airworthinessClass}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Airworthiness Class</dt>
									<dd>{aircraftRegistration.airworthinessClass}</dd>
								</div>
							{/if}

							<!-- Airworthiness Date -->
							{#if aircraftRegistration.airworthinessDate}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Airworthiness Date</dt>
									<dd>{dayjs(aircraftRegistration.airworthinessDate).format('YYYY-MM-DD')}</dd>
								</div>
							{/if}

							<!-- Certificate Issue Date -->
							{#if aircraftRegistration.certificateIssueDate}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">
										Certificate Issue Date
									</dt>
									<dd>{dayjs(aircraftRegistration.certificateIssueDate).format('YYYY-MM-DD')}</dd>
								</div>
							{/if}

							<!-- Expiration Date -->
							{#if aircraftRegistration.expirationDate}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Expiration Date</dt>
									<dd>{dayjs(aircraftRegistration.expirationDate).format('YYYY-MM-DD')}</dd>
								</div>
							{/if}

							<!-- Kit Manufacturer/Model -->
							{#if aircraftRegistration.kitManufacturerName || aircraftRegistration.kitModelName}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Kit</dt>
									<dd>
										{[aircraftRegistration.kitManufacturerName, aircraftRegistration.kitModelName]
											.filter(Boolean)
											.join(' ')}
									</dd>
								</div>
							{/if}

							<!-- Light Sport Type -->
							{#if aircraftRegistration.lightSportType}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Light Sport Type</dt>
									<dd>{aircraftRegistration.lightSportType}</dd>
								</div>
							{/if}

							<!-- Other Names -->
							{#if aircraftRegistration.otherNames && aircraftRegistration.otherNames.length > 0}
								<div>
									<dt class="text-surface-600-300-token mb-1 font-medium">Other Names</dt>
									<dd>{aircraftRegistration.otherNames.join(', ')}</dd>
								</div>
							{/if}
						</dl>
					{:else}
						<div class="text-surface-600-300-token py-4 text-center text-xs">
							<p>
								Registration data unavailable
								<br />
								<i>(Currently only available for USA-registered aircraft)</i>
							</p>
						</div>
					{/if}
				</div>
			</div>

			<!-- Position Fixes Section -->
			<div class="space-y-4 card p-6">
				<div class="flex items-center justify-between">
					<h2 class="flex items-center gap-2 h2">
						<Activity class="h-6 w-6" />
						Recent Position Fixes
					</h2>
					<button
						class="btn preset-tonal btn-sm"
						onclick={() => (isFixesCollapsed = !isFixesCollapsed)}
						type="button"
					>
						{#if isFixesCollapsed}
							<ChevronDown class="h-4 w-4" />
							<span>Show</span>
						{:else}
							<ChevronUp class="h-4 w-4" />
							<span>Hide</span>
						{/if}
					</button>
				</div>

				<div class:hidden={isFixesCollapsed}>
					<FixesList
						{fixes}
						loading={loadingFixes}
						showHideInactive={true}
						showRaw={true}
						emptyMessage="No recent position data in the last 24 hours"
						hideInactiveValue={hideInactiveFixes}
						onHideInactiveChange={handleHideInactiveChange}
						fixesInChronologicalOrder={false}
					/>

					<!-- Pagination for fixes -->
					{#if fixesTotalPages > 1}
						<div class="flex items-center justify-between pt-4">
							<p class="text-surface-600-300-token text-sm">
								Page {fixesPage} of {fixesTotalPages}
							</p>
							<div class="flex gap-2">
								<button
									class="btn preset-tonal btn-sm"
									disabled={fixesPage <= 1}
									onclick={() => loadFixes(1)}
								>
									Newest
								</button>
								<button
									class="btn preset-tonal btn-sm"
									disabled={fixesPage <= 1}
									onclick={() => loadFixes(fixesPage - 1)}
								>
									Previous
								</button>
								<button
									class="btn preset-tonal btn-sm"
									disabled={fixesPage >= fixesTotalPages}
									onclick={() => loadFixes(fixesPage + 1)}
								>
									Next
								</button>
								<button
									class="btn preset-tonal btn-sm"
									disabled={fixesPage >= fixesTotalPages}
									onclick={() => loadFixes(fixesTotalPages)}
								>
									Oldest
								</button>
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
</div>
