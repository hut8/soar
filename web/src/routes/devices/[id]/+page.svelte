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
		Settings
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import {
		Device,
		type AircraftRegistration,
		type AircraftModel,
		type Fix,
		type Flight
	} from '$lib/types';
	import { formatTitleCase, formatDeviceAddress, getStatusCodeDescription } from '$lib/formatters';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';

	// Extend dayjs with relative time plugin
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

	let device: Device | null = null;
	let aircraftRegistration: AircraftRegistration | null = null;
	let aircraftModel: AircraftModel | null = null;
	let fixes: Fix[] = [];
	let flights: Flight[] = [];
	let loading = true;
	let loadingFixes = false;
	let loadingFlights = false;
	let error = '';
	let deviceId = '';
	let fixesPage = 1;
	let flightsPage = 1;
	let fixesTotalPages = 1;
	let flightsTotalPages = 1;
	let hideInactiveFixes = false;

	$: deviceId = $page.params.id || '';

	function handleFilterChange() {
		loadFixes(1);
	}

	onMount(async () => {
		if (deviceId) {
			await loadDevice();
			await loadFixes();
			await loadFlights();
		}
	});

	async function loadDevice() {
		loading = true;
		error = '';

		try {
			// Load device data
			const deviceData = await serverCall<{
				id?: string;
				address_type: string;
				address: string;
				aircraft_model: string;
				registration: string;
				cn: string;
				tracked: boolean;
				identified: boolean;
				aircraft?: AircraftRegistration | null;
				aircraftModel?: AircraftModel | null;
			}>(`/devices/${deviceId}`);
			device = Device.fromJSON(deviceData);

			// Load aircraft registration and model data in parallel
			const [registration, model] = await Promise.all([
				serverCall<AircraftRegistration>(`/devices/${deviceId}/aircraft/registration`).catch(
					() => null
				),
				serverCall<AircraftModel>(`/devices/${deviceId}/aircraft/model`).catch(() => null)
			]);

			aircraftRegistration = registration;
			aircraftModel = model;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load device: ${errorMessage}`;
			console.error('Error loading device:', err);
		} finally {
			loading = false;
		}
	}

	async function loadFixes(page: number = 1) {
		loadingFixes = true;
		try {
			const activeParam = hideInactiveFixes ? '&active=true' : '';
			const response = await serverCall<FixesResponse>(
				`/devices/${deviceId}/fixes?page=${page}&per_page=50${activeParam}`
			);
			fixes = response.fixes;
			fixesPage = response.page;
			fixesTotalPages = response.total_pages;
		} catch (err) {
			console.error('Failed to load fixes:', err);
		} finally {
			loadingFixes = false;
		}
	}

	async function loadFlights(page: number = 1) {
		loadingFlights = true;
		try {
			const response = await serverCall<FlightsResponse>(
				`/devices/${deviceId}/flights?page=${page}&per_page=100`
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

	function formatDate(dateString: string): string {
		const date = new Date(dateString);
		const year = date.getFullYear();
		const month = String(date.getMonth() + 1).padStart(2, '0');
		const day = String(date.getDate()).padStart(2, '0');
		const hours = String(date.getHours()).padStart(2, '0');
		const minutes = String(date.getMinutes()).padStart(2, '0');
		const seconds = String(date.getSeconds()).padStart(2, '0');
		return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
	}

	function formatRelativeTime(dateString: string): string {
		return dayjs(dateString).fromNow();
	}

	function formatAltitude(altitude_feet: number | undefined): string {
		if (altitude_feet === undefined || altitude_feet === null) return 'Unknown';
		return `${altitude_feet.toLocaleString()} ft`;
	}

	function formatSpeed(speed_knots: number | undefined): string {
		if (speed_knots === undefined || speed_knots === null) return 'Unknown';
		return `${Math.round(speed_knots)} kts`;
	}

	function formatTrack(track_degrees: number | undefined): string {
		if (track_degrees === undefined || track_degrees === null) return 'Unknown';
		return `${Math.round(track_degrees)}°`;
	}

	function formatCoordinates(lat: number, lng: number): string {
		const latDir = lat >= 0 ? 'N' : 'S';
		const lngDir = lng >= 0 ? 'E' : 'W';
		return `${Math.abs(lat).toFixed(4)}°${latDir}, ${Math.abs(lng).toFixed(4)}°${lngDir}`;
	}

	function getGoogleMapsUrl(lat: number, lng: number): string {
		return `https://www.google.com/maps?q=${lat},${lng}`;
	}

	function goBack() {
		goto(resolve('/devices'));
	}
</script>

<svelte:head>
	<title>{device?.registration || 'Device'} - Device Details</title>
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
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Loading device details...</span>
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
					<button class="btn preset-filled" onclick={loadDevice}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Device Details -->
	{#if !loading && !error && device}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Radio class="h-8 w-8 text-primary-500" />
							<div>
								<h1 class="h1">{device.registration}</h1>
								<p class="text-surface-600-300-token font-mono text-sm">
									Address: {formatDeviceAddress(device.address_type, device.address)}
								</p>
							</div>
						</div>

						<div class="mt-3 flex flex-wrap gap-2">
							<span
								class="badge {device.tracked
									? 'preset-filled-success-500'
									: 'preset-filled-surface-500'}"
							>
								{device.tracked ? 'Tracked' : 'Not Tracked'}
							</span>
							<span
								class="badge {device.identified
									? 'preset-filled-primary-500'
									: 'preset-filled-surface-500'}"
							>
								{device.identified ? 'Identified' : 'Unidentified'}
							</span>
							<span class="badge preset-tonal">
								{device.address_type}
							</span>
						</div>
					</div>
				</div>
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				<!-- Device Information -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Settings class="h-6 w-6" />
						Device Information
					</h2>

					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Radio class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Device Address</p>
								<p class="font-mono">
									{formatDeviceAddress(device.address_type, device.address)}
								</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<Plane class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Aircraft Model</p>
								<p>{device.aircraft_model}</p>
							</div>
						</div>

						{#if device.cn}
							<div class="flex items-start gap-3">
								<Activity class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Competition Number</p>
									<p class="font-mono">{device.cn}</p>
								</div>
							</div>
						{/if}
					</div>
				</div>

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
										{device.registration || aircraftRegistration.n_number}
									</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Transponder Code</p>
									<p class="font-mono">{aircraftRegistration.mode_s_code_hex || 'N/A'}</p>
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
						</div>
					{:else}
						<div class="text-surface-600-300-token py-8 text-center">
							<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
							<p>No aircraft registration found for {device.registration}</p>
							<p class="mt-2 text-sm">
								The device may be linked to an aircraft not in our database
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

						<div class="space-y-3">
							<div class="flex items-start gap-3">
								<Plane class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Manufacturer</p>
									<p>{aircraftModel.manufacturer_name}</p>
								</div>
							</div>

							<div class="flex items-start gap-3">
								<Plane class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Model</p>
									<p>{aircraftModel.model_name}</p>
								</div>
							</div>

							{#if aircraftModel.aircraft_type}
								<div class="flex items-start gap-3">
									<Info class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Aircraft Type</p>
										<p>{formatTitleCase(aircraftModel.aircraft_type)}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.engine_type}
								<div class="flex items-start gap-3">
									<Settings class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Engine Type</p>
										<p>{formatTitleCase(aircraftModel.engine_type)}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.aircraft_category}
								<div class="flex items-start gap-3">
									<Info class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Category</p>
										<p>{formatTitleCase(aircraftModel.aircraft_category)}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.builder_certification}
								<div class="flex items-start gap-3">
									<Info class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Builder Certification</p>
										<p>{formatTitleCase(aircraftModel.builder_certification)}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.number_of_seats}
								<div class="flex items-start gap-3">
									<User class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Seats</p>
										<p>{aircraftModel.number_of_seats}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.number_of_engines !== null && aircraftModel.number_of_engines !== undefined}
								<div class="flex items-start gap-3">
									<Settings class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Engines</p>
										<p>{aircraftModel.number_of_engines}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.weight_class}
								<div class="flex items-start gap-3">
									<Info class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Weight Class</p>
										<p>{formatTitleCase(aircraftModel.weight_class)}</p>
									</div>
								</div>
							{/if}

							{#if aircraftModel.cruising_speed && aircraftModel.cruising_speed > 0}
								<div class="flex items-start gap-3">
									<Activity class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Cruising Speed</p>
										<p>{aircraftModel.cruising_speed} kts</p>
									</div>
								</div>
							{/if}
						</div>
					</div>
				{/if}
			</div>

			<!-- Position Fixes Section -->
			<div class="space-y-4 card p-6">
				<div class="flex items-center justify-between">
					<h2 class="flex items-center gap-2 h2">
						<Activity class="h-6 w-6" />
						Recent Position Fixes (Last 24 Hours)
					</h2>
					{#if fixes.length > 0}
						<label class="flex items-center gap-2 text-sm">
							<input
								type="checkbox"
								class="checkbox"
								bind:checked={hideInactiveFixes}
								onchange={handleFilterChange}
							/>
							<span>Hide inactive fixes</span>
						</label>
					{/if}
				</div>

				{#if loadingFixes}
					<div class="flex items-center justify-center py-8">
						<ProgressRing size="w-6 h-6" />
						<span class="ml-2">Loading position fixes...</span>
					</div>
				{:else if fixes.length === 0}
					<div class="text-surface-600-300-token py-8 text-center">
						<Activity class="mx-auto mb-4 h-12 w-12 text-surface-400" />
						<p>
							{hideInactiveFixes
								? 'No active fixes found'
								: 'No position fixes found in the last 24 hours'}
						</p>
					</div>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full table-auto">
							<thead class="bg-surface-100-800-token border-surface-300-600-token border-b">
								<tr>
									<th class="px-3 py-2 text-left text-sm font-medium">Time</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Coordinates</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Altitude MSL</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Altitude AGL</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Speed</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Track</th>
								</tr>
							</thead>
							<tbody>
								{#each fixes as fix, index (fix.id)}
									<tr
										class="border-surface-200-700-token hover:bg-surface-100-800-token border-b {index %
											2 ===
										0
											? 'bg-surface-50-900-token'
											: ''} {!fix.active ? 'opacity-50' : ''}"
									>
										<td class="px-3 py-2 text-sm" title={formatDate(fix.timestamp)}>
											{formatRelativeTime(fix.timestamp)}
										</td>
										<td class="px-3 py-2 font-mono text-sm">
											<a
												href={getGoogleMapsUrl(fix.latitude, fix.longitude)}
												target="_blank"
												rel="noopener noreferrer"
												class="text-primary-500 hover:text-primary-600 hover:underline"
											>
												{formatCoordinates(fix.latitude, fix.longitude)}
											</a>
										</td>
										<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitude_feet)}</td>
										<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitude_agl_feet)}</td>
										<td class="px-3 py-2 text-sm">{formatSpeed(fix.ground_speed_knots)}</td>
										<td class="px-3 py-2 text-sm">{formatTrack(fix.track_degrees)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>

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
							</div>
						</div>
					{/if}
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
						<ProgressRing size="w-6 h-6" />
						<span class="ml-2">Loading flight history...</span>
					</div>
				{:else if flights.length === 0}
					<div class="text-surface-600-300-token py-8 text-center">
						<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
						<p>No flights found for this aircraft</p>
					</div>
				{:else}
					<div class="overflow-x-auto">
						<table class="w-full table-auto">
							<thead class="bg-surface-100-800-token border-surface-300-600-token border-b">
								<tr>
									<th class="px-3 py-2 text-left text-sm font-medium">Takeoff</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Landing</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Duration</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Departure</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Arrival</th>
									<th class="px-3 py-2 text-left text-sm font-medium">Actions</th>
								</tr>
							</thead>
							<tbody>
								{#each flights as flight, index (flight.id)}
									<tr
										class="border-surface-200-700-token hover:bg-surface-100-800-token border-b {index %
											2 ===
										0
											? 'bg-surface-50-900-token'
											: ''}"
									>
										<td class="px-3 py-2 text-sm">
											{flight.takeoff_time ? formatDate(flight.takeoff_time) : 'Unknown'}
										</td>
										<td class="px-3 py-2 text-sm">
											{flight.landing_time ? formatDate(flight.landing_time) : 'In Progress'}
										</td>
										<td class="px-3 py-2 text-sm">
											{#if flight.takeoff_time && flight.landing_time}
												{@const start = new Date(flight.takeoff_time)}
												{@const end = new Date(flight.landing_time)}
												{@const diffMs = end.getTime() - start.getTime()}
												{@const hours = Math.floor(diffMs / (1000 * 60 * 60))}
												{@const minutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60))}
												{hours}h {minutes}m
											{:else}
												-
											{/if}
										</td>
										<td class="px-3 py-2 text-sm">{flight.departure_airport || 'Unknown'}</td>
										<td class="px-3 py-2 text-sm">{flight.arrival_airport || 'Unknown'}</td>
										<td class="px-3 py-2 text-sm">
											<a
												href="/flights/{flight.id}"
												target="_blank"
												rel="noopener noreferrer"
												class="text-primary-500 underline hover:text-primary-700"
											>
												View Flight
											</a>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>

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
		</div>
	{/if}
</div>
