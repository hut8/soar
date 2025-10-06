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
		MoveUpRight
	} from '@lucide/svelte';
	import type { PageData } from './$types';

	let { data }: { data: PageData } = $props();

	const GOOGLE_MAPS_API_KEY = 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

	let mapContainer: HTMLElement;
	let map: google.maps.Map;
	let flightPath: google.maps.Polyline | null = null;

	// Pagination state
	let currentPage = $state(1);
	let pageSize = 50;

	const totalPages = $derived(Math.ceil(data.fixes.length / pageSize));
	const paginatedFixes = $derived(
		data.fixes.slice((currentPage - 1) * pageSize, currentPage * pageSize)
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

	// Format date/time
	function formatDateTime(dateString: string | undefined): string {
		if (!dateString) return 'N/A';
		const date = new Date(dateString);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
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
		if (data.fixes.length === 0) return;

		try {
			const loader = new Loader({
				apiKey: GOOGLE_MAPS_API_KEY,
				version: 'weekly'
			});

			await loader.importLibrary('maps');
			await loader.importLibrary('marker');

			// Calculate center and bounds
			const bounds = new google.maps.LatLngBounds();
			data.fixes.forEach((fix) => {
				bounds.extend({ lat: fix.latitude, lng: fix.longitude });
			});

			const center = bounds.getCenter();

			// Create map
			map = new google.maps.Map(mapContainer, {
				center: { lat: center.lat(), lng: center.lng() },
				zoom: 12,
				mapId: 'FLIGHT_MAP'
			});

			// Fit bounds
			map.fitBounds(bounds);

			// Create flight path
			const pathCoordinates = data.fixes.map((fix) => ({
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

			// Add takeoff marker (green)
			if (data.fixes.length > 0) {
				const first = data.fixes[0];
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

			// Add landing marker (red) if flight is complete
			if (data.flight.landing_time && data.fixes.length > 0) {
				const last = data.fixes[data.fixes.length - 1];
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

<div class="container mx-auto space-y-6 p-4">
	<!-- Flight Header -->
	<div class="card p-6">
		<div class="mb-4 flex items-center justify-between">
			<h1 class="flex items-center gap-2 h1">
				<Plane class="h-8 w-8" />
				Flight {data.flight.device_address}
			</h1>
			<button
				onclick={downloadKML}
				class="variant-filled-primary btn flex items-center gap-2"
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
					<div class="text-surface-600-300-token text-sm">Takeoff</div>
					<div class="font-semibold">{formatDateTime(data.flight.takeoff_time)}</div>
					{#if data.flight.takeoff_runway_ident}
						<div class="text-surface-600-300-token text-sm">
							Runway {data.flight.takeoff_runway_ident}
						</div>
					{/if}
				</div>
			</div>

			<!-- Landing Time -->
			<div class="flex items-start gap-3">
				<Clock class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Landing</div>
					<div class="font-semibold">
						{data.flight.landing_time ? formatDateTime(data.flight.landing_time) : 'In Progress'}
					</div>
					{#if data.flight.landing_runway_ident}
						<div class="text-surface-600-300-token text-sm">
							Runway {data.flight.landing_runway_ident}
						</div>
					{/if}
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
			{#if data.flight.departure_airport}
				<div class="flex items-start gap-3">
					<MapPin class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Departure</div>
						<div class="font-semibold">{data.flight.departure_airport}</div>
					</div>
				</div>
			{/if}

			<!-- Arrival Airport -->
			{#if data.flight.arrival_airport}
				<div class="flex items-start gap-3">
					<MapPin class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Arrival</div>
						<div class="font-semibold">{data.flight.arrival_airport}</div>
					</div>
				</div>
			{/if}

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

	<!-- Map -->
	{#if data.fixes.length > 0}
		<div class="card p-6">
			<h2 class="mb-4 h2">Flight Track</h2>
			<div bind:this={mapContainer} class="h-96 w-full rounded-lg"></div>
		</div>
	{/if}

	<!-- Fixes Table -->
	<div class="card p-6" id="fixes-table">
		<h2 class="mb-4 h2">Position Fixes ({data.fixesCount})</h2>

		{#if data.fixes.length === 0}
			<div class="text-surface-600-300-token py-8 text-center">
				<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<p>No position data available for this flight.</p>
			</div>
		{:else}
			<div class="overflow-x-auto">
				<table class="table">
					<thead>
						<tr>
							<th>Time</th>
							<th>Latitude</th>
							<th>Longitude</th>
							<th>Altitude</th>
							<th>AGL</th>
							<th>Speed</th>
							<th>Track</th>
							<th>Climb</th>
						</tr>
					</thead>
					<tbody>
						{#each paginatedFixes as fix (fix.id)}
							<tr>
								<td>{new Date(fix.timestamp).toLocaleTimeString()}</td>
								<td>{fix.latitude.toFixed(6)}</td>
								<td>{fix.longitude.toFixed(6)}</td>
								<td>{formatAltitude(fix.altitude_feet)}</td>
								<td>{formatAltitude(fix.altitude_agl_feet)}</td>
								<td>{fix.ground_speed_knots ? `${fix.ground_speed_knots.toFixed(1)} kt` : 'N/A'}</td
								>
								<td>{fix.track_degrees ? `${fix.track_degrees.toFixed(0)}°` : 'N/A'}</td>
								<td>{fix.climb_fpm ? `${fix.climb_fpm.toFixed(0)} fpm` : 'N/A'}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<!-- Pagination -->
			{#if totalPages > 1}
				<div class="mt-4 flex items-center justify-between">
					<div class="text-surface-600-300-token text-sm">
						Page {currentPage} of {totalPages}
					</div>
					<div class="flex gap-2">
						<button
							onclick={() => goToPage(currentPage - 1)}
							disabled={currentPage === 1}
							class="variant-filled-surface btn btn-sm"
							type="button"
						>
							Previous
						</button>
						<button
							onclick={() => goToPage(currentPage + 1)}
							disabled={currentPage === totalPages}
							class="variant-filled-surface btn btn-sm"
							type="button"
						>
							Next
						</button>
					</div>
				</div>
			{/if}
		{/if}
	</div>
</div>
