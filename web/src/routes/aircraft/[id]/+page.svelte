<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Radio,
		Plane,
		User,
		Calendar,
		Info,
		Activity,
		Building2,
		Save
	} from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import type {
		Aircraft,
		AircraftRegistration,
		AircraftModel,
		Fix,
		Flight,
		Club
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

	interface FixesResponse {
		fixes: Fix[];
		total: number;
		page: number;
		per_page: number;
		total_pages: number;
	}

	interface FlightsResponse {
		flights: Flight[];
		total: number;
		page: number;
		per_page: number;
		total_pages: number;
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

	$: aircraftId = $page.params.id || '';
	$: isAdmin = $auth.user?.access_level === 'admin';
	$: userClubId = $auth.user?.club_id;

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
			const deviceData = await serverCall<Aircraft>(`/aircraft/${aircraftId}`);
			aircraft = deviceData;

			// Initialize selected club ID if aircraft has one
			if (aircraft.club_id) {
				selectedClubId = aircraft.club_id;
			}

			// Load aircraft registration and model data in parallel
			const [registration, model] = await Promise.all([
				serverCall<AircraftRegistration>(`/aircraft/${aircraftId}/aircraft/registration`).catch(
					() => null
				),
				serverCall<AircraftModel>(`/aircraft/${aircraftId}/aircraft/model`).catch(() => null)
			]);

			aircraftRegistration = registration;
			aircraftModel = model;
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
					per_page: 50,
					after,
					...(hideInactiveFixes && { active: true })
				}
			});
			fixes = response.fixes;
			fixesPage = response.page;
			fixesTotalPages = response.total_pages;
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
				`/aircraft/${aircraftId}/flights?page=${page}&per_page=100`
			);
			flights = response.flights || [];
			flightsPage = response.page || 1;
			flightsTotalPages = response.total_pages || 1;
		} catch (err) {
			console.error('Failed to load flights:', err);
			flights = [];
		} finally {
			loadingFlights = false;
		}
	}

	function goBack() {
		goto(resolve('/aircraft'));
	}

	async function loadClubs() {
		try {
			const response = await serverCall<{ clubs: Club[] }>('/clubs');
			clubs = response.clubs || [];
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
				body: JSON.stringify({ club_id: selectedClubId || null })
			});

			toaster.success({ title: 'Device club assignment updated successfully' });

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
</script>

<svelte:head>
	<title>{aircraft?.registration || 'Device'} - Device Details</title>
</svelte:head>

