<script lang="ts">
	import { onMount } from 'svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import maplibregl from 'maplibre-gl';
	import 'maplibre-gl/dist/maplibre-gl.css';
	import { page } from '$app/stores';
	import { Settings, ListChecks, LocateFixed, Globe, Map, Satellite, Loader } from '@lucide/svelte';
	import WatchlistModal from '$lib/components/WatchlistModal.svelte';
	import SettingsModal from '$lib/components/SettingsModal.svelte';
	import AircraftStatusModal from '$lib/components/AircraftStatusModal.svelte';
	import AirportModal from '$lib/components/AirportModal.svelte';
	import AirspaceModal from '$lib/components/AirspaceModal.svelte';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { FixFeed } from '$lib/services/FixFeed';
	import type {
		Aircraft,
		Airport,
		Airspace,
		Fix,
		AircraftSearchResponse,
		AircraftCluster
	} from '$lib/types';
	import { isAircraftItem, isClusterItem } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { isStaging } from '$lib/config';
	import { getLogger } from '$lib/logging';
	import { loadMapState, saveMapState } from '$lib/utils/mapStatePersistence';
	import { serverCall } from '$lib/api/server';
	import {
		createAircraftIconDataUrl,
		getAircraftIconName,
		getAllIconShapes,
		getIconShapeForCategory,
		createAltitudeColorExpression
	} from '$lib/utils/aircraftIcons';
	import {
		AirspaceLayerManager,
		AirportLayerManager,
		ReceiverLayerManager,
		RunwayLayerManager
	} from '$lib/services/maplibre';

	const logger = getLogger(['soar', 'Live']);

	// Map projection type
	type MapProjection = 'globe' | 'mercator';

	// Map style type
	type MapStyle = 'satellite' | 'streets' | 'terrain';

	// Aircraft rendering limit to prevent performance issues
	const MAX_AIRCRAFT_DISPLAY = 200;

	// Map state
	let mapContainer: HTMLDivElement;
	let map: maplibregl.Map | null = null;
	let isLocating = $state(false);
	let userMarker: maplibregl.Marker | null = null;

	// Projection and style state
	let currentProjection: MapProjection = $state('globe');
	let currentStyle: MapStyle = $state('satellite');

	// Debug state
	let debugSquareMiles = $state(0);
	let debugAircraftCount = $state(0);
	let debugZoomLevel = $state(0);
	let showDebugPanel = $state(false);

	// Modal state
	let showSettingsModal = $state(false);
	let showWatchlistModal = $state(false);
	let showAircraftStatusModal = $state(false);
	let selectedAircraft: Aircraft | null = $state(null);
	let showAirportModal = $state(false);
	let selectedAirport: Airport | null = $state(null);
	let showAirspaceModal = $state(false);
	let selectedAirspace: Airspace | null = $state(null);

	// Settings state (received from SettingsModal via callback)
	let currentSettings = $state({
		showCompassRose: true,
		showAirportMarkers: true,
		showReceiverMarkers: true,
		showAirspaceMarkers: true,
		showRunwayOverlays: false
	});

	// Loading states
	let mapLoading = $state(true);
	let aircraftLoading = $state(false);

	// Debounce timers
	let viewportDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Aircraft data - using SvelteMap for reactivity (SvelteMap is already reactive)
	const aircraftMap = new SvelteMap<string, { aircraft: Aircraft; fix: Fix }>();

	// Cluster data
	const clusterMap = new SvelteMap<string, AircraftCluster>();
	let isClusteredMode = $state(false);

	// Services
	const fixFeed = FixFeed.getInstance();
	const aircraftRegistry = AircraftRegistry.getInstance();

	// Layer managers
	const airspaceLayerManager = new AirspaceLayerManager({
		onAirspaceClick: (airspace) => {
			selectedAirspace = airspace;
			showAirspaceModal = true;
		}
	});

	const airportLayerManager = new AirportLayerManager({
		onAirportClick: (airport) => {
			selectedAirport = airport;
			showAirportModal = true;
		},
		onAirportsLoaded: () => {
			// Refresh runway overlays when airports are loaded
			runwayLayerManager.refresh();
		}
	});

	const receiverLayerManager = new ReceiverLayerManager();

	// Runway layer manager (uses airport manager for runway data)
	const runwayLayerManager = new RunwayLayerManager({
		getRunways: () => airportLayerManager.getRunways()
	});

	// Get style specification for the given map style (using free tile sources)
	function getStyleSpec(style: MapStyle): maplibregl.StyleSpecification {
		switch (style) {
			case 'satellite':
				// ESRI World Imagery - free for non-commercial use
				return {
					version: 8,
					sources: {
						esri: {
							type: 'raster',
							tiles: [
								'https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}'
							],
							tileSize: 256,
							attribution:
								'Tiles &copy; Esri &mdash; Source: Esri, i-cubed, USDA, USGS, AEX, GeoEye, Getmapping, Aerogrid, IGN, IGP, UPR-EGP, and the GIS User Community',
							maxzoom: 19
						}
					},
					layers: [
						{
							id: 'esri-satellite',
							type: 'raster',
							source: 'esri',
							minzoom: 0,
							maxzoom: 19
						}
					]
				};
			case 'streets':
				// OpenStreetMap
				return {
					version: 8,
					sources: {
						osm: {
							type: 'raster',
							tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
							tileSize: 256,
							attribution:
								'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
							maxzoom: 19
						}
					},
					layers: [
						{
							id: 'osm',
							type: 'raster',
							source: 'osm',
							minzoom: 0,
							maxzoom: 19
						}
					]
				};
			case 'terrain':
				// OpenTopoMap - topographic map
				return {
					version: 8,
					sources: {
						opentopomap: {
							type: 'raster',
							tiles: ['https://tile.opentopomap.org/{z}/{x}/{y}.png'],
							tileSize: 256,
							attribution:
								'Map data: &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors, <a href="http://viewfinderpanoramas.org">SRTM</a> | Map style: &copy; <a href="https://opentopomap.org">OpenTopoMap</a>',
							maxzoom: 17
						}
					},
					layers: [
						{
							id: 'opentopomap',
							type: 'raster',
							source: 'opentopomap',
							minzoom: 0,
							maxzoom: 17
						}
					]
				};
		}
	}

	// Calculate viewport area in square miles
	function calculateViewportArea(): number {
		if (!map) return 0;

		const bounds = map.getBounds();
		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		// Calculate area using spherical approximation
		const latDiff = Math.abs(ne.lat - sw.lat);
		const lngDiff = Math.abs(ne.lng - sw.lng);
		const avgLat = (ne.lat + sw.lat) / 2;

		// Approximate miles per degree
		const milesPerDegLat = 69.0;
		const milesPerDegLng = 69.0 * Math.cos((avgLat * Math.PI) / 180);

		const heightMiles = latDiff * milesPerDegLat;
		const widthMiles = lngDiff * milesPerDegLng;

		return heightMiles * widthMiles;
	}

	// Convert aircraft to GeoJSON feature
	function aircraftToFeature(aircraft: Aircraft, fix: Fix): GeoJSON.Feature<GeoJSON.Point> {
		// Get icon shape based on aircraft category
		const shape = getIconShapeForCategory(aircraft.aircraftCategory);
		return {
			type: 'Feature',
			geometry: {
				type: 'Point',
				coordinates: [fix.longitude, fix.latitude]
			},
			properties: {
				id: aircraft.id,
				registration: aircraft.registration || aircraft.address,
				altitude: fix.altitudeMslFeet, // Used by altitude color expression (null = gray)
				track: fix.trackDegrees || 0,
				isActive: fix.active,
				timestamp: fix.timestamp,
				aircraftModel: aircraft.aircraftModel || '',
				aircraftCategory: aircraft.aircraftCategory || null,
				iconName: getAircraftIconName(shape)
			}
		};
	}

	// Create aircraft GeoJSON FeatureCollection
	function createAircraftGeoJson(): GeoJSON.FeatureCollection<GeoJSON.Point> {
		const features: GeoJSON.Feature<GeoJSON.Point>[] = [];

		for (const [, data] of aircraftMap) {
			features.push(aircraftToFeature(data.aircraft, data.fix));
		}

		return {
			type: 'FeatureCollection',
			features
		};
	}

	// Convert cluster to GeoJSON feature
	function clusterToFeature(cluster: AircraftCluster): GeoJSON.Feature<GeoJSON.Point> {
		return {
			type: 'Feature',
			geometry: {
				type: 'Point',
				coordinates: [cluster.longitude, cluster.latitude]
			},
			properties: {
				id: cluster.id,
				count: Number(cluster.count),
				north: cluster.bounds.north,
				south: cluster.bounds.south,
				east: cluster.bounds.east,
				west: cluster.bounds.west
			}
		};
	}

	// Create cluster GeoJSON FeatureCollection
	function createClusterGeoJson(): GeoJSON.FeatureCollection<GeoJSON.Point> {
		const features: GeoJSON.Feature<GeoJSON.Point>[] = [];

		for (const [, cluster] of clusterMap) {
			features.push(clusterToFeature(cluster));
		}

		return {
			type: 'FeatureCollection',
			features
		};
	}

	// Update aircraft source
	function updateAircraftSource() {
		if (!map) return;

		const source = map.getSource('aircraft') as maplibregl.GeoJSONSource;
		if (source) {
			const geojson = createAircraftGeoJson();
			logger.debug('[AIRCRAFT] Updating source with {count} features', {
				count: geojson.features.length
			});
			if (geojson.features.length > 0) {
				logger.debug('[AIRCRAFT] First feature: {feature}', {
					feature: JSON.stringify(geojson.features[0])
				});
			}
			source.setData(geojson);
			debugAircraftCount = aircraftMap.size;
		} else {
			logger.warn('[AIRCRAFT] Source not found when trying to update');
		}
	}

	// Update cluster source
	function updateClusterSource() {
		if (!map) return;

		const source = map.getSource('clusters') as maplibregl.GeoJSONSource;
		if (source) {
			const geojson = createClusterGeoJson();
			logger.debug('[CLUSTERS] Updating source with {count} clusters', {
				count: geojson.features.length
			});
			source.setData(geojson);
		}
	}

	// Add aircraft icon images to the map
	// Registers one icon per shape (7 total) - colors are applied at runtime via SDF
	async function addAircraftIcons() {
		if (!map) return;

		const shapes = getAllIconShapes();
		const iconSize = 48; // Icon size in pixels
		const loadPromises: Promise<void>[] = [];

		for (const shape of shapes) {
			const iconName = getAircraftIconName(shape);
			const iconUrl = createAircraftIconDataUrl(shape, iconSize);

			loadPromises.push(
				new Promise<void>((resolve) => {
					const img = new Image();
					img.onload = () => {
						if (map && !map.hasImage(iconName)) {
							// SDF mode allows runtime coloring via icon-color
							map.addImage(iconName, img, { sdf: true });
						}
						resolve();
					};
					img.onerror = () => {
						logger.warn('[AIRCRAFT] Failed to load icon: {iconName}', { iconName });
						resolve();
					};
					img.src = iconUrl;
				})
			);
		}

		await Promise.all(loadPromises);
		logger.debug('[AIRCRAFT] Registered {count} aircraft icons', {
			count: shapes.length
		});
	}

	// Add aircraft layers to map
	async function addAircraftLayers() {
		if (!map) return;

		// Add aircraft icons first
		await addAircraftIcons();

		// Add aircraft source (clustering is handled by backend)
		map.addSource('aircraft', {
			type: 'geojson',
			data: createAircraftGeoJson()
		});

		// Add aircraft markers with rotated icons and fluid altitude coloring
		map.addLayer({
			id: 'aircraft-markers',
			type: 'symbol',
			source: 'aircraft',
			layout: {
				'icon-image': ['get', 'iconName'],
				'icon-size': ['interpolate', ['linear'], ['zoom'], 4, 0.4, 8, 0.6, 12, 0.8],
				'icon-rotate': ['get', 'track'],
				'icon-rotation-alignment': 'map',
				'icon-allow-overlap': true
			},
			paint: {
				// Fluid altitude-based coloring (red at ground -> light blue at 40k ft)
				'icon-color': createAltitudeColorExpression()
			}
		});

		// Add aircraft labels (hidden at low zoom)
		map.addLayer({
			id: 'aircraft-labels',
			type: 'symbol',
			source: 'aircraft',
			minzoom: 8,
			layout: {
				'text-field': ['get', 'registration'],
				'text-font': ['Open Sans Semibold', 'Arial Unicode MS Bold'],
				'text-size': 12,
				'text-offset': [0, 1.5],
				'text-anchor': 'top'
			},
			paint: {
				'text-color': '#ffffff',
				'text-halo-color': 'rgba(0, 0, 0, 0.8)',
				'text-halo-width': 1.5
			}
		});

		// Add click handler for aircraft markers
		map.on('click', 'aircraft-markers', async (e) => {
			if (!e.features || e.features.length === 0) return;

			// Stop propagation to prevent airspace click handler from firing
			e.originalEvent.stopPropagation();

			const feature = e.features[0];
			const aircraftId = feature.properties?.id;

			if (aircraftId) {
				const data = aircraftMap.get(aircraftId);
				if (data) {
					selectedAircraft = data.aircraft;
					showAircraftStatusModal = true;
				}
			}
		});

		// Change cursor on hover
		map.on('mouseenter', 'aircraft-markers', () => {
			if (map) map.getCanvas().style.cursor = 'pointer';
		});
		map.on('mouseleave', 'aircraft-markers', () => {
			if (map) map.getCanvas().style.cursor = '';
		});

		// Add cluster source
		map.addSource('clusters', {
			type: 'geojson',
			data: createClusterGeoJson()
		});

		// Add cluster circle markers
		map.addLayer({
			id: 'cluster-circles',
			type: 'circle',
			source: 'clusters',
			paint: {
				'circle-radius': ['interpolate', ['linear'], ['get', 'count'], 2, 20, 10, 30, 50, 45],
				'circle-color': '#6366f1', // Indigo color for clusters
				'circle-opacity': 0.7,
				'circle-stroke-width': 2,
				'circle-stroke-color': '#ffffff'
			}
		});

		// Add cluster count labels
		map.addLayer({
			id: 'cluster-labels',
			type: 'symbol',
			source: 'clusters',
			layout: {
				'text-field': ['get', 'count'],
				'text-font': ['Open Sans Bold', 'Arial Unicode MS Bold'],
				'text-size': 14,
				'text-allow-overlap': true
			},
			paint: {
				'text-color': '#ffffff'
			}
		});

		// Add click handler for cluster markers - zoom to bounds
		map.on('click', 'cluster-circles', (e) => {
			if (!e.features || e.features.length === 0 || !map) return;

			e.originalEvent.stopPropagation();

			const feature = e.features[0];
			const props = feature.properties;

			if (props?.north && props?.south && props?.east && props?.west) {
				logger.debug('[CLUSTER] Zooming to cluster bounds');
				map.fitBounds(
					[
						[props.west, props.south],
						[props.east, props.north]
					],
					{ padding: 50 }
				);
			}
		});

		// Change cursor on hover for clusters
		map.on('mouseenter', 'cluster-circles', () => {
			if (map) map.getCanvas().style.cursor = 'pointer';
		});
		map.on('mouseleave', 'cluster-circles', () => {
			if (map) map.getCanvas().style.cursor = '';
		});
	}

	// Fetch aircraft in current viewport
	async function fetchAircraftInViewport() {
		if (!map) return;

		aircraftLoading = true;

		try {
			const bounds = map.getBounds();
			const params = new URLSearchParams({
				north: bounds.getNorth().toFixed(6),
				south: bounds.getSouth().toFixed(6),
				east: bounds.getEast().toFixed(6),
				west: bounds.getWest().toFixed(6),
				limit: MAX_AIRCRAFT_DISPLAY.toString()
			});

			const response = await serverCall<AircraftSearchResponse>(`/aircraft?${params.toString()}`);

			logger.debug('[AIRCRAFT] Fetched {total} items from API (clustered: {clustered})', {
				total: response.items.length,
				clustered: response.clustered
			});

			// Update aircraft and cluster maps with fetched data
			aircraftMap.clear();
			clusterMap.clear();
			isClusteredMode = response.clustered;

			let skipped = 0;
			let hasTrackCount = 0;
			for (const item of response.items) {
				if (isAircraftItem(item)) {
					const ac = item.data;
					// currentFix is always present when latitude/longitude exist (updated together in DB)
					const fix = ac.currentFix as Fix | null;
					if (fix) {
						if (fix.trackDegrees != null) {
							hasTrackCount++;
						}
						aircraftMap.set(ac.id, { aircraft: ac, fix });
					} else {
						skipped++;
					}
				} else if (isClusterItem(item)) {
					const cluster = item.data;
					clusterMap.set(cluster.id, cluster);
				}
			}

			logger.debug(
				'[AIRCRAFT] Added {added} aircraft, {clusters} clusters, skipped {skipped} without coords, {hasTrack} with track data',
				{
					added: aircraftMap.size,
					clusters: clusterMap.size,
					skipped,
					hasTrack: hasTrackCount
				}
			);
			updateAircraftSource();
			updateClusterSource();
		} catch (err) {
			logger.error('Failed to fetch aircraft: {error}', { error: err });
			toaster.error({ title: 'Failed to load aircraft' });
		} finally {
			aircraftLoading = false;
		}
	}

	// Handle viewport change
	function handleViewportChange() {
		if (viewportDebounceTimer) {
			clearTimeout(viewportDebounceTimer);
		}

		viewportDebounceTimer = setTimeout(() => {
			const viewportArea = calculateViewportArea();
			debugSquareMiles = viewportArea;
			debugZoomLevel = map?.getZoom() || 0;

			// Save map state
			if (map) {
				const center = map.getCenter();
				saveMapState({ lat: center.lat, lng: center.lng }, map.getZoom());
			}

			// Update layer managers
			airspaceLayerManager.checkAndUpdate(viewportArea, currentSettings.showAirspaceMarkers);
			airportLayerManager.checkAndUpdate(viewportArea, currentSettings.showAirportMarkers);
			receiverLayerManager.checkAndUpdate(viewportArea, currentSettings.showReceiverMarkers);
			runwayLayerManager.checkAndUpdate(viewportArea, currentSettings.showRunwayOverlays);

			fetchAircraftInViewport();
		}, 300);
	}

	// Set map projection
	function setProjection(projection: MapProjection) {
		if (!map) return;
		currentProjection = projection;
		map.setProjection({ type: projection });
	}

	// Set map style
	async function setStyle(style: MapStyle) {
		if (!map) return;

		currentStyle = style;
		const styleSpec = getStyleSpec(style);
		logger.debug('[MAP] Setting style to {style}', { style });

		// Clear layer managers before style change (they need to re-add layers)
		airspaceLayerManager.clear();
		airportLayerManager.clear();
		receiverLayerManager.clear();

		map.setStyle(styleSpec);
		// Re-add layers after style change
		map.once('style.load', async () => {
			map!.setProjection({ type: currentProjection });
			await addAircraftLayers();

			// Re-initialize layer managers
			airspaceLayerManager.setMap(map!);
			airportLayerManager.setMap(map!);
			receiverLayerManager.setMap(map!);
			runwayLayerManager.setMap(map!);

			const viewportArea = calculateViewportArea();
			airspaceLayerManager.checkAndUpdate(viewportArea, currentSettings.showAirspaceMarkers);
			airportLayerManager.checkAndUpdate(viewportArea, currentSettings.showAirportMarkers);
			receiverLayerManager.checkAndUpdate(viewportArea, currentSettings.showReceiverMarkers);
			runwayLayerManager.checkAndUpdate(viewportArea, currentSettings.showRunwayOverlays);

			fetchAircraftInViewport();
		});
	}

	// Handle user location request
	async function handleLocationRequest() {
		if (!map || isLocating) return;

		isLocating = true;

		try {
			const position = await new Promise<GeolocationPosition>((resolve, reject) => {
				navigator.geolocation.getCurrentPosition(resolve, reject, {
					enableHighAccuracy: true,
					timeout: 10000
				});
			});

			const { latitude, longitude } = position.coords;

			// Add or update user marker
			if (userMarker) {
				userMarker.setLngLat([longitude, latitude]);
			} else {
				const el = document.createElement('div');
				el.className = 'user-location-marker';
				el.innerHTML = `
					<div class="pulse"></div>
					<div class="dot"></div>
				`;
				userMarker = new maplibregl.Marker({ element: el })
					.setLngLat([longitude, latitude])
					.addTo(map);
			}

			// Fly to user location
			map.flyTo({
				center: [longitude, latitude],
				zoom: 10,
				duration: 2000
			});

			logger.debug('User location: {lat}, {lng}', { lat: latitude, lng: longitude });
		} catch (err) {
			logger.error('Failed to get location: {error}', { error: err });
			toaster.error({ title: 'Failed to get your location' });
		} finally {
			isLocating = false;
		}
	}

	// Handle settings changes from SettingsModal
	function handleSettingsChange(newSettings: typeof currentSettings) {
		currentSettings = { ...newSettings };

		// Update layer managers with new settings
		if (map) {
			const viewportArea = calculateViewportArea();
			airspaceLayerManager.checkAndUpdate(viewportArea, newSettings.showAirspaceMarkers);
			airportLayerManager.checkAndUpdate(viewportArea, newSettings.showAirportMarkers);
			receiverLayerManager.checkAndUpdate(viewportArea, newSettings.showReceiverMarkers);
			runwayLayerManager.checkAndUpdate(viewportArea, newSettings.showRunwayOverlays);
		}
	}

	// Subscribe to aircraft registry updates
	function subscribeToAircraftUpdates() {
		aircraftRegistry.subscribe((event) => {
			// Ignore updates when in clustered mode - only show cluster markers
			if (isClusteredMode) return;

			if (event.type === 'fix_received' && event.fix) {
				const existing = aircraftMap.get(event.fix.aircraftId);
				if (existing) {
					aircraftMap.set(event.fix.aircraftId, {
						aircraft: existing.aircraft,
						fix: event.fix
					});
					updateAircraftSource();
				}
			}
		});
	}

	onMount(() => {
		// Load initial map state
		const urlParams = new URLSearchParams($page.url.search);
		const loadedState = loadMapState(urlParams);

		// Initialize map with current style
		map = new maplibregl.Map({
			container: mapContainer,
			style: getStyleSpec(currentStyle),
			center: [loadedState.state.center.lng, loadedState.state.center.lat],
			zoom: loadedState.state.zoom
		});

		// Add navigation controls
		map.addControl(new maplibregl.NavigationControl(), 'top-right');

		// Handle map load
		map.on('load', async () => {
			mapLoading = false;

			// Set projection after style is loaded
			map!.setProjection({ type: currentProjection });

			logger.info('MapLibre map loaded with {projection} projection', {
				projection: currentProjection
			});

			// Add aircraft layers
			await addAircraftLayers();

			// Set up layer managers
			airspaceLayerManager.setMap(map!);
			airportLayerManager.setMap(map!);
			receiverLayerManager.setMap(map!);
			runwayLayerManager.setMap(map!);

			// Fit to bounds if provided in URL
			if (loadedState.bounds) {
				map!.fitBounds([
					[loadedState.bounds.west, loadedState.bounds.south],
					[loadedState.bounds.east, loadedState.bounds.north]
				]);
			}

			// Initial aircraft fetch
			fetchAircraftInViewport();

			// Initial layer manager updates
			const initialArea = calculateViewportArea();
			airspaceLayerManager.checkAndUpdate(initialArea, currentSettings.showAirspaceMarkers);
			airportLayerManager.checkAndUpdate(initialArea, currentSettings.showAirportMarkers);
			receiverLayerManager.checkAndUpdate(initialArea, currentSettings.showReceiverMarkers);
			runwayLayerManager.checkAndUpdate(initialArea, currentSettings.showRunwayOverlays);

			// Start live feed
			fixFeed.startLiveFixesFeed();
			subscribeToAircraftUpdates();
		});

		// Handle viewport changes
		map.on('moveend', handleViewportChange);
		map.on('zoomend', handleViewportChange);

		// Cleanup
		return () => {
			fixFeed.stopLiveFixesFeed();
			airspaceLayerManager.dispose();
			airportLayerManager.dispose();
			receiverLayerManager.dispose();
			runwayLayerManager.dispose();
			map?.remove();
		};
	});
