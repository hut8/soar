<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount, onDestroy } from 'svelte';
	import { setOptions, importLibrary } from '@googlemaps/js-api-loader';
	import Plotly from 'plotly.js-dist-min';
	import {
		Download,
		Plane,
		PlaneTakeoff,
		PlaneLanding,
		Gauge,
		TrendingUp,
		Route,
		MoveUpRight,
		MapPinMinus,
		ChevronsLeft,
		ChevronLeft,
		ChevronRight,
		ChevronsRight,
		Info,
		ExternalLink,
		MountainSnow,
		Clock
	} from '@lucide/svelte';
	import type { PageData } from './$types';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import {
		getAircraftTypeOgnDescription,
		formatDeviceAddress,
		getAircraftTypeColor
	} from '$lib/formatters';
	import { GOOGLE_MAPS_API_KEY } from '$lib/config';
	import { serverCall } from '$lib/api/server';
	import FlightStateBadge from '$lib/components/FlightStateBadge.svelte';
	import RadarLoader from '$lib/components/RadarLoader.svelte';
	import FixesList from '$lib/components/FixesList.svelte';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';
	import { theme } from '$lib/stores/theme';

	dayjs.extend(relativeTime);

	let { data }: { data: PageData } = $props();

	let mapContainer = $state<HTMLElement>();
	let map = $state<google.maps.Map>();
	let flightPath = $state<google.maps.Polyline | null>(null);
	let altitudeChartContainer = $state<HTMLElement>();
	let altitudeInfoWindow = $state<google.maps.InfoWindow | null>(null);
	let fixMarkers: google.maps.marker.AdvancedMarkerElement[] = [];
	let pollingInterval: ReturnType<typeof setInterval> | null = null;

	// Pagination state
	let currentPage = $state(1);
	let pageSize = 50;

	// Display options
	let includeNearbyFlights = $state(false);

	// Nearby flights data - type includes aircraft_model and registration from device
	interface NearbyFlight {
		id: string;
		device_address: string;
		takeoff_time?: string;
		landing_time?: string;
		departure_airport?: string;
		arrival_airport?: string;
		aircraft_model?: string;
		registration?: string;
	}
	let nearbyFlights = $state<NearbyFlight[]>([]);
	let nearbyFlightPaths = $state<google.maps.Polyline[]>([]);
	let isLoadingNearbyFlights = $state(false);

	// Standalone nearby flights section (not tied to map)
	let standaloneNearbyFlights = $state<NearbyFlight[]>([]);
	let isLoadingStandaloneNearby = $state(false);
	let showStandaloneNearby = $state(false);

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

	// Calculate maximum altitude from fixes
	const maxAltitude = $derived(() => {
		if (data.fixes.length === 0) return null;
		const maxMsl = Math.max(...data.fixes.map((f) => f.altitude_msl_feet || 0));
		return maxMsl > 0 ? maxMsl : null;
	});

	// Calculate maximum AGL altitude from fixes
	const maxAglAltitude = $derived(() => {
		if (data.fixes.length === 0) return null;
		const maxAgl = Math.max(...data.fixes.map((f) => f.altitude_agl_feet || 0));
		return maxAgl > 0 ? maxAgl : null;
	});

	// Check if this is an outlanding (flight complete with known departure but no arrival airport)
	const isOutlanding = $derived(
		data.flight.landing_time !== null &&
			data.flight.landing_time !== undefined &&
			data.flight.departure_airport &&
			!data.flight.arrival_airport
	);

	// Check if any fix has AGL data available
	const hasAglData = $derived(
		data.fixes.some(
			(fix) =>
				fix.altitude_agl_feet !== null &&
				fix.altitude_agl_feet !== undefined &&
				fix.altitude_agl_feet > 0
		)
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

	// Format altitude
	function formatAltitude(feet: number | undefined): string {
		if (feet === undefined || feet === null) return 'N/A';
		return `${Math.round(feet)} ft`;
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

	// Check if flight is in progress
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

	// Fetch nearby flights for standalone section (no map paths)
	async function fetchStandaloneNearbyFlights() {
		isLoadingStandaloneNearby = true;
		showStandaloneNearby = true;
		try {
			const flights = await serverCall<NearbyFlight[]>(`/flights/${data.flight.id}/nearby`);
			standaloneNearbyFlights = flights;
		} catch (err) {
			console.error('Failed to fetch nearby flights:', err);
		} finally {
			isLoadingStandaloneNearby = false;
		}
	}

	// Fetch nearby flights and their fixes
	async function fetchNearbyFlights() {
		isLoadingNearbyFlights = true;
		try {
			// Fetch nearby flights
			const flights = await serverCall<NearbyFlight[]>(`/flights/${data.flight.id}/nearby`);
			nearbyFlights = flights;

			// Fetch fixes for each nearby flight and add to map
			if (map) {
				// Clear existing nearby flight paths
				nearbyFlightPaths.forEach((path) => path.setMap(null));
				nearbyFlightPaths = [];

				// Color palette for nearby flights (excluding red which is used for main flight)
				const colors = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4'];

				for (let i = 0; i < nearbyFlights.length; i++) {
					const nearbyFlight = nearbyFlights[i];
					try {
						const fixesResponse = await serverCall<{
							fixes: typeof data.fixes;
							count: number;
						}>(`/flights/${nearbyFlight.id}/fixes`);

						if (fixesResponse.fixes.length > 0) {
							// Draw flight path for this nearby flight
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
			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<{
					flight: typeof data.flight;
					device?: typeof data.device;
				}>(`/flights/${data.flight.id}`),
				serverCall<{
					fixes: typeof data.fixes;
					count: number;
				}>(`/flights/${data.flight.id}/fixes`)
			]);

			// Update flight data
			data.flight = flightResponse.flight;
			if (flightResponse.device) {
				data.device = flightResponse.device;
			}

			// Update fixes data
			data.fixes = fixesResponse.fixes;
			data.fixesCount = fixesResponse.count;

			// If flight has landed or timed out, stop polling
			if (data.flight.state !== 'active') {
				stopPolling();
			}

			// Update map and chart
			updateMapAndChart();
		} catch (err) {
			console.error('Failed to poll for flight updates:', err);
		}
	}

	// Start polling for in-progress flights
	function startPolling() {
		if (isFlightInProgress() && !pollingInterval) {
			pollingInterval = setInterval(pollForUpdates, 10000); // Poll every 10 seconds
		}
	}

	// Stop polling
	function stopPolling() {
		if (pollingInterval) {
			clearInterval(pollingInterval);
			pollingInterval = null;
		}
	}

	// Get Plotly layout configuration based on theme
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

	// Update map and altitude chart with new data
	function updateMapAndChart() {
		if (data.fixes.length === 0) return;

		// Update map
		if (map && flightPath) {
			const fixesInOrder = [...data.fixes].reverse();

			// Update flight path
			const pathCoordinates = fixesInOrder.map((fix) => ({
				lat: fix.latitude,
				lng: fix.longitude
			}));
			flightPath.setPath(pathCoordinates);

			// Clear existing fix markers
			fixMarkers.forEach((marker) => {
				marker.map = null;
			});
			fixMarkers = [];

			// Re-add fix markers
			fixesInOrder.forEach((fix) => {
				const fixDot = document.createElement('div');
				fixDot.innerHTML = `
					<div style="background-color: white; width: 6px; height: 6px; border-radius: 50%; border: 1px solid rgba(0,0,0,0.3); box-shadow: 0 0 2px rgba(0,0,0,0.5); cursor: pointer;"></div>
				`;

				const marker = new google.maps.marker.AdvancedMarkerElement({
					map,
					position: { lat: fix.latitude, lng: fix.longitude },
					content: fixDot
				});

				marker.addListener('click', () => {
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
				});

				fixMarkers.push(marker);
			});

			// Update bounds to show all fixes
			const bounds = new google.maps.LatLngBounds();
			fixesInOrder.forEach((fix) => {
				bounds.extend({ lat: fix.latitude, lng: fix.longitude });
			});
			map.fitBounds(bounds);

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

		// Update altitude chart
		if (altitudeChartContainer && Plotly) {
			const fixesInOrder = [...data.fixes].reverse();
			const timestamps = fixesInOrder.map((fix) => new Date(fix.timestamp));
			const altitudesMsl = fixesInOrder.map((fix) => fix.altitude_msl_feet || 0);

			// Only include AGL trace if AGL data is available
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

			Plotly.react(altitudeChartContainer, traces, getPlotlyLayout($theme === 'dark'));
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

			// Create info window for altitude display
			altitudeInfoWindow = new google.maps.InfoWindow();

			// Add small markers for each fix (white dots) with click/touch handlers
			fixesInOrder.forEach((fix) => {
				const fixDot = document.createElement('div');
				fixDot.innerHTML = `
					<div style="background-color: white; width: 6px; height: 6px; border-radius: 50%; border: 1px solid rgba(0,0,0,0.3); box-shadow: 0 0 2px rgba(0,0,0,0.5); cursor: pointer;"></div>
				`;

				const marker = new google.maps.marker.AdvancedMarkerElement({
					map,
					position: { lat: fix.latitude, lng: fix.longitude },
					content: fixDot
				});

				// Add click/touch handler to show altitude info
				marker.addListener('click', () => {
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
				});

				fixMarkers.push(marker);
			});

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

		// Initialize altitude chart
		if (altitudeChartContainer && data.fixes.length > 0) {
			try {
				const fixesInOrder = [...data.fixes].reverse();

				// Prepare data for the chart
				const timestamps = fixesInOrder.map((fix) => new Date(fix.timestamp));
				const altitudesMsl = fixesInOrder.map((fix) => fix.altitude_msl_feet || 0);

				// Create traces - only include AGL if data is available
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
					const data = event.detail || event;
					if (data.points && data.points.length > 0) {
						const pointIndex = data.points[0].pointIndex;
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

							// Pan to the position on the map
							map?.panTo({ lat: fix.latitude, lng: fix.longitude });
						}
					}
				});

				// Close info window when not hovering
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
		// Access theme to make this effect reactive
		const currentTheme = $theme;

		// Update chart if it exists
		if (altitudeChartContainer && Plotly && data.fixes.length > 0) {
			const isDark = currentTheme === 'dark';
			Plotly.relayout(altitudeChartContainer, getPlotlyLayout(isDark));
		}
	});

	// Cleanup on component unmount
	onDestroy(() => {
		stopPolling();
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
		<div class="mb-4 flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
			<div class="flex flex-col gap-2">
				<div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-4">
					<h1 class="flex items-center gap-2 h1">
						<Plane class="h-8 w-8" />
						Flight
					</h1>
					<div class="flex flex-wrap items-center gap-2">
						<FlightStateBadge state={data.flight.state} />
						{#if isOutlanding}
							<span
								class="chip flex items-center gap-2 preset-filled-warning-500 text-base font-semibold"
							>
								<MapPinMinus class="h-5 w-5" />
								Outlanding
							</span>
						{/if}
					</div>
				</div>
				{#if data.device}
					<div class="flex flex-wrap items-center gap-2 text-sm">
						{#if data.device.registration}
							<span class="font-mono font-semibold">
								{data.device.registration}
								{#if data.device.competition_number}
									<span class="text-surface-500-400-token ml-1"
										>({data.device.competition_number})</span
									>
								{/if}
							</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if data.device.aircraft_model}
							<span class="font-semibold">{data.device.aircraft_model}</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if data.device.aircraft_type_ogn}
							<span
								class="chip {getAircraftTypeColor(
									data.device.aircraft_type_ogn
								)} text-xs font-semibold"
							>
								{getAircraftTypeOgnDescription(data.device.aircraft_type_ogn)}
							</span>
							<span class="text-surface-400-500-token">•</span>
						{/if}
						{#if data.flight.device_id && data.flight.device_address && data.flight.device_address_type}
							<a
								href="/devices/{data.flight.device_id}"
								target="_blank"
								rel="noopener noreferrer"
								class="btn flex items-center gap-1 preset-filled-primary-500 btn-sm"
							>
								<span class="font-mono text-xs">
									{formatDeviceAddress(data.flight.device_address_type, data.flight.device_address)}
								</span>
								<ExternalLink class="h-3 w-3" />
							</a>
						{/if}
					</div>
				{/if}
			</div>
			<div class="flex items-center gap-2">
				{#if data.flight.previous_flight_id || data.flight.next_flight_id}
					<div class="flex items-center gap-1">
						{#if data.flight.previous_flight_id}
							<a
								href="/flights/{data.flight.previous_flight_id}"
								class="btn preset-tonal btn-sm"
								title="Previous flight for this device"
							>
								<ChevronLeft class="h-4 w-4" />
								Previous
							</a>
						{:else}
							<button class="btn preset-tonal btn-sm" disabled title="No previous flight">
								<ChevronLeft class="h-4 w-4" />
								Previous
							</button>
						{/if}
						{#if data.flight.next_flight_id}
							<a
								href="/flights/{data.flight.next_flight_id}"
								class="btn preset-tonal btn-sm"
								title="Next flight for this device"
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</a>
						{:else}
							<button class="btn preset-tonal btn-sm" disabled title="No next flight">
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						{/if}
					</div>
				{/if}
				<button
					onclick={downloadKML}
					class="btn flex items-center gap-2 preset-filled-primary-500"
					type="button"
				>
					<Download class="h-4 w-4" />
					KML
				</button>
			</div>
		</div>

		<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
			<!-- Takeoff -->
			<div class="flex items-start gap-3">
				<PlaneTakeoff class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Takeoff</div>
					<div class="font-semibold">
						{#if data.flight.takeoff_time}
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(data.flight.takeoff_time)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(data.flight.takeoff_time)}</span>
						{:else}
							Unknown
						{/if}
					</div>
					<div class="text-surface-600-300-token text-sm">
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
						<div class="text-surface-600-300-token flex items-center gap-2 text-sm">
							<span>Runway {data.flight.takeoff_runway_ident}</span>
							{#if data.flight.runways_inferred === true}
								<span
									class="preset-tonal-surface-500 chip flex items-center gap-1 text-xs"
									title="This runway was inferred from the aircraft's heading during takeoff, not matched to airport runway data"
								>
									<Info class="h-3 w-3" />
									Inferred
								</span>
							{/if}
						</div>
					{:else if data.flight.departure_airport}
						<div class="text-surface-600-300-token text-sm">Runway Unknown</div>
					{/if}
				</div>
			</div>

			<!-- Landing / Timeout (hidden for active flights) -->
			{#if data.flight.state === 'timed_out' || data.flight.landing_time}
				<div class="flex items-start gap-3">
					<PlaneLanding class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">
							{data.flight.state === 'timed_out' ? 'Timed Out' : 'Landing'}
						</div>
						<div class="font-semibold">
							{#if data.flight.state === 'timed_out' && data.flight.timed_out_at}
								<!-- Mobile: relative time only -->
								<span class="md:hidden">{formatDateTimeMobile(data.flight.timed_out_at)}</span>
								<!-- Desktop: relative time with full datetime -->
								<span class="hidden md:inline">{formatDateTime(data.flight.timed_out_at)}</span>
							{:else if data.flight.landing_time}
								<!-- Mobile: relative time only -->
								<span class="md:hidden">{formatDateTimeMobile(data.flight.landing_time)}</span>
								<!-- Desktop: relative time with full datetime -->
								<span class="hidden md:inline">{formatDateTime(data.flight.landing_time)}</span>
							{/if}
						</div>
						<div class="text-surface-600-300-token text-sm">
							{#if data.flight.state === 'timed_out'}
								No beacons received for 5+ minutes
							{:else if data.flight.landing_time}
								{#if data.flight.arrival_airport && data.flight.arrival_airport_id}
									<a href="/airports/{data.flight.arrival_airport_id}" class="anchor">
										{data.flight.arrival_airport}
									</a>
								{:else if data.flight.arrival_airport}
									{data.flight.arrival_airport}
								{:else}
									Unknown
								{/if}
							{/if}
						</div>
						{#if data.flight.landing_time && data.flight.arrival_airport}
							{#if data.flight.landing_runway_ident}
								<div class="text-surface-600-300-token flex items-center gap-2 text-sm">
									<span>Runway {data.flight.landing_runway_ident}</span>
									{#if data.flight.runways_inferred === true}
										<span
											class="preset-tonal-surface-500 chip flex items-center gap-1 text-xs"
											title="This runway was inferred from the aircraft's heading during landing, not matched to airport runway data"
										>
											<Info class="h-3 w-3" />
											Inferred
										</span>
									{/if}
								</div>
							{:else}
								<div class="text-surface-600-300-token text-sm">Runway Unknown</div>
							{/if}
						{/if}
					</div>
				</div>
			{/if}

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

			<!-- Maximum Altitude -->
			{#if maxAltitude() || maxAglAltitude()}
				<div class="flex items-start gap-3">
					<MountainSnow class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Maximum Altitude</div>
						{#if maxAltitude()}
							<div class="font-semibold">{formatAltitude(maxAltitude() ?? undefined)} MSL</div>
						{/if}
						{#if maxAglAltitude()}
							<div class="text-surface-600-300-token text-sm">
								{formatAltitude(maxAglAltitude() ?? undefined)} AGL
							</div>
						{/if}
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

			<!-- Tow Aircraft -->
			{#if data.flight.towed_by_device_id}
				<div class="flex items-start gap-3">
					<TrendingUp class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Tow Aircraft</div>
						<div class="font-semibold">
							<TowAircraftLink deviceId={data.flight.towed_by_device_id} size="md" />
						</div>
					</div>
				</div>
			{/if}

			<!-- Recognized at -->
			<div class="flex items-start gap-3">
				<Clock class="mt-1 h-5 w-5 text-primary-500" />
				<div>
					<div class="text-surface-600-300-token text-sm">Recognized at</div>
					<div class="font-semibold">
						<!-- Mobile: relative time only -->
						<span class="md:hidden">{formatDateTimeMobile(data.flight.created_at)}</span>
						<!-- Desktop: relative time with full datetime -->
						<span class="hidden md:inline">{formatDateTime(data.flight.created_at)}</span>
					</div>
					<div class="text-surface-600-300-token text-sm">When flight was first detected</div>
				</div>
			</div>

			<!-- Latest fix (for active flights) -->
			{#if data.flight.state === 'active' && data.fixes.length > 0}
				<div class="flex items-start gap-3">
					<Clock class="mt-1 h-5 w-5 text-primary-500" />
					<div>
						<div class="text-surface-600-300-token text-sm">Latest fix</div>
						<div class="font-semibold">
							<!-- Mobile: relative time only -->
							<span class="md:hidden">{formatDateTimeMobile(data.fixes[0].timestamp)}</span>
							<!-- Desktop: relative time with full datetime -->
							<span class="hidden md:inline">{formatDateTime(data.fixes[0].timestamp)}</span>
						</div>
						<div class="text-surface-600-300-token text-sm">Most recent position update</div>
					</div>
				</div>
			{/if}
		</div>
	</div>

	<!-- Map -->
	{#if data.fixes.length > 0}
		<div class="card p-4">
			<div class="mb-3 flex items-center justify-between">
				<h2 class="h3">Flight Track</h2>
				<label class="flex cursor-pointer items-center gap-2">
					<input
						type="checkbox"
						class="checkbox"
						bind:checked={includeNearbyFlights}
						onchange={handleNearbyFlightsToggle}
					/>
					<span class="text-sm">Include Nearby Flights</span>
					{#if isLoadingNearbyFlights}
						<span class="text-surface-600-300-token text-xs">(Loading...)</span>
					{/if}
				</label>
			</div>
			<div bind:this={mapContainer} class="h-96 w-full rounded-lg"></div>
		</div>

		<!-- Altitude Chart -->
		<div class="card p-4">
			<h2 class="mb-3 h3">Altitude Profile</h2>
			<div bind:this={altitudeChartContainer} class="h-80 w-full"></div>
		</div>
	{/if}

	<!-- Nearby Flights Section -->
	<div class="card p-6">
		<h2 class="mb-4 h2">Nearby Flights</h2>

		{#if !showStandaloneNearby}
			<button
				onclick={fetchStandaloneNearbyFlights}
				class="btn preset-filled-primary-500"
				type="button"
			>
				Find nearby flights
			</button>
		{:else if isLoadingStandaloneNearby}
			<div class="flex flex-col items-center gap-4 py-8">
				<div class="flex items-center gap-3">
					<RadarLoader />
					<span class="text-lg font-semibold">Searching for nearby flights...</span>
				</div>
				<div class="text-surface-600-300-token max-w-2xl text-center text-sm">
					<p class="mb-2">This may take a minute.</p>
					<p>
						Finding flights that were in the air within <strong>15 minutes</strong> of this flight
						and within <strong>50 miles</strong> of the departure airport.
					</p>
				</div>
			</div>
		{:else if standaloneNearbyFlights.length === 0}
			<div class="text-surface-600-300-token py-8 text-center">
				<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<p>No nearby flights found.</p>
			</div>
		{:else}
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each standaloneNearbyFlights as flight (flight.id)}
					<a href="/flights/{flight.id}" class="card p-4 hover:ring-2 hover:ring-primary-500">
						<div class="space-y-2">
							{#if flight.registration}
								<div>
									<div class="text-surface-600-300-token text-xs">Registration</div>
									<div class="font-mono text-sm font-semibold">{flight.registration}</div>
								</div>
							{/if}
							{#if flight.aircraft_model}
								<div>
									<div class="text-surface-600-300-token text-xs">Model</div>
									<div class="text-sm font-semibold">{flight.aircraft_model}</div>
								</div>
							{/if}
							<div>
								<div class="text-surface-600-300-token text-xs">Takeoff</div>
								<div class="text-sm">{formatDateTimeMobile(flight.takeoff_time)}</div>
								{#if flight.departure_airport}
									<div class="text-surface-600-300-token text-xs">{flight.departure_airport}</div>
								{/if}
							</div>
							{#if flight.landing_time}
								<div>
									<div class="text-surface-600-300-token text-xs">Landing</div>
									<div class="text-sm">{formatDateTimeMobile(flight.landing_time)}</div>
									{#if flight.arrival_airport}
										<div class="text-surface-600-300-token text-xs">{flight.arrival_airport}</div>
									{/if}
								</div>
							{/if}
						</div>
					</a>
				{/each}
			</div>
		{/if}
	</div>

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
		</div>

		{#if data.fixes.length === 0}
			<div class="text-surface-600-300-token py-8 text-center">
				<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<p>No position data available for this flight.</p>
			</div>
		{:else}
			<FixesList
				fixes={paginatedFixes}
				showHideInactive={false}
				showRaw={true}
				useRelativeTimes={true}
				showClimb={true}
				emptyMessage="No position data available for this flight."
			/>

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
						</button>
						<button
							onclick={() => goToPage(currentPage + 1)}
							disabled={currentPage === totalPages}
							class="btn preset-tonal btn-sm"
							type="button"
							title="Next page"
						>
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

	<!-- Nearby Flights List -->
	{#if includeNearbyFlights && nearbyFlights.length > 0}
		<div class="card p-6">
			<h2 class="mb-4 h2">Nearby Flights ({nearbyFlights.length})</h2>
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each nearbyFlights as flight, index (flight.id)}
					{@const colorIndex = index % 6}
					{@const colors = ['#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4']}
					{@const colorNames = ['Blue', 'Green', 'Orange', 'Purple', 'Pink', 'Cyan']}
					<a href="/flights/{flight.id}" class="card p-4 hover:ring-2 hover:ring-primary-500">
						<div class="mb-3 flex items-center justify-between">
							<div class="flex items-center gap-2">
								<div
									class="h-3 w-8 rounded"
									style="background-color: {colors[colorIndex]}; opacity: 0.6;"
								></div>
								<span class="text-surface-600-300-token text-xs">{colorNames[colorIndex]}</span>
							</div>
							{#if flight.aircraft_model}
								<span class="text-surface-600-300-token text-xs">{flight.aircraft_model}</span>
							{/if}
						</div>
						<div class="space-y-2">
							{#if flight.registration}
								<div>
									<div class="text-surface-600-300-token text-xs">Registration</div>
									<div class="font-mono text-sm font-semibold">{flight.registration}</div>
								</div>
							{/if}
							<div>
								<div class="text-surface-600-300-token text-xs">Takeoff</div>
								<div class="text-sm">{formatDateTimeMobile(flight.takeoff_time)}</div>
								{#if flight.departure_airport}
									<div class="text-surface-600-300-token text-xs">{flight.departure_airport}</div>
								{/if}
							</div>
							{#if flight.landing_time}
								<div>
									<div class="text-surface-600-300-token text-xs">Landing</div>
									<div class="text-sm">{formatDateTimeMobile(flight.landing_time)}</div>
									{#if flight.arrival_airport}
										<div class="text-surface-600-300-token text-xs">{flight.arrival_airport}</div>
									{/if}
								</div>
							{/if}
						</div>
					</a>
				{/each}
			</div>
		</div>
	{/if}
</div>
