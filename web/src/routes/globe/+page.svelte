<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { Ion, Viewer, Cartesian3, Math as CesiumMath } from 'cesium';
	import { CESIUM_ION_TOKEN } from '$lib/config';
	import AircraftLayer from '$lib/components/cesium/AircraftLayer.svelte';
	import FlightPathLayer from '$lib/components/cesium/FlightPathLayer.svelte';
	import AirportLayer from '$lib/components/cesium/AirportLayer.svelte';
	import ReceiverLayer from '$lib/components/cesium/ReceiverLayer.svelte';
	import GlobeControls from '$lib/components/cesium/GlobeControls.svelte';
	import TimelineController from '$lib/components/cesium/TimelineController.svelte';
	import 'cesium/Build/Cesium/Widgets/widgets.css';

	let cesiumContainer: HTMLDivElement;
	let viewer = $state<Viewer | null>(null);
	let viewerReady = $state(false);

	// Layer visibility state
	let showAirports = $state(true);
	let showReceivers = $state(true);

	// Flight path state
	let selectedFlightIds = $state<string[]>([]);
	let flightColorScheme = $state<'altitude' | 'time'>('altitude');

	// Timeline playback state - read from URL parameter if present
	let playbackFlightId = $state<string | null>(page.url.searchParams.get('flight'));

	onMount(() => {
		// Set Cesium Ion access token
		Ion.defaultAccessToken = CESIUM_ION_TOKEN;

		// Create Cesium Viewer with ion imagery and terrain
		viewer = new Viewer(cesiumContainer, {
			timeline: false, // Disable timeline for now
			animation: false, // Disable animation widget for now
			baseLayerPicker: true, // Allow switching base layers
			geocoder: true, // Enable location search
			homeButton: true, // Enable home button
			sceneModePicker: true, // Enable 2D/3D/Columbus view switcher
			navigationHelpButton: true, // Show navigation help
			fullscreenButton: true // Enable fullscreen
		});

		// Enable Cesium World Terrain for 3D terrain (async)
		import('cesium').then(({ createWorldTerrainAsync }) => {
			createWorldTerrainAsync()
				.then((terrainProvider) => {
					if (viewer) {
						viewer.terrainProvider = terrainProvider;
					}
				})
				.catch((error) => {
					console.warn('Failed to load Cesium World Terrain, using default ellipsoid:', error);
				});
		});

		// Set initial camera position to CONUS center
		viewer.camera.setView({
			destination: Cartesian3.fromDegrees(-98.5795, 39.8283, 5000000), // CONUS center, 5000km altitude
			orientation: {
				heading: 0.0,
				pitch: -CesiumMath.PI_OVER_TWO, // Looking straight down
				roll: 0.0
			}
		});

		// Enable depth testing against terrain (once terrain is enabled)
		viewer.scene.globe.depthTestAgainstTerrain = true;

		// Enable atmospheric fog for better depth perception
		viewer.scene.fog.enabled = true;
		viewer.scene.fog.density = 0.0002;

		// Mark viewer as ready for child components
		viewerReady = true;

		// Cleanup on component destroy
		return () => {
			if (viewer && !viewer.isDestroyed()) {
				viewer.destroy();
			}
		};
	});
</script>

<svelte:head>
	<title>3D Globe - SOAR</title>
</svelte:head>

<div class="globe-page">
	<div bind:this={cesiumContainer} class="cesium-container"></div>

	<!-- Layers - render when viewer is ready -->
	{#if viewerReady && viewer}
		<AircraftLayer {viewer} />
		<FlightPathLayer
			{viewer}
			bind:flightIds={selectedFlightIds}
			bind:colorScheme={flightColorScheme}
		/>
		<AirportLayer {viewer} bind:enabled={showAirports} />
		<ReceiverLayer {viewer} bind:enabled={showReceivers} />

		<!-- UI Controls -->
		<GlobeControls
			{viewer}
			bind:showAirports
			bind:showReceivers
			bind:flightColorScheme
			bind:playbackFlightId
		/>

		<!-- Timeline Controller for flight playback -->
		<TimelineController
			{viewer}
			bind:flightId={playbackFlightId}
			onClose={() => (playbackFlightId = null)}
		/>
	{/if}
</div>

<style>
	.globe-page {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		margin: 0;
		padding: 0;
		overflow: hidden;
	}

	.cesium-container {
		width: 100%;
		height: 100%;
	}

	/* Ensure Cesium widgets are visible and styled */
	:global(.cesium-viewer) {
		width: 100%;
		height: 100%;
	}

	:global(.cesium-viewer-toolbar) {
		top: 10px;
		right: 10px;
	}

	/* Move InfoBox to the left side */
	:global(.cesium-infoBox) {
		left: 10px;
		right: auto;
	}

	/* InfoBox light mode styling */
	:global(.cesium-infoBox) {
		background-color: rgba(255, 255, 255, 0.95);
	}

	:global(.cesium-infoBox-title) {
		background-color: rgba(0, 0, 0, 0.1);
	}

	:global(.cesium-infoBox),
	:global(.cesium-infoBox *) {
		color: #000;
	}

	/* InfoBox dark mode styling */
	:global(html.dark .cesium-infoBox) {
		background-color: rgba(30, 30, 30, 0.95);
	}

	:global(html.dark .cesium-infoBox-title) {
		background-color: rgba(255, 255, 255, 0.1);
	}

	:global(html.dark .cesium-infoBox),
	:global(html.dark .cesium-infoBox *) {
		color: #fff;
	}

	/* Adjust mobile positioning */
	@media (max-width: 640px) {
		:global(.cesium-infoBox) {
			left: 8px;
			top: 60px;
		}
	}
</style>