</script>

<div class="fixed inset-x-0 top-[42px] bottom-0 flex w-full flex-col">
	<!-- Map container -->
	<div bind:this={mapContainer} class="relative flex-1">
		{#if mapLoading}
			<div class="absolute inset-0 z-10 flex items-center justify-center bg-surface-900/50">
				<Loader class="h-8 w-8 animate-spin text-white" />
			</div>
		{/if}

		<!-- Controls overlay -->
		<div class="absolute top-4 left-4 z-10 flex flex-col gap-2">
			<!-- Location button -->
			<button
				onclick={handleLocationRequest}
				disabled={isLocating}
				class="btn h-10 w-10 rounded-full preset-filled-surface-500 p-0 shadow-lg"
				title="Go to my location"
			>
				{#if isLocating}
					<Loader class="h-5 w-5 animate-spin" />
				{:else}
					<LocateFixed class="h-5 w-5" />
				{/if}
			</button>

			<!-- Watchlist button -->
			<button
				onclick={() => (showWatchlistModal = true)}
				class="btn h-10 w-10 rounded-full preset-filled-surface-500 p-0 shadow-lg"
				title="Watchlist"
			>
				<ListChecks class="h-5 w-5" />
			</button>

			<!-- Settings button -->
			<button
				onclick={() => (showSettingsModal = true)}
				class="btn h-10 w-10 rounded-full preset-filled-surface-500 p-0 shadow-lg"
				title="Settings"
			>
				<Settings class="h-5 w-5" />
			</button>
		</div>

		<!-- Projection and style controls -->
		<div class="absolute top-4 right-16 z-10 flex flex-col gap-2">
			<!-- Projection toggle -->
			<div class="flex shadow-lg">
				<button
					onclick={() => setProjection('globe')}
					class="rounded-l-lg px-3 py-2 transition-colors"
					class:bg-primary-500={currentProjection === 'globe'}
					class:text-white={currentProjection === 'globe'}
					class:bg-surface-700={currentProjection !== 'globe'}
					class:text-surface-200={currentProjection !== 'globe'}
					class:hover:bg-surface-600={currentProjection !== 'globe'}
					title="Globe view"
				>
					<Globe class="h-4 w-4" />
				</button>
				<button
					onclick={() => setProjection('mercator')}
					class="rounded-r-lg px-3 py-2 transition-colors"
					class:bg-primary-500={currentProjection === 'mercator'}
					class:text-white={currentProjection === 'mercator'}
					class:bg-surface-700={currentProjection !== 'mercator'}
					class:text-surface-200={currentProjection !== 'mercator'}
					class:hover:bg-surface-600={currentProjection !== 'mercator'}
					title="Flat map"
				>
					<Map class="h-4 w-4" />
				</button>
			</div>

			<!-- Style selector -->
			<div class="flex shadow-lg">
				<button
					onclick={() => setStyle('satellite')}
					class="rounded-l-lg px-3 py-2 transition-colors"
					class:bg-primary-500={currentStyle === 'satellite'}
					class:text-white={currentStyle === 'satellite'}
					class:bg-surface-700={currentStyle !== 'satellite'}
					class:text-surface-200={currentStyle !== 'satellite'}
					class:hover:bg-surface-600={currentStyle !== 'satellite'}
					title="Satellite"
				>
					<Satellite class="h-4 w-4" />
				</button>
				<button
					onclick={() => setStyle('streets')}
					class="px-3 py-2 transition-colors"
					class:bg-primary-500={currentStyle === 'streets'}
					class:text-white={currentStyle === 'streets'}
					class:bg-surface-700={currentStyle !== 'streets'}
					class:text-surface-200={currentStyle !== 'streets'}
					class:hover:bg-surface-600={currentStyle !== 'streets'}
					title="Streets"
				>
					<Map class="h-4 w-4" />
				</button>
				<button
					onclick={() => setStyle('terrain')}
					class="rounded-r-lg px-3 py-2 transition-colors"
					class:bg-primary-500={currentStyle === 'terrain'}
					class:text-white={currentStyle === 'terrain'}
					class:bg-surface-700={currentStyle !== 'terrain'}
					class:text-surface-200={currentStyle !== 'terrain'}
					class:hover:bg-surface-600={currentStyle !== 'terrain'}
					title="Terrain"
				>
					<svg
						class="h-4 w-4"
						viewBox="0 0 24 24"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
					>
						<path d="M8 21l4-10 4 10M12 3l8 18H4L12 3z" />
					</svg>
				</button>
			</div>
		</div>

		<!-- Debug panel (staging only) -->
		{#if isStaging()}
			<div class="absolute bottom-4 left-4 z-10">
				<button
					onclick={() => (showDebugPanel = !showDebugPanel)}
					class="btn preset-filled-surface-500 text-xs shadow-lg"
				>
					Debug
				</button>
				{#if showDebugPanel}
					<div class="mt-2 rounded-lg bg-surface-800/90 p-3 text-xs text-white shadow-lg">
						<div>Zoom: {debugZoomLevel.toFixed(1)}</div>
						<div>Area: {debugSquareMiles.toLocaleString()} sq mi</div>
						<div>Aircraft: {debugAircraftCount}</div>
						<div>Clusters: {clusterMap.size}</div>
						<div>Mode: {isClusteredMode ? 'Clustered' : 'Individual'}</div>
						<div>Projection: {currentProjection}</div>
					</div>
				{/if}
			</div>
		{/if}

		<!-- Aircraft loading indicator -->
		{#if aircraftLoading}
			<div
				class="absolute right-4 bottom-4 z-10 flex items-center gap-2 rounded-lg bg-surface-800/90 px-3 py-2 text-sm text-white shadow-lg"
			>
				<Loader class="h-4 w-4 animate-spin" />
				Loading aircraft...
			</div>
		{/if}
	</div>
</div>

<!-- Modals -->
<WatchlistModal bind:showModal={showWatchlistModal} />

<SettingsModal bind:showModal={showSettingsModal} onSettingsChange={handleSettingsChange} />

{#if selectedAircraft}
	<AircraftStatusModal bind:showModal={showAircraftStatusModal} bind:selectedAircraft />
{/if}

{#if selectedAirport}
	<AirportModal bind:showModal={showAirportModal} bind:selectedAirport />
{/if}

{#if selectedAirspace}
	<AirspaceModal bind:showModal={showAirspaceModal} bind:selectedAirspace />
{/if}

<style>
	/* User location marker */
	:global(.user-location-marker) {
		position: relative;
		width: 20px;
		height: 20px;
	}

	:global(.user-location-marker .dot) {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 12px;
		height: 12px;
		background: #f97316;
		border: 2px solid white;
		border-radius: 50%;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
	}

	:global(.user-location-marker .pulse) {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		width: 40px;
		height: 40px;
		background: rgba(249, 115, 22, 0.3);
		border-radius: 50%;
		animation: pulse 2s ease-out infinite;
	}

	@keyframes pulse {
		0% {
			transform: translate(-50%, -50%) scale(0.5);
			opacity: 1;
		}
		100% {
			transform: translate(-50%, -50%) scale(1.5);
			opacity: 0;
		}
	}
</style>