<div class="container mx-auto max-w-6xl space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn preset-tonal btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Devices
		</button>
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
				<h3 class="h3">Error Loading Device</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn preset-filled" onclick={loadAircraft}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Device Details -->
	{#if !loading && !error && aircraft}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Radio class="h-8 w-8 text-primary-500" />
							<div>
								<h1 class="h1">
									{aircraft.registration || 'Unknown'}
									{#if aircraft.competition_number}
										<span class="text-surface-600-300-token">({aircraft.competition_number})</span>
									{/if}
								</h1>
								{#if aircraft.aircraft_model}
									<p class="text-lg">{aircraft.aircraft_model}</p>
								{/if}
								{#if aircraft.icao_model_code}
									<p class="text-surface-600-300-token text-sm">
										ICAO Model Code: <span class="font-mono">{aircraft.icao_model_code}</span>
									</p>
								{/if}
								<p class="text-surface-600-300-token font-mono text-sm">
									Address: {formatAircraftAddress(aircraft.address_type, aircraft.address)}
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
								class="badge {aircraft.from_ddb
									? 'preset-filled-success-500'
									: 'preset-filled-secondary-500'}"
							>
								{aircraft.from_ddb ? 'From OGN DB' : 'Not in OGN DB'}
							</span>
							{#if aircraft.aircraft_type_ogn}
								<span class="badge {getAircraftTypeColor(aircraft.aircraft_type_ogn)} text-xs">
									{getAircraftTypeOgnDescription(aircraft.aircraft_type_ogn)}
								</span>
							{/if}
							{#if aircraft.tracker_device_type}
								<span class="preset-tonal-primary-500 badge text-xs">
									{aircraft.tracker_device_type}
								</span>
							{/if}
							{#if aircraft.country_code}
								{@const countryName = getCountryName(aircraft.country_code)}
								{@const flagPath = getFlagPath(aircraft.country_code)}
								<span class="preset-tonal-tertiary-500 badge flex items-center gap-1.5 text-xs">
									{#if flagPath}
										<img
											src={flagPath}
											alt="{aircraft.country_code} flag"
											class="h-3 w-4 object-cover"
										/>
									{/if}
									{countryName
										? `${countryName} (${aircraft.country_code})`
										: aircraft.country_code}
								</span>
							{/if}
						</div>
					</div>
				</div>
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
									Assign Device to Club
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

				<!-- Aircraft Registration -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Plane class="h-6 w-6" />
						Aircraft Registration
					</h2>

					{#if aircraftRegistration}
						<div class="space-y-3">
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Registration Number</p>
									<p class="font-mono font-semibold">
										{aircraft.registration || aircraftRegistration.n_number || 'Unknown'}
									</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Transponder Code</p>
									<p class="font-mono">
										{formatTransponderCode(aircraftRegistration.transponder_code)}
									</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Year Manufactured</p>
									<p>{aircraftRegistration.year_mfr}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<User class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Owner</p>
									<p>{aircraftRegistration.registrant_name}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Serial Number</p>
									<p class="font-mono">{aircraftRegistration.serial_number}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Status</p>
									<p>
										{getStatusCodeDescription(aircraftRegistration.status_code)}
										<span class="ml-1 text-xs text-surface-500"
											>({aircraftRegistration.status_code})</span
										>
									</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Certificate Issue Date</p>
									<p>{dayjs(aircraftRegistration.cert_issue_date).format('YYYY-MM-DD')}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Expiration Date</p>
									<p>{dayjs(aircraftRegistration.expiration_date).format('YYYY-MM-DD')}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Airworthiness Date</p>
									<p>{dayjs(aircraftRegistration.air_worth_date).format('YYYY-MM-DD')}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Last Action Date</p>
									<p>{dayjs(aircraftRegistration.last_action_date).format('YYYY-MM-DD')}</p>
								</div>
							</div>
						</div>
					{:else}
						<div class="text-surface-600-300-token py-8 text-center">
							<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
							<p>
								No aircraft registration found for {aircraft.registration || 'Unknown'}
								<br />
								<i>Data is currently only available for aircraft registered in the USA</i>
							</p>
						</div>
					{/if}
				</div>

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
										<dd class="font-semibold">{aircraftModel.manufacturer_name}</dd>
									</div>
									<div>
										<dt class="text-surface-600-300-token mb-1 font-medium">Model</dt>
										<dd class="font-semibold">{aircraftModel.model_name}</dd>
									</div>
									{#if aircraftModel.aircraft_type}
										<div>
											<dt class="text-surface-600-300-token mb-1 font-medium">Aircraft Type</dt>
											<dd>{formatTitleCase(aircraftModel.aircraft_type)}</dd>
										</div>
									{/if}
									{#if aircraftModel.aircraft_category}
										<div>
											<dt class="text-surface-600-300-token mb-1 font-medium">Category</dt>
											<dd>{formatTitleCase(aircraftModel.aircraft_category)}</dd>
										</div>
									{/if}
								</dl>
							</div>

							<!-- Technical Specifications -->
							{#if aircraftModel.engine_type || (aircraftModel.number_of_engines !== null && aircraftModel.number_of_engines !== undefined) || aircraftModel.builder_certification || aircraftModel.weight_class}
								<div
									class="border-surface-300-600-token bg-surface-50-900-token rounded-lg border p-4"
								>
									<h3
										class="text-surface-600-300-token mb-3 text-xs font-semibold tracking-wide uppercase"
									>
										Technical Specifications
									</h3>
									<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
										{#if aircraftModel.engine_type}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Engine Type</dt>
												<dd>{formatTitleCase(aircraftModel.engine_type)}</dd>
											</div>
										{/if}
										{#if aircraftModel.number_of_engines !== null && aircraftModel.number_of_engines !== undefined}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Engines</dt>
												<dd>{aircraftModel.number_of_engines}</dd>
											</div>
										{/if}
										{#if aircraftModel.builder_certification}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">
													Builder Certification
												</dt>
												<dd>{formatTitleCase(aircraftModel.builder_certification)}</dd>
											</div>
										{/if}
										{#if aircraftModel.weight_class}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Weight Class</dt>
												<dd>{formatTitleCase(aircraftModel.weight_class)}</dd>
											</div>
										{/if}
									</dl>
								</div>
							{/if}

							<!-- Capacity & Performance -->
							{#if aircraftModel.number_of_seats || (aircraftModel.cruising_speed && aircraftModel.cruising_speed > 0)}
								<div
									class="border-surface-300-600-token bg-surface-50-900-token rounded-lg border p-4"
								>
									<h3
										class="text-surface-600-300-token mb-3 text-xs font-semibold tracking-wide uppercase"
									>
										Capacity & Performance
									</h3>
									<dl class="grid grid-cols-2 gap-x-6 gap-y-3 text-sm">
										{#if aircraftModel.number_of_seats}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Seats</dt>
												<dd>{aircraftModel.number_of_seats}</dd>
											</div>
										{/if}
										{#if aircraftModel.cruising_speed && aircraftModel.cruising_speed > 0}
											<div>
												<dt class="text-surface-600-300-token mb-1 font-medium">Cruising Speed</dt>
												<dd>{aircraftModel.cruising_speed} kts</dd>
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
