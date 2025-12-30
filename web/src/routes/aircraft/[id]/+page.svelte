<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Plane,
		User,
		Calendar,
		Info,
		Activity,
		Building2,
		Save,
		Eye,
		EyeOff
	} from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { watchlist } from '$lib/stores/watchlist';
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
	import FlightsList from '$lib/components/FlightsList.svelte';
	import FixesList from '$lib/components/FixesList.svelte';
	import {
		formatTitleCase,
		formatAircraftAddress,
		getStatusCodeDescription,
		getAircraftTypeOgnDescription,
		getAircraftTypeColor,
		formatTransponderCode,
		getCountryName,
		getFlagPath
	} from '$lib/formatters';
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
	let loadingFixes = false;
	let loadingFlights = false;
	let error = '';
	let aircraftId = '';
	let fixesPage = 1;
	let flightsPage = 1;
	let fixesTotalPages = 1;
	let flightsTotalPages = 1;
	let hideInactiveFixes = false;
	let clubs: Club[] = [];
	let selectedClubId: string = '';
	let savingClub = false;
	let aircraftImages: AircraftImage[] = [];
	let loadingImages = true;

	$: aircraftId = $page.params.id || '';
	$: isAdmin = $auth.user?.isAdmin === true;
	$: userClubId = $auth.user?.clubId;
	$: isInWatchlist = watchlist.has(aircraftId);

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
			await loadFixes();
			await loadFlights();
			await loadImages();
			if (isAdmin) {
				await loadClubs();
			}
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

			// Load aircraft registration and model data in parallel
			const [registrationResponse, modelResponse] = await Promise.all([
				serverCall<DataResponse<AircraftRegistration>>(
					`/aircraft/${aircraftId}/registration`
				).catch(() => null),
				serverCall<DataResponse<AircraftModel>>(`/aircraft/${aircraftId}/model`).catch(() => null)
			]);

			aircraftRegistration = registrationResponse?.data || null;
			aircraftModel = modelResponse?.data || null;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load aircraft: ${errorMessage}`;
			console.error('Error loading aircraft:', err);
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
					perPage: 10,
					after,
					...(hideInactiveFixes && { active: true })
				}
			});
			fixes = response.data;
			fixesPage = response.metadata.page;
			fixesTotalPages = response.metadata.totalPages;
		} catch (err) {
			console.error('Failed to load fixes:', err);
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
				`/aircraft/${aircraftId}/flights?page=${page}&perPage=100`
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
			console.error('Failed to load flights:', err);
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
			console.error('Failed to load aircraft images:', err);
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
			console.error('Failed to load clubs:', err);
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
			console.error('Error updating aircraft club:', err);
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
	<title>{aircraft?.registration || 'Aircraft'} - Aircraft Details</title>
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
								<p class="text-surface-600-300-token font-mono text-sm">
									Address: {formatAircraftAddress(aircraft.addressType, aircraft.address)}
								</p>
							</div>
						</div>

						<div class="mt-3 flex flex-wrap gap-2">
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
							<span
								class="badge {aircraft.fromOgnDdb
									? 'preset-filled-success-500'
									: 'preset-filled-secondary-500'}"
							>
								{aircraft.fromOgnDdb ? 'In OGN DB' : 'Not in OGN DB'}
							</span>
							<span
								class="badge {aircraft.fromAdsbxDdb
									? 'preset-filled-success-500'
									: 'preset-filled-secondary-500'}"
							>
								{aircraft.fromAdsbxDdb ? 'In ADSB Exchange DB' : 'Not in ADSB Exchange DB'}
							</span>
							{#if aircraft.aircraftTypeOgn}
								<span class="badge {getAircraftTypeColor(aircraft.aircraftTypeOgn)} text-xs">
									{getAircraftTypeOgnDescription(aircraft.aircraftTypeOgn)}
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
								<span class="preset-tonal-tertiary-500 badge flex items-center gap-1.5 text-xs">
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
						</div>
					</div>
				</div>
			</div>

			<!-- Aircraft Images Section -->
			{#if !loadingImages && aircraftImages.length > 0}
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Plane class="h-6 w-6" />
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

				<!-- Aircraft Model Details -->
				{#if aircraftModel}
					<div class="space-y-4 card p-6">
						<h2 class="flex items-center gap-2 h2">
							<Plane class="h-6 w-6" />
							Aircraft Model Details
						</h2>

						<div class="space-y-4">
							<!-- Primary Information -->
							<div
								class="border-surface-300-600-token bg-surface-50-900-token rounded-lg border p-4"
							>
								<h3
									class="text-surface-600-300-token mb-3 text-xs font-semibold tracking-wide uppercase"
								>
									Basic Information
								</h3>
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
								</dl>
							</div>

							<!-- Technical Specifications -->
							{#if aircraftModel.engineType || (aircraftModel.numberOfEngines !== null && aircraftModel.numberOfEngines !== undefined) || aircraftModel.builderCertification || aircraftModel.weightClass}
								<div
									class="border-surface-300-600-token bg-surface-50-900-token rounded-lg border p-4"
								>
									<h3
										class="text-surface-600-300-token mb-3 text-xs font-semibold tracking-wide uppercase"
									>
										Technical Specifications
									</h3>
									<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
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
												<dt class="text-surface-600-300-token mb-1 font-medium">
													Builder Certification
												</dt>
												<dd>{formatTitleCase(aircraftModel.builderCertification)}</dd>
											</div>
										{/if}
										{#if aircraftModel.weightClass}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Weight Class</dt>
												<dd>{formatTitleCase(aircraftModel.weightClass)}</dd>
											</div>
										{/if}
									</dl>
								</div>
							{/if}

							<!-- Capacity & Performance -->
							{#if aircraftModel.numberOfSeats || (aircraftModel.cruisingSpeed && aircraftModel.cruisingSpeed > 0)}
								<div
									class="border-surface-300-600-token bg-surface-50-900-token rounded-lg border p-4"
								>
									<h3
										class="text-surface-600-300-token mb-3 text-xs font-semibold tracking-wide uppercase"
									>
										Capacity & Performance
									</h3>
									<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
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
									</dl>
								</div>
							{/if}

							<!-- Type Certificate Information -->
							{#if aircraftModel.typeCertificateDataSheet || aircraftModel.typeCertificateDataHolder}
								<div
									class="border-surface-300-600-token bg-surface-50-900-token rounded-lg border p-4"
								>
									<h3
										class="text-surface-600-300-token mb-3 text-xs font-semibold tracking-wide uppercase"
									>
										Type Certificate
									</h3>
									<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
										{#if aircraftModel.typeCertificateDataSheet}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Data Sheet</dt>
												<dd>{aircraftModel.typeCertificateDataSheet}</dd>
											</div>
										{/if}
										{#if aircraftModel.typeCertificateDataHolder}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Data Holder</dt>
												<dd>{aircraftModel.typeCertificateDataHolder}</dd>
											</div>
										{/if}
									</dl>
								</div>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<!-- Flights Section -->
			<div class="space-y-4 card p-6">
				<h2 class="flex items-center gap-2 h2">
					<Plane class="h-6 w-6" />
					Flight History
				</h2>

				{#if loadingFlights}
					<div class="flex items-center justify-center py-8">
						<Progress class="h-6 w-6" />
						<span class="ml-2">Loading flight history...</span>
					</div>
				{:else if flights.length === 0}
					<div class="text-surface-600-300-token py-8 text-center">
						<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
						<p>No flights found for this aircraft</p>
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

			<!-- Aircraft Registration -->
			<div class="space-y-4 card p-6">
				<h2 class="flex items-center gap-2 h2">
					<Plane class="h-6 w-6" />
					Aircraft Registration
				</h2>

				{#if aircraftRegistration}
					<div class="space-y-3">
						<!-- Registration Number -->
						<div class="flex items-start gap-3">
							<Info class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Registration Number</p>
								<p class="font-mono font-semibold">
									{aircraft.registration || aircraftRegistration.registrationNumber || 'Unknown'}
								</p>
							</div>
						</div>

						<!-- Serial Number -->
						<div class="flex items-start gap-3">
							<Info class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Serial Number</p>
								<p class="font-mono">{aircraftRegistration.serialNumber}</p>
							</div>
						</div>

						<!-- Manufacturer/Model/Series Codes -->
						{#if aircraftRegistration.manufacturerCode || aircraftRegistration.modelCode || aircraftRegistration.seriesCode}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Manufacturer/Model/Series</p>
									<p class="font-mono">
										{[
											aircraftRegistration.manufacturerCode,
											aircraftRegistration.modelCode,
											aircraftRegistration.seriesCode
										]
											.filter(Boolean)
											.join('-')}
									</p>
								</div>
							</div>
						{/if}

						<!-- Engine Codes -->
						{#if aircraftRegistration.engineManufacturerCode || aircraftRegistration.engineModelCode}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Engine Manufacturer/Model</p>
									<p class="font-mono">
										{[
											aircraftRegistration.engineManufacturerCode,
											aircraftRegistration.engineModelCode
										]
											.filter(Boolean)
											.join('-')}
									</p>
								</div>
							</div>
						{/if}

						<!-- Year Manufactured -->
						{#if aircraftRegistration.yearManufactured}
							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Year Manufactured</p>
									<p>{aircraftRegistration.yearManufactured}</p>
								</div>
							</div>
						{/if}

						<!-- Aircraft Type -->
						{#if aircraftRegistration.aircraftType}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Aircraft Type</p>
									<p>{aircraftRegistration.aircraftType}</p>
								</div>
							</div>
						{/if}

						<!-- Engine Type -->
						{#if aircraftRegistration.engineType}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Engine Type</p>
									<p>{aircraftRegistration.engineType}</p>
								</div>
							</div>
						{/if}

						<!-- Registrant Name -->
						{#if aircraftRegistration.registrantName}
							<div class="flex items-start gap-3">
								<User class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Owner</p>
									<p>{aircraftRegistration.registrantName}</p>
								</div>
							</div>
						{/if}

						<!-- Registrant Type -->
						{#if aircraftRegistration.registrantType}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Registrant Type</p>
									<p>{aircraftRegistration.registrantType}</p>
								</div>
							</div>
						{/if}

						<!-- Status Code -->
						{#if aircraftRegistration.statusCode}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Status</p>
									<p>
										{getStatusCodeDescription(aircraftRegistration.statusCode)}
										<span class="ml-1 text-xs text-surface-500"
											>({aircraftRegistration.statusCode})</span
										>
									</p>
								</div>
							</div>
						{/if}

						<!-- Transponder Code -->
						{#if aircraftRegistration.transponderCode}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Transponder Code</p>
									<p class="font-mono">
										{formatTransponderCode(aircraftRegistration.transponderCode)}
									</p>
								</div>
							</div>
						{/if}

						<!-- Airworthiness Class -->
						{#if aircraftRegistration.airworthinessClass}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Airworthiness Class</p>
									<p>{aircraftRegistration.airworthinessClass}</p>
								</div>
							</div>
						{/if}

						<!-- Airworthiness Date -->
						{#if aircraftRegistration.airworthinessDate}
							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Airworthiness Date</p>
									<p>{dayjs(aircraftRegistration.airworthinessDate).format('YYYY-MM-DD')}</p>
								</div>
							</div>
						{/if}

						<!-- Certificate Issue Date -->
						{#if aircraftRegistration.certificateIssueDate}
							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Certificate Issue Date</p>
									<p>
										{dayjs(aircraftRegistration.certificateIssueDate).format('YYYY-MM-DD')}
									</p>
								</div>
							</div>
						{/if}

						<!-- Expiration Date -->
						{#if aircraftRegistration.expirationDate}
							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Expiration Date</p>
									<p>{dayjs(aircraftRegistration.expirationDate).format('YYYY-MM-DD')}</p>
								</div>
							</div>
						{/if}

						<!-- Kit Manufacturer/Model -->
						{#if aircraftRegistration.kitManufacturerName || aircraftRegistration.kitModelName}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Kit</p>
									<p>
										{[aircraftRegistration.kitManufacturerName, aircraftRegistration.kitModelName]
											.filter(Boolean)
											.join(' ')}
									</p>
								</div>
							</div>
						{/if}

						<!-- Light Sport Type -->
						{#if aircraftRegistration.lightSportType}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Light Sport Type</p>
									<p>{aircraftRegistration.lightSportType}</p>
								</div>
							</div>
						{/if}

						<!-- Other Names -->
						{#if aircraftRegistration.otherNames && aircraftRegistration.otherNames.length > 0}
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Other Names</p>
									<p>{aircraftRegistration.otherNames.join(', ')}</p>
								</div>
							</div>
						{/if}
					</div>
				{:else}
					<div class="text-surface-600-300-token py-4 text-center text-xs">
						<p>
							No registration data found
							<br />
							<i>(Data currently only available for USA-registered aircraft)</i>
						</p>
					</div>
				{/if}
			</div>

			<!-- Position Fixes Section -->
			<div class="space-y-4 card p-6">
				<h2 class="flex items-center gap-2 h2">
					<Activity class="h-6 w-6" />
					Recent Position Fixes
				</h2>

				<FixesList
					{fixes}
					loading={loadingFixes}
					showHideInactive={true}
					showRaw={true}
					emptyMessage="No position fixes found in the last 24 hours"
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
	{/if}
</div>
