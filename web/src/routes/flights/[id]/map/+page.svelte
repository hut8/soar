<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount, onDestroy } from 'svelte';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import { goto } from '$app/navigation';
	import Plotly from 'plotly.js-dist-min';
	import { ArrowLeft, ChevronDown, ChevronUp } from '@lucide/svelte';
	import type { PageData } from './$types';
	import type { Flight } from '$lib/types';
	import dayjs from 'dayjs';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { serverCall } from '$lib/api/server';
	import { theme } from '$lib/stores/theme';

	let { data }: { data: PageData } = $props();

	let mapContainer = $state<HTMLElement>();
	let map = $state<google.maps.Map>();
	let flightPathSegments = $state<google.maps.Polyline[]>([]);
	let altitudeChartContainer = $state<HTMLElement>();
	let altitudeInfoWindow = $state<google.maps.InfoWindow | null>(null);
	let fixMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let pollingInterval: ReturnType<typeof setInterval> | null = null;

	// Display options
	let includeNearbyFlights = $state(false);
	let isPanelCollapsed = $state(false);

	// Nearby flights data
	let nearbyFlights = $state<Flight[]>([]);
	let nearbyFlightPaths = $state<google.maps.Polyline[]>([]);
	let isLoadingNearbyFlights = $state(false);

	// Check if fixes have AGL data
	const hasAglData = $derived(data.fixes.some((f) => f.altitude_agl_feet !== null));

	// Calculate maximum altitude from fixes
	const maxAltitude = $derived(() => {
		if (data.fixes.length === 0) return null;
		const maxMsl = Math.max(...data.fixes.map((f) => f.altitude_msl_feet || 0));
		return maxMsl > 0 ? maxMsl : null;
	});

	// Calculate minimum altitude from fixes
	const minAltitude = $derived(() => {
		if (data.fixes.length === 0) return null;
		const validAltitudes = data.fixes
			.map((f) => f.altitude_msl_feet)
			.filter((alt): alt is number => alt !== null && alt !== undefined);
		if (validAltitudes.length === 0) return null;
		return Math.min(...validAltitudes);
	});

	// Helper function to calculate bearing between two points
	function calculateBearing(lat1: number, lon1: number, lat2: number, lon2: number): number {
		const φ1 = (lat1 * Math.PI) / 180;
		const φ2 = (lat2 * Math.PI) / 180;
		const Δλ = ((lon2 - lon1) * Math.PI) / 180;

		const y = Math.sin(Δλ) * Math.cos(φ2);
		const x = Math.cos(φ1) * Math.sin(φ2) - Math.sin(φ1) * Math.cos(φ2) * Math.cos(Δλ);
		const θ = Math.atan2(y, x);
		const bearing = ((θ * 180) / Math.PI + 360) % 360;
		return bearing;
	}

	// Helper function to map altitude to color (red→blue gradient)
	function altitudeToColor(altitude: number | null | undefined, min: number, max: number): string {
		if (altitude === null || altitude === undefined || max === min) {
			return '#888888'; // Gray for unknown altitude
		}

		// Normalize altitude to 0-1 range
		const normalized = (altitude - min) / (max - min);

		// Interpolate from red (low) to blue (high)
		const r = Math.round(239 - normalized * (239 - 59));
		const g = Math.round(68 + normalized * (130 - 68));
		const b = Math.round(68 + normalized * (246 - 68));

		return `rgb(${r}, ${g}, ${b})`;
	}

	// Helper function to create gradient polyline segments
	function createGradientPolylines(
		fixesInOrder: typeof data.fixes,
		targetMap: google.maps.Map
	): google.maps.Polyline[] {
		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;
		const segments: google.maps.Polyline[] = [];

		for (let i = 0; i < fixesInOrder.length - 1; i++) {
			const fix1 = fixesInOrder[i];
			const fix2 = fixesInOrder[i + 1];

			const color = altitudeToColor(fix1.altitude_msl_feet, minAlt, maxAlt);

			const segment = new google.maps.Polyline({
				path: [
					{ lat: fix1.latitude, lng: fix1.longitude },
					{ lat: fix2.latitude, lng: fix2.longitude }
				],
				geodesic: true,
				strokeColor: color,
				strokeOpacity: 1.0,
				strokeWeight: 3
			});

			segment.setMap(targetMap);
			segments.push(segment);
		}

		return segments;
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	function getPlotlyLayout(isDark: boolean): any {
		return {
			title: {
				text: 'Altitude Profile',
				font: {
					color: isDark ? '#e5e7eb' : '#111827'
				}
			},
			xaxis: {
				title: {
					text: 'Time',
					font: {
						color: isDark ? '#e5e7eb' : '#111827'
					}
				},
				type: 'date',
				color: isDark ? '#9ca3af' : '#6b7280',
				gridcolor: isDark ? '#374151' : '#e5e7eb'
			},
			yaxis: {
				title: {
					text: 'Altitude (ft)',
					font: {
						color: isDark ? '#e5e7eb' : '#111827'
					}
				},
				rangemode: 'tozero',
				color: isDark ? '#9ca3af' : '#6b7280',
				gridcolor: isDark ? '#374151' : '#e5e7eb'
			},
			hovermode: 'x unified',
			showlegend: true,
			legend: {
				x: 0.01,
				y: 0.99,
				bgcolor: isDark ? 'rgba(31, 41, 55, 0.8)' : 'rgba(255, 255, 255, 0.8)',
				font: {
					color: isDark ? '#e5e7eb' : '#111827'
				}
			},
			margin: { l: 60, r: 20, t: 40, b: 60 },
			paper_bgcolor: isDark ? '#1f2937' : '#ffffff',
			plot_bgcolor: isDark ? '#111827' : '#f9fafb'
		};
	}

	function isFlightInProgress(): boolean {
		return data.flight.state === 'active';
	}

	// Handle nearby flights toggle
	function handleNearbyFlightsToggle() {
		if (includeNearbyFlights) {
			fetchNearbyFlights();
		} else {
			// Clear nearby flights from map
			nearbyFlightPaths.forEach((path) => path.setMap(null));
			nearbyFlightPaths = [];
			nearbyFlights = [];
		}
	}

	async function fetchNearbyFlights() {
		isLoadingNearbyFlights = true;
		try {
			const flights = await serverCall<Flight[]>(`/flights/${data.flight.id}/nearby`);
			nearbyFlights = flights;

			if (map) {
				// Clear existing nearby flight paths
				nearbyFlightPaths.forEach((path) => path.setMap(null));
				nearbyFlightPaths = [];

				// Color palette for nearby flights
				const colors = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4'];

				for (let i = 0; i < nearbyFlights.length; i++) {
					const nearbyFlight = nearbyFlights[i];
					try {
						const fixesResponse = await serverCall<{
							fixes: typeof data.fixes;
							count: number;
						}>(`/flights/${nearbyFlight.id}/fixes`);

						if (fixesResponse.fixes.length > 0) {
							const fixesInOrder = [...fixesResponse.fixes].reverse();
							const pathCoordinates = fixesInOrder.map((fix) => ({
								lat: fix.latitude,
								lng: fix.longitude
							}));

							const flightPath = new google.maps.Polyline({
								path: pathCoordinates,
								geodesic: true,
								strokeColor: colors[i % colors.length],
								strokeOpacity: 0.6,
								strokeWeight: 2
							});

							flightPath.setMap(map);
							nearbyFlightPaths.push(flightPath);
						}
					} catch (err) {
						console.error(`Failed to fetch fixes for nearby flight ${nearbyFlight.id}:`, err);
					}
				}
			}
		} catch (err) {
			console.error('Failed to fetch nearby flights:', err);
		} finally {
			isLoadingNearbyFlights = false;
		}
	}

	// Poll for updates to in-progress flights
	async function pollForUpdates() {
		try {
			// Get the timestamp of the most recent fix (fixes are in DESC order, so first element is newest)
			const latestFixTimestamp = data.fixes.length > 0 ? data.fixes[0].timestamp : null;

			// Build URL with 'after' parameter if we have fixes
			const fixesUrl = latestFixTimestamp
				? `/flights/${data.flight.id}/fixes?after=${encodeURIComponent(latestFixTimestamp)}`
				: `/flights/${data.flight.id}/fixes`;

			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<{
					flight: typeof data.flight;
				}>(`/flights/${data.flight.id}`),
				serverCall<{
					fixes: typeof data.fixes;
					count: number;
				}>(fixesUrl)
			]);

			data.flight = flightResponse.flight;
			// Device doesn't change during a flight, so we don't re-fetch it

			// Append new fixes to the existing list (new fixes are in DESC order)
			if (fixesResponse.fixes.length > 0) {
				data.fixes = [...fixesResponse.fixes, ...data.fixes];
				data.fixesCount = data.fixes.length;
			}

			if (data.flight.state !== 'active') {
				stopPolling();
			}

			// Only update map/chart if we got new fixes
			if (fixesResponse.fixes.length > 0) {
				updateMapAndChart();
			}
		} catch (err) {
			console.error('Failed to poll for flight updates:', err);
		}
	}

	// Start polling for in-progress flights
	function startPolling() {
		if (isFlightInProgress() && !pollingInterval) {
			pollingInterval = setInterval(pollForUpdates, 10000);
		}
	}

	// Stop polling
	function stopPolling() {
		if (pollingInterval) {
			clearInterval(pollingInterval);
			pollingInterval = null;
		}
	}

	// Update map and altitude chart with new data
	async function updateMapAndChart() {
		if (data.fixes.length === 0) return;

		// Update map
		if (map && flightPathSegments.length > 0) {
			const fixesInOrder = [...data.fixes].reverse();

			// Clear existing flight path segments
			flightPathSegments.forEach((segment) => {
				segment.setMap(null);
			});
			flightPathSegments = [];

			// Create new gradient polyline segments
			flightPathSegments = createGradientPolylines(fixesInOrder, map);

			// Clear existing fix markers
			fixMarkers.forEach((marker) => {
				marker.map = null;
			});
			fixMarkers = [];

			// Wait for map to be ready before adding markers
			google.maps.event.addListenerOnce(map, 'idle', () => {
				addFixMarkers(fixesInOrder);
			});
		}

		// Update altitude chart
		if (altitudeChartContainer && data.fixes.length > 0) {
			try {
				const fixesInOrder = [...data.fixes].reverse();
				const timestamps = fixesInOrder.map((fix) => new Date(fix.timestamp));
				const altitudesMsl = fixesInOrder.map((fix) => fix.altitude_msl_feet || 0);

				const traces = [
					{
						x: timestamps,
						y: altitudesMsl,
						type: 'scatter' as const,
						mode: 'lines' as const,
						name: 'MSL Altitude',
						line: { color: '#3b82f6', width: 2 },
						hovertemplate: '<b>MSL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
					}
				];

				if (hasAglData) {
					const altitudesAgl = fixesInOrder.map((fix) => fix.altitude_agl_feet || 0);
					traces.push({
						x: timestamps,
						y: altitudesAgl,
						type: 'scatter' as const,
						mode: 'lines' as const,
						name: 'AGL Altitude',
						line: { color: '#10b981', width: 2 },
						hovertemplate: '<b>AGL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
					});
				}

				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const layout: any = getPlotlyLayout($theme === 'dark');
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const config: any = {
					responsive: true,
					displayModeBar: true,
					displaylogo: false,
					modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d']
				};

				await Plotly.react(altitudeChartContainer, traces, layout, config);
			} catch (error) {
				console.error('Failed to update altitude chart:', error);
			}
		}
	}

	function addFixMarkers(fixesInOrder: typeof data.fixes) {
		if (!map) return;

		const minAlt = minAltitude() ?? 0;
		const maxAlt = maxAltitude() ?? 1000;

		fixesInOrder.forEach((fix, index) => {
			// Calculate bearing to next fix
			let bearing = 0;
			if (index < fixesInOrder.length - 1) {
				const nextFix = fixesInOrder[index + 1];
				bearing = calculateBearing(
					fix.latitude,
					fix.longitude,
					nextFix.latitude,
					nextFix.longitude
				);
			} else if (index > 0) {
				const prevFix = fixesInOrder[index - 1];
				bearing = calculateBearing(
					prevFix.latitude,
					prevFix.longitude,
					fix.latitude,
					fix.longitude
				);
			}

			const color = altitudeToColor(fix.altitude_msl_feet, minAlt, maxAlt);

			// Create SVG arrow element (6x6 pixels)
			const arrowSvg = document.createElement('div');
			arrowSvg.innerHTML = `
				<svg width="6" height="6" viewBox="0 0 16 16" style="transform: rotate(${bearing}deg); filter: drop-shadow(0 0 1px rgba(0,0,0,0.5)); cursor: pointer;">
					<path d="M8 2 L14 14 L8 11 L2 14 Z" fill="${color}" stroke="rgba(0,0,0,0.3)" stroke-width="0.4"/>
				</svg>
			`;

			const marker = new google.maps.marker.AdvancedMarkerElement({
				map,
				position: { lat: fix.latitude, lng: fix.longitude },
				content: arrowSvg
			});

			marker.addListener('click', () => {
				const mslAlt = fix.altitude_msl_feet ? Math.round(fix.altitude_msl_feet) : 'N/A';
				const aglAlt = fix.altitude_agl_feet ? Math.round(fix.altitude_agl_feet) : 'N/A';
				const heading =
					fix.track_degrees !== undefined ? Math.round(fix.track_degrees) + '°' : 'N/A';
				const turnRate =
					fix.turn_rate_rot !== undefined ? fix.turn_rate_rot.toFixed(2) + ' rot/min' : 'N/A';
				const climbRate = fix.climb_fpm !== undefined ? Math.round(fix.climb_fpm) + ' fpm' : 'N/A';
				const groundSpeed =
					fix.ground_speed_knots !== undefined ? Math.round(fix.ground_speed_knots) + ' kt' : 'N/A';
				const timestamp = dayjs(fix.timestamp).format('h:mm:ss A');

				const content = `
					<div style="padding: 12px; min-width: 200px; background: white; color: #1f2937; border-radius: 8px; font-family: system-ui, -apple-system, sans-serif;">
						<div style="font-weight: 600; margin-bottom: 8px; font-size: 14px; color: #111827; border-bottom: 1px solid #e5e7eb; padding-bottom: 6px;">${timestamp}</div>
						<div style="display: flex; flex-direction: column; gap: 6px; font-size: 13px;">
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">MSL:</span>
								<span style="font-weight: 600; color: #3b82f6;">${mslAlt} ft</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">AGL:</span>
								<span style="font-weight: 600; color: #10b981;">${aglAlt} ft</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Heading:</span>
								<span style="font-weight: 500; color: #111827;">${heading}</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Turn Rate:</span>
								<span style="font-weight: 500; color: #111827;">${turnRate}</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Climb:</span>
								<span style="font-weight: 500; color: #111827;">${climbRate}</span>
							</div>
							<div style="display: flex; justify-content: space-between;">
								<span style="color: #6b7280;">Speed:</span>
								<span style="font-weight: 500; color: #111827;">${groundSpeed}</span>
							</div>
						</div>
					</div>
				`;

				altitudeInfoWindow?.setContent(content);
				altitudeInfoWindow?.setPosition({ lat: fix.latitude, lng: fix.longitude });
				altitudeInfoWindow?.open(map);
			});

			fixMarkers.push(marker);
		});

		// Add landing marker if flight is complete
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
	}

	// Initialize map
	onMount(async () => {
		if (data.fixes.length === 0 || !mapContainer) return;

		try {
			setOptions({
				key: GOOGLE_MAPS_API_KEY,
				v: 'weekly'
			});

			await importLibrary('maps');
			await importLibrary('marker');

			const fixesInOrder = [...data.fixes].reverse();

			// Calculate center and bounds
			const bounds = new google.maps.LatLngBounds();
			fixesInOrder.forEach((fix) => {
				bounds.extend({ lat: fix.latitude, lng: fix.longitude });
			});

			const center = bounds.getCenter();

			// Create map with satellite view
			map = new google.maps.Map(mapContainer, {
				center: { lat: center.lat(), lng: center.lng() },
				zoom: 12,
				mapId: 'FLIGHT_MAP',
				mapTypeId: google.maps.MapTypeId.SATELLITE
			});

			// Fit bounds
			map.fitBounds(bounds);

			// Create gradient polyline segments
			flightPathSegments = createGradientPolylines(fixesInOrder, map);

			// Create info window
			altitudeInfoWindow = new google.maps.InfoWindow();

			// Wait for map to be ready before adding markers
			google.maps.event.addListenerOnce(map, 'idle', () => {
				// Add takeoff marker (green)
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

				// Add landing marker if flight is complete
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

				// Add directional arrow markers
				addFixMarkers(fixesInOrder);
			});
		} catch (error) {
			console.error('Failed to load Google Maps:', error);
		}

		// Initialize altitude chart
		if (altitudeChartContainer && data.fixes.length > 0) {
			try {
				const fixesInOrder = [...data.fixes].reverse();
				const timestamps = fixesInOrder.map((fix) => new Date(fix.timestamp));
				const altitudesMsl = fixesInOrder.map((fix) => fix.altitude_msl_feet || 0);

				const traces = [
					{
						x: timestamps,
						y: altitudesMsl,
						type: 'scatter' as const,
						mode: 'lines' as const,
						name: 'MSL Altitude',
						line: { color: '#3b82f6', width: 2 },
						hovertemplate: '<b>MSL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
					}
				];

				if (hasAglData) {
					const altitudesAgl = fixesInOrder.map((fix) => fix.altitude_agl_feet || 0);
					traces.push({
						x: timestamps,
						y: altitudesAgl,
						type: 'scatter' as const,
						mode: 'lines' as const,
						name: 'AGL Altitude',
						line: { color: '#10b981', width: 2 },
						hovertemplate: '<b>AGL:</b> %{y:.0f} ft<br>%{x}<extra></extra>'
					});
				}

				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const layout: any = getPlotlyLayout($theme === 'dark');
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				const config: any = {
					responsive: true,
					displayModeBar: true,
					displaylogo: false,
					modeBarButtonsToRemove: ['pan2d', 'lasso2d', 'select2d']
				};

				await Plotly.newPlot(altitudeChartContainer, traces, layout, config);

				// Add hover event to highlight position on map
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				altitudeChartContainer.addEventListener('plotly_hover', (event: any) => {
					const data_event = event.detail || event;
					if (data_event.points && data_event.points.length > 0) {
						const pointIndex = data_event.points[0].pointIndex;
						if (pointIndex >= 0 && pointIndex < fixesInOrder.length) {
							const fix = fixesInOrder[pointIndex];
							const mslAlt = fix.altitude_msl_feet ? Math.round(fix.altitude_msl_feet) : 'N/A';
							const aglAlt = fix.altitude_agl_feet ? Math.round(fix.altitude_agl_feet) : 'N/A';
							const timestamp = dayjs(fix.timestamp).format('h:mm:ss A');

							const content = `
								<div style="padding: 8px; min-width: 180px;">
									<div style="font-weight: bold; margin-bottom: 6px;">${timestamp}</div>
									<div style="display: flex; flex-direction: column; gap: 4px;">
										<div><span style="color: #3b82f6; font-weight: 600;">MSL:</span> ${mslAlt} ft</div>
										<div><span style="color: #10b981; font-weight: 600;">AGL:</span> ${aglAlt} ft</div>
									</div>
								</div>
							`;

							altitudeInfoWindow?.setContent(content);
							altitudeInfoWindow?.setPosition({ lat: fix.latitude, lng: fix.longitude });
							altitudeInfoWindow?.open(map);

							map?.panTo({ lat: fix.latitude, lng: fix.longitude });
						}
					}
				});

				altitudeChartContainer.addEventListener('plotly_unhover', () => {
					altitudeInfoWindow?.close();
				});
			} catch (error) {
				console.error('Failed to create altitude chart:', error);
			}
		}

		// Start polling if flight is in progress
		startPolling();
	});

	// Update chart when theme changes
	$effect(() => {
		const currentTheme = $theme;

		if (altitudeChartContainer && Plotly && data.fixes.length > 0) {
			const isDark = currentTheme === 'dark';
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const currentData = (altitudeChartContainer as any).data || [];

			if (currentData.length > 0) {
				Plotly.react(altitudeChartContainer, currentData, getPlotlyLayout(isDark));
			}
		}
	});

	// Cleanup
	onDestroy(() => {
		stopPolling();
	});

	function goBack() {
		goto(`/flights/${data.flight.id}`);
	}

	function togglePanel() {
		isPanelCollapsed = !isPanelCollapsed;
	}
