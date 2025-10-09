<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount } from 'svelte';
	import { Loader } from '@googlemaps/js-api-loader';
	import {
		Download,
		Plane,
		MapPin,
		Clock,
		Gauge,
		TrendingUp,
		Route,
		MoveUpRight,
		MapPinMinus,
		ChevronsLeft,
		ChevronLeft,
		ChevronRight,
		ChevronsRight
	} from '@lucide/svelte';
	import type { PageData } from './$types';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';

	dayjs.extend(relativeTime);

	let { data }: { data: PageData } = $props();

	const GOOGLE_MAPS_API_KEY = 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

	let mapContainer = $state<HTMLElement>();
	let map = $state<google.maps.Map>();
	let flightPath = $state<google.maps.Polyline | null>(null);

	// Pagination state
	let currentPage = $state(1);
	let pageSize = 50;

	// Display options
	let showRawData = $state(false);
	let useRelativeTimes = $state(false);

	// Reverse fixes to show chronologically (earliest first, landing last)
	const reversedFixes = $derived([...data.fixes].reverse());
	const totalPages = $derived(Math.ceil(reversedFixes.length / pageSize));
	const paginatedFixes = $derived(
		reversedFixes.slice((currentPage - 1) * pageSize, currentPage * pageSize)
	);

	// Calculate flight duration
	const duration = $derived(() => {
		if (!data.flight.takeoff_time || !data.flight.landing_time) {
			return null;
		}
		const start = new Date(data.flight.takeoff_time);
		const end = new Date(data.flight.landing_time);
		const diffMs = end.getTime() - start.getTime();
		const hours = Math.floor(diffMs / (1000 * 60 * 60));
		const minutes = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	});

	// Calculate fixes per second rate
	const fixesPerSecond = $derived(() => {
		if (!data.flight.takeoff_time || !data.flight.landing_time || data.fixesCount === 0) {
			return null;
		}
		const start = new Date(data.flight.takeoff_time);
		const end = new Date(data.flight.landing_time);
		const durationSeconds = (end.getTime() - start.getTime()) / 1000;
		if (durationSeconds <= 0) return null;
		return (data.fixesCount / durationSeconds).toFixed(2);
	});

	// Check if this is an outlanding (flight complete with known departure but no arrival airport)
	const isOutlanding = $derived(
		data.flight.landing_time !== null &&
			data.flight.landing_time !== undefined &&
			data.flight.departure_airport &&
			!data.flight.arrival_airport
	);

	// Format date/time with relative time and full datetime
	function formatDateTime(dateString: string | undefined): string {
		if (!dateString) return 'N/A';
		const date = dayjs(dateString);
		const relative = date.fromNow();
		const fullDate = date.format('MMM D, YYYY h:mm A');
		return `${relative} (${fullDate})`;
	}

	// Format date/time - mobile only shows relative
	function formatDateTimeMobile(dateString: string | undefined): string {
		if (!dateString) return 'N/A';
		return dayjs(dateString).fromNow();
	}

	// Format timestamp for fixes table (relative or absolute based on checkbox)
	function formatFixTime(timestamp: string): string {
		if (useRelativeTimes) {
			return dayjs(timestamp).fromNow();
		}
		return new Date(timestamp).toLocaleTimeString();
	}

	// Format altitude
	function formatAltitude(feet: number | undefined): string {
		if (feet === undefined || feet === null) return 'N/A';
		return `${feet.toLocaleString()} ft`;
	}

	// Format distance in meters to nautical miles and kilometers
	function formatDistance(meters: number | undefined): string {
		if (meters === undefined || meters === null) return 'N/A';
		// Convert meters to nautical miles (1 nm = 1852 meters)
		const nm = meters / 1852;
		// Convert meters to kilometers
		const km = meters / 1000;

		if (nm >= 1) {
			return `${nm.toFixed(2)} nm (${km.toFixed(2)} km)`;
		} else {
			return `${km.toFixed(2)} km`;
		}
	}

	// Initialize map
	onMount(async () => {
		if (data.fixes.length === 0 || !mapContainer) return;

		try {
			const loader = new Loader({
				apiKey: GOOGLE_MAPS_API_KEY,
				version: 'weekly'
			});

			await loader.importLibrary('maps');
			await loader.importLibrary('marker');

			// Use reversed fixes for chronological order (earliest to latest)
			const fixesInOrder = [...data.fixes].reverse();

			// Calculate center and bounds
			const bounds = new google.maps.LatLngBounds();
			fixesInOrder.forEach((fix) => {
				bounds.extend({ lat: fix.latitude, lng: fix.longitude });
			});

			const center = bounds.getCenter();

			// Create map with satellite view by default
			map = new google.maps.Map(mapContainer, {
				center: { lat: center.lat(), lng: center.lng() },
				zoom: 12,
				mapId: 'FLIGHT_MAP',
				mapTypeId: google.maps.MapTypeId.SATELLITE
			});

			// Fit bounds
			map.fitBounds(bounds);

			// Create flight path (in chronological order)
			const pathCoordinates = fixesInOrder.map((fix) => ({
				lat: fix.latitude,
				lng: fix.longitude
			}));

			flightPath = new google.maps.Polyline({
				path: pathCoordinates,
				geodesic: true,
				strokeColor: '#FF0000',
				strokeOpacity: 1.0,
				strokeWeight: 3
			});

			flightPath.setMap(map);

			// Add takeoff marker (green) - first fix chronologically
			if (fixesInOrder.length > 0) {
				const first = fixesInOrder[0];
				const takeoffPin = document.createElement('div');
				takeoffPin.innerHTML = `
					<div style="background-color: #10b981; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;"></div>
				`;

				new google.maps.marker.AdvancedMarkerElement({
					map,
					position: { lat: first.latitude, lng: first.longitude },
					content: takeoffPin,
					title: 'Takeoff'
				});
			}

			// Add landing marker (red) if flight is complete - last fix chronologically
			if (data.flight.landing_time && fixesInOrder.length > 0) {
				const last = fixesInOrder[fixesInOrder.length - 1];
				const landingPin = document.createElement('div');
				landingPin.innerHTML = `
					<div style="background-color: #ef4444; width: 20px; height: 20px; border-radius: 50%; border: 2px solid white;"></div>
				`;

				new google.maps.marker.AdvancedMarkerElement({
					map,
					position: { lat: last.latitude, lng: last.longitude },
					content: landingPin,
					title: 'Landing'
				});
			}
		} catch (error) {
			console.error('Failed to load Google Maps:', error);
		}
	});

	// KML download
	function downloadKML() {
		window.open(`/data/flights/${data.flight.id}/kml`, '_blank');
	}

	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			currentPage = page;
			// Scroll to top of fixes table
			document.getElementById('fixes-table')?.scrollIntoView({ behavior: 'smooth' });
		}
	}