</script>

<!-- Container that fills viewport below AppBar -->
<div class="relative h-[calc(100vh-4rem)] w-full overflow-hidden">
	<!-- Map container -->
	<div
		bind:this={mapContainer}
		class="absolute inset-0 h-full w-full"
		style={isPanelCollapsed ? '' : 'height: calc(100% - 300px);'}
	></div>

	<!-- Back button (top-left) -->
	<button
		onclick={goBack}
		class="variant-filled-surface absolute top-4 left-4 z-[80] btn flex items-center gap-2 shadow-lg"
	>
		<ArrowLeft class="h-4 w-4" />
		<span>Back to Flight</span>
	</button>

	<!-- Nearby flights toggle (top-right) -->
	<div class="absolute top-4 right-4 z-[80]">
		<label
			class="bg-surface-50-900-token flex cursor-pointer items-center gap-2 rounded-lg p-3 shadow-lg"
		>
			<input
				type="checkbox"
				class="checkbox"
				bind:checked={includeNearbyFlights}
				onchange={handleNearbyFlightsToggle}
			/>
			<span class="text-sm">Nearby Flights</span>
			{#if isLoadingNearbyFlights}
				<span class="text-surface-600-300-token text-xs">(Loading...)</span>
			{/if}
		</label>
	</div>

	<!-- Bottom panel with altitude chart -->
	<div
		class="bg-surface-50-900-token absolute right-0 bottom-0 left-0 z-[80] shadow-lg transition-all duration-300"
		style={isPanelCollapsed ? 'height: 48px;' : 'height: 300px;'}
	>
		<!-- Panel header -->
		<div class="border-surface-300-600-token flex items-center justify-between border-b px-4 py-2">
			<h3 class="font-semibold">Altitude Profile</h3>
			<button onclick={togglePanel} class="variant-soft btn-icon btn-icon-sm">
				{#if isPanelCollapsed}
					<ChevronUp class="h-4 w-4" />
				{:else}
					<ChevronDown class="h-4 w-4" />
				{/if}
			</button>
		</div>

		<!-- Panel content -->
		{#if !isPanelCollapsed}
			<div class="h-[252px] w-full p-4">
				<div bind:this={altitudeChartContainer} class="h-full w-full"></div>
			</div>
		{/if}
	</div>
</div>