</script>

<svelte:head>
	<title>Flight {data.flight.device_address} | SOAR</title>
</svelte:head>

<div class="container mx-auto space-y-4 p-4">
	<!-- Flight Header -->
	<div class="card p-6">
		<div class="mb-4 flex items-center justify-between">
			<div class="flex items-center gap-4">
				<h1 class="flex items-center gap-2 h1">
					<Plane class="h-8 w-8" />
					Flight {data.flight.device_address}
				</h1>
				{#if isOutlanding}
					<span
						class="chip flex items-center gap-2 preset-filled-warning-500 text-base font-semibold"
					>
						<MapPinMinus class="h-5 w-5" />
						Outlanding
					</span>
				{/if}
			</div>
			<button
				onclick={downloadKML}
				class="btn flex items-center gap-2 preset-filled-primary-500"
				type="button"
			>
				<Download class="h-4 w-4" />
				Download KML
			</button>
		</div>

		<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
			<!-- Takeoff Time -->
			<div class="flex items-start gap-3">
				<Clock class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Takeoff Time</div>
					<div class="font-semibold">
						<!-- Mobile: relative time only -->
						<span class="md:hidden">{formatDateTimeMobile(data.flight.takeoff_time)}</span>
						<!-- Desktop: relative time with full datetime -->
						<span class="hidden md:inline">{formatDateTime(data.flight.takeoff_time)}</span>
					</div>
				</div>
			</div>

			<!-- Landing Time -->
			<div class="flex items-start gap-3">
				<Clock class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Landing Time</div>
					<div class="font-semibold">
						{#if data.flight.landing_time}
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(data.flight.landing_time)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(data.flight.landing_time)}</span>
						{:else}
							In Progress
						{/if}
					</div>
				</div>
			</div>

			<!-- Duration -->
			{#if duration()}
				<div class="flex items-start gap-3">
					<Gauge class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Duration</div>
						<div class="font-semibold">{duration()}</div>
					</div>
				</div>
			{/if}

			<!-- Total Distance -->
			{#if data.flight.total_distance_meters}
				<div class="flex items-start gap-3">
					<Route class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Total Distance</div>
						<div class="font-semibold">{formatDistance(data.flight.total_distance_meters)}</div>
					</div>
				</div>
			{/if}

			<!-- Maximum Displacement -->
			{#if data.flight.maximum_displacement_meters}
				<div class="flex items-start gap-3">
					<MoveUpRight class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Max Displacement</div>
						<div class="font-semibold">
							{formatDistance(data.flight.maximum_displacement_meters)}
						</div>
						<div class="text-surface-600-300-token text-sm">
							from {data.flight.departure_airport}
						</div>
					</div>
				</div>
			{/if}

			<!-- Departure Airport -->
			<div class="flex items-start gap-3">
				<MapPin class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Departure</div>
					<div class="font-semibold">
						{#if data.flight.departure_airport && data.flight.departure_airport_id}
							<a href="/airports/{data.flight.departure_airport_id}" class="anchor">
								{data.flight.departure_airport}
							</a>
						{:else if data.flight.departure_airport}
							{data.flight.departure_airport}
						{:else}
							Unknown
						{/if}
					</div>
					{#if data.flight.takeoff_runway_ident}
						<div class="text-surface-600-300-token text-sm">
							Runway {data.flight.takeoff_runway_ident}
						</div>
					{:else if data.flight.departure_airport}
						<div class="text-surface-600-300-token text-sm">Runway Unknown</div>
					{/if}
				</div>
			</div>

			<!-- Arrival Airport -->
			<div class="flex items-start gap-3">
				<MapPin class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Arrival</div>
					<div class="font-semibold">
						{#if data.flight.landing_time}
							{#if data.flight.arrival_airport && data.flight.arrival_airport_id}
								<a href="/airports/{data.flight.arrival_airport_id}" class="anchor">
									{data.flight.arrival_airport}
								</a>
							{:else if data.flight.arrival_airport}
								{data.flight.arrival_airport}
							{:else}
								Unknown
							{/if}
						{:else}
							In Progress
						{/if}
					</div>
					{#if data.flight.landing_time && data.flight.arrival_airport}
						{#if data.flight.landing_runway_ident}
							<div class="text-surface-600-300-token text-sm">
								Runway {data.flight.landing_runway_ident}
							</div>
						{:else}
							<div class="text-surface-600-300-token text-sm">Runway Unknown</div>
						{/if}
					{/if}
				</div>
			</div>

			<!-- Tow Aircraft -->
			{#if data.flight.tow_aircraft_id}
				<div class="flex items-start gap-3">
					<TrendingUp class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Tow Aircraft</div>
						<div class="font-semibold">{data.flight.tow_aircraft_id}</div>
						{#if data.flight.tow_release_height_msl}
							<div class="text-surface-600-300-token text-sm">
								Release: {data.flight.tow_release_height_msl} ft MSL
							</div>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Aircraft Information -->
	{#if data.device}
		<div class="card p-4">
			<h2 class="mb-3 h3">Aircraft Information</h2>
			<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
				<div>
					<div class="text-surface-600-300-token text-sm">Registration</div>
					<div class="font-mono text-sm font-semibold">
						{data.device.registration || 'Unknown'}
						{#if data.device.competition_number}
							<span class="text-surface-500-400-token ml-1">({data.device.competition_number})</span
							>
						{/if}
					</div>
				</div>
				<div>
					<div class="text-surface-600-300-token text-sm">Model</div>
					<div class="text-sm font-semibold">
						{data.device.aircraft_model || 'Unknown'}
					</div>
				</div>
				<div>
					<div class="text-surface-600-300-token text-sm">Aircraft Type</div>
					<div class="text-sm font-semibold">
						{#if data.device.aircraft_type_ogn}
							{data.device.aircraft_type_ogn.replace(/_/g, ' ')}
						{:else}
							Unknown
						{/if}
					</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Map -->
	{#if data.fixes.length > 0}
		<div class="card p-4">
			<h2 class="mb-3 h3">Flight Track</h2>
			<div bind:this={mapContainer} class="h-96 w-full rounded-lg"></div>
		</div>
	{/if}

	<!-- Fixes Table -->
	<div class="card p-6" id="fixes-table">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="h2">
				Position Fixes ({data.fixesCount})
				{#if fixesPerSecond()}
					<span class="text-surface-600-300-token ml-2 text-lg">
						({fixesPerSecond()} fixes/sec)
					</span>
				{/if}
			</h2>
			<div class="flex gap-4">
				<label class="flex cursor-pointer items-center gap-2">
					<input type="checkbox" class="checkbox" bind:checked={showRawData} />
					<span class="text-sm">Display Raw</span>
				</label>
				<label class="flex cursor-pointer items-center gap-2">
					<input type="checkbox" class="checkbox" bind:checked={useRelativeTimes} />
					<span class="text-sm">Relative Times</span>
				</label>
			</div>
		</div>

		{#if data.fixes.length === 0}
			<div class="text-surface-600-300-token py-8 text-center">
				<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<p>No position data available for this flight.</p>
			</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="w-full table-auto">
					<thead class="bg-surface-100-800-token border-surface-300-600-token border-b">
						<tr>
							<th class="px-3 py-2 text-left text-sm font-medium">Time</th>
							<th class="px-3 py-2 text-left text-sm font-medium">Location</th>
							<th class="px-3 py-2 text-left text-sm font-medium">Altitude</th>
							<th class="px-3 py-2 text-left text-sm font-medium">AGL</th>
							<th class="px-3 py-2 text-left text-sm font-medium">Speed</th>
							<th class="px-3 py-2 text-left text-sm font-medium">Track</th>
							<th class="px-3 py-2 text-left text-sm font-medium">Climb</th>
						</tr>
					</thead>
					<tbody>
						{#each paginatedFixes as fix, index (fix.id)}
							<tr
								class="border-b border-gray-200 hover:bg-gray-100 dark:border-gray-700 dark:hover:bg-gray-800 {index %
									2 ===
								0
									? 'bg-gray-50 dark:bg-gray-900'
									: ''}"
							>
								<td class="px-3 py-2 text-sm">{formatFixTime(fix.timestamp)}</td>
								<td class="px-3 py-2 text-sm">
									<a
										href="https://www.google.com/maps?q={fix.latitude},{fix.longitude}"
										target="_blank"
										rel="noopener noreferrer"
										class="anchor text-primary-500 hover:text-primary-600"
									>
										{fix.latitude.toFixed(6)}, {fix.longitude.toFixed(6)}
									</a>
								</td>
								<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitude_msl_feet)}</td>
								<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitude_agl_feet)}</td>
								<td class="px-3 py-2 text-sm"
									>{fix.ground_speed_knots ? `${fix.ground_speed_knots.toFixed(1)} kt` : 'N/A'}</td
								>
								<td class="px-3 py-2 text-sm"
									>{fix.track_degrees ? `${fix.track_degrees.toFixed(0)}°` : 'N/A'}</td
								>
								<td class="px-3 py-2 text-sm"
									>{fix.climb_fpm ? `${fix.climb_fpm.toFixed(0)} fpm` : 'N/A'}</td
								>
							</tr>
							{#if showRawData}
								<tr
									class="border-b border-gray-200 dark:border-gray-700 {index % 2 === 0
										? 'bg-gray-100 dark:bg-gray-800'
										: ''}"
								>
									<td colspan="7" class="px-3 py-2 font-mono text-sm">
										{fix.raw_packet}
									</td>
								</tr>
							{/if}
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Pagination -->
			{#if totalPages > 1}
				<div class="mt-4 flex flex-col items-center gap-3 sm:flex-row sm:justify-between">
					<div class="text-surface-600-300-token text-sm">
						Page {currentPage} of {totalPages}
					</div>
					<div class="flex flex-wrap justify-center gap-2">
						<button
							onclick={() => goToPage(1)}
							disabled={currentPage === 1}
							class="btn preset-tonal btn-sm"
							type="button"
							title="First page (Takeoff)"
						>
							<ChevronsLeft class="h-4 w-4" />
							Takeoff
						</button>
						<button
							onclick={() => goToPage(currentPage - 1)}
							disabled={currentPage === 1}
							class="btn preset-tonal btn-sm"
							type="button"
							title="Previous page"
						>
							<ChevronLeft class="h-4 w-4" />
							Previous
						</button>
						<button
							onclick={() => goToPage(currentPage + 1)}
							disabled={currentPage === totalPages}
							class="btn preset-tonal btn-sm"
							type="button"
							title="Next page"
						>
							Next
							<ChevronRight class="h-4 w-4" />
						</button>
						<button
							onclick={() => goToPage(totalPages)}
							disabled={currentPage === totalPages}
							class="btn preset-tonal btn-sm"
							type="button"
							title="Last page (Landing)"
						>
							Landing
							<ChevronsRight class="h-4 w-4" />
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</div>
