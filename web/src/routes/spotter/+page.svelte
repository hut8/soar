<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { X } from '@lucide/svelte';
	import { SvelteMap, SvelteSet } from 'svelte/reactivity';
	import { CESIUM_ION_TOKEN } from '$lib/config';
	import { FixFeed } from '$lib/services/FixFeed';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { calculateBoundingBox, fixToARPosition } from '$lib/ar/calculations';
	import { throttle } from '$lib/ar/projection';
	import { getLogger } from '$lib/logging';
	import type {
		ARSettings,
		ARUserPosition,
		ARDeviceOrientation,
		ARAircraftPosition,
		ARScreenPosition
	} from '$lib/ar/types';
	import type { BulkAreaSubscriptionMessage } from '$lib/services/FixFeed';
	import type { Fix } from '$lib/types';
	import { watchlist } from '$lib/stores/watchlist';
	import AircraftMarker from '$lib/components/ar/AircraftMarker.svelte';
	import CompassOverlay from '$lib/components/ar/CompassOverlay.svelte';
	import DebugPanel from '$lib/components/ar/DebugPanel.svelte';
	import AircraftListModal from '$lib/components/ar/AircraftListModal.svelte';
	import DirectionIndicator from '$lib/components/ar/DirectionIndicator.svelte';
	import LocationPicker from '$lib/components/spotter/LocationPicker.svelte';
	import SpotterControls from '$lib/components/spotter/SpotterControls.svelte';
	import 'cesium/Build/Cesium/Widgets/widgets.css';

	const logger = getLogger(['soar', 'SpotterPage']);

	// Cesium state
	let cesiumContainer: HTMLDivElement | undefined = $state();
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let viewer = $state<any>(null);
	let viewerReady = $state(false);
	let cesiumLoading = $state(true);
	let cesiumError = $state<string | null>(null);

	// Location state
	let showLocationPicker = $state(true);
	let userPosition: ARUserPosition | null = $state(null);
	let locationLabel = $state<string | undefined>(undefined);

	// Camera orientation state (derived from Cesium camera)
	let deviceOrientation: ARDeviceOrientation | null = $state(null);

	// Settings
	let settings: ARSettings = $state({
		rangeNm: 50,
		filterAirborne: false,
		showDebug: false,
		fovHorizontal: 60,
		fovVertical: 45
	});

	// Aircraft state
	let showAircraftList = $state(false);
	let targetAircraftId: string | null = $state(null);
	let aircraftPositions = new SvelteMap<
		string,
		{ aircraft: ARAircraftPosition; screen: ARScreenPosition }
	>();
	let currentSubscription: BulkAreaSubscriptionMessage | null = null;

	const targetAircraft = $derived(
		targetAircraftId ? (aircraftPositions.get(targetAircraftId)?.aircraft ?? null) : null
	);

	let screenWidth = $state(0);
	let screenHeight = $state(0);

	// Watched aircraft IDs
	const watchedIds = $derived(new Set($watchlist.entries.map((e) => e.aircraftId)));

	// Services
	const fixFeed = FixFeed.getInstance();
	const aircraftRegistry = AircraftRegistry.getInstance();

	// Check URL params for initial location
	const urlLat = page.url.searchParams.get('lat');
	const urlLon = page.url.searchParams.get('lon');
	const urlHeading = page.url.searchParams.get('heading');

	if (urlLat && urlLon) {
		const lat = parseFloat(urlLat);
		const lon = parseFloat(urlLon);
		if (!isNaN(lat) && !isNaN(lon)) {
			showLocationPicker = false;
			userPosition = { latitude: lat, longitude: lon, altitude: 0, accuracy: 0 };
		}
	}

	function updateDimensions() {
		screenWidth = window.innerWidth;
		screenHeight = window.innerHeight;
	}

	// Handle aircraft registry events
	const unsubscribeRegistry = aircraftRegistry.subscribe((event) => {
		if (event.type === 'fix_received' || event.type === 'aircraft_updated') {
			updateAircraftProjections();
		}
	});

	// Update aircraft subscription based on user position and range
	const updateAircraftSubscription = throttle(() => {
		if (!userPosition) return;

		const bounds = calculateBoundingBox(
			userPosition.latitude,
			userPosition.longitude,
			settings.rangeNm
		);

		if (currentSubscription) {
			fixFeed.sendWebSocketMessage({
				...currentSubscription,
				action: 'unsubscribe'
			});
		}

		const subscription: BulkAreaSubscriptionMessage = {
			action: 'subscribe',
			type: 'area_bulk',
			bounds
		};

		fixFeed.sendWebSocketMessage(subscription);
		currentSubscription = subscription;
	}, 2000);

	// Project aircraft using Cesium's wgs84ToWindowCoordinates for on-screen,
	// and fall back to bearing math for off-screen direction indicators
	const updateAircraftProjections = throttle(() => {
		if (!userPosition || !deviceOrientation || !viewer || viewer.isDestroyed()) return;

		const Cesium = window.Cesium;
		const scene = viewer.scene;
		const allAircraft = aircraftRegistry.getAllAircraft();
		const activeIds = new SvelteSet<string>();

		for (const aircraft of allAircraft) {
			const currentFix = aircraft.currentFix as Fix | null;
			if (!currentFix) continue;

			const arPosition = fixToARPosition(currentFix, userPosition, aircraft.registration);
			if (!arPosition) continue;

			if (arPosition.distance > settings.rangeNm) continue;
			if (settings.filterAirborne && arPosition.altitudeFeet < 100) continue;

			// Use Cesium for screen projection
			const altMeters = arPosition.altitudeFeet * 0.3048;
			const cartesian = Cesium.Cartesian3.fromDegrees(
				arPosition.longitude,
				arPosition.latitude,
				altMeters
			);

			const windowPos = Cesium.SceneTransforms.wgs84ToWindowCoordinates(scene, cartesian);

			let screenPosition: ARScreenPosition;
			if (
				windowPos &&
				windowPos.x >= -100 &&
				windowPos.x <= screenWidth + 100 &&
				windowPos.y >= -100 &&
				windowPos.y <= screenHeight + 100
			) {
				screenPosition = {
					x: Math.round(windowPos.x),
					y: Math.round(windowPos.y),
					visible: true,
					distance: arPosition.distance,
					bearing: arPosition.bearing - deviceOrientation.heading,
					elevation: arPosition.elevation
				};
			} else {
				// Off-screen: store with visible=false for DirectionIndicator
				screenPosition = {
					x: -1000,
					y: -1000,
					visible: false,
					distance: arPosition.distance,
					bearing: arPosition.bearing - deviceOrientation.heading,
					elevation: arPosition.elevation
				};
			}

			aircraftPositions.set(aircraft.id, {
				aircraft: arPosition,
				screen: screenPosition
			});
			activeIds.add(aircraft.id);
		}

		// Remove aircraft no longer in range
		for (const id of aircraftPositions.keys()) {
			if (!activeIds.has(id)) {
				aircraftPositions.delete(id);
			}
		}
	}, 100);

	function handleAircraftClick(aircraftId: string) {
		logger.debug('Aircraft clicked: {aircraftId}', { aircraftId });
	}

	function handleListClick() {
		showAircraftList = true;
	}

	function handleAircraftSelect(aircraft: ARAircraftPosition) {
		targetAircraftId = aircraft.aircraftId;
		showAircraftList = false;
	}

	function dismissTarget() {
		targetAircraftId = null;
	}

	const allAircraftList = $derived(Array.from(aircraftPositions.values()).map((p) => p.aircraft));

	const visibleAircraftCount = $derived.by(() => {
		let count = 0;
		for (const { screen } of aircraftPositions.values()) {
			if (screen.visible) count++;
		}
		return count;
	});

	const targetScreenPosition = $derived.by(() => {
		if (!targetAircraftId) return null;
		return aircraftPositions.get(targetAircraftId)?.screen ?? null;
	});

	// Cesium loading
	function loadCesiumScript(): Promise<void> {
		return new Promise((resolve, reject) => {
			if (window.Cesium) {
				resolve();
				return;
			}
			const script = document.createElement('script');
			script.src = '/cesium/Cesium.js';
			script.async = true;
			script.onload = () => resolve();
			script.onerror = () => reject(new Error('Failed to load Cesium.js'));
			document.head.appendChild(script);
		});
	}

	function handleLocationSelect(lat: number, lon: number, label?: string) {
		showLocationPicker = false;
		locationLabel = label;
		userPosition = { latitude: lat, longitude: lon, altitude: 0, accuracy: 0 };

		if (viewer && !viewer.isDestroyed()) {
			placeCamera(lat, lon);
		}
	}

	async function placeCamera(lat: number, lon: number) {
		if (!viewer || viewer.isDestroyed()) return;

		const Cesium = window.Cesium;

		// Sample terrain height at location
		let terrainHeight = 0;
		try {
			const positions = [Cesium.Cartographic.fromDegrees(lon, lat)];
			const provider = viewer.terrainProvider;
			const sampled = await Cesium.sampleTerrainMostDetailed(provider, positions);
			if (sampled && sampled[0] && sampled[0].height != null) {
				terrainHeight = sampled[0].height;
			}
		} catch (err) {
			logger.warn('Terrain sampling failed, using 0m: {err}', { err });
		}

		const eyeHeight = terrainHeight + 1.7;

		// Update altitude in userPosition
		if (userPosition) {
			userPosition = { ...userPosition, altitude: eyeHeight };
		}

		const initialHeading = urlHeading ? parseFloat(urlHeading) : 0;
		const headingRad = Cesium.Math.toRadians(isNaN(initialHeading) ? 0 : initialHeading);

		viewer.camera.setView({
			destination: Cesium.Cartesian3.fromDegrees(lon, lat, eyeHeight),
			orientation: {
				heading: headingRad,
				pitch: 0.0, // Horizontal
				roll: 0.0
			}
		});

		// Lock the camera: allow rotation/look but disable translation/zoom
		const controller = viewer.scene.screenSpaceCameraController;
		controller.enableTranslate = false;
		controller.enableZoom = false;
		controller.enableTilt = false;
		controller.enableLook = true;

		// Enable rotate for mouse-drag panning
		controller.enableRotate = true;

		// Override rotation behavior to act like "look" (first-person)
		// Set minimum/maximum distance to 0 to prevent zooming
		controller.minimumZoomDistance = 0;
		controller.maximumZoomDistance = 0;

		// Start tracking subscription + projections
		updateAircraftSubscription();
		startCameraTracking();
	}

	function startCameraTracking() {
		if (!viewer || viewer.isDestroyed()) return;

		const Cesium = window.Cesium;

		viewer.scene.postRender.addEventListener(() => {
			if (viewer.isDestroyed()) return;

			const camera = viewer.camera;
			const heading = Cesium.Math.toDegrees(camera.heading);
			const pitch = Cesium.Math.toDegrees(camera.pitch);
			const fov = camera.frustum.fov ? Cesium.Math.toDegrees(camera.frustum.fov) : 60;
			const aspect = camera.frustum.aspectRatio || screenWidth / screenHeight || 16 / 9;

			// Cesium's fov is vertical; derive horizontal via trigonometry
			const fovVertical = fov;
			const halfVRad = (fovVertical * Math.PI) / 360;
			const fovHorizontal = (2 * Math.atan(Math.tan(halfVRad) * aspect) * 180) / Math.PI;

			settings.fovHorizontal = fovHorizontal;
			settings.fovVertical = fovVertical;

			deviceOrientation = {
				heading: ((heading % 360) + 360) % 360,
				pitch,
				roll: 0,
				absolute: true
			};

			updateAircraftProjections();
		});
	}

	async function initCesium() {
		try {
			await loadCesiumScript();
			cesiumLoading = false;

			await new Promise((r) => {
				requestAnimationFrame(() => requestAnimationFrame(r));
			});

			if (!cesiumContainer) {
				throw new Error('Cesium container not found');
			}

			const Cesium = window.Cesium;

			Cesium.Ion.defaultAccessToken = CESIUM_ION_TOKEN;

			viewer = new Cesium.Viewer(cesiumContainer, {
				timeline: false,
				animation: false,
				baseLayerPicker: false,
				geocoder: false,
				homeButton: false,
				sceneModePicker: false,
				navigationHelpButton: false,
				fullscreenButton: false,
				infoBox: false,
				selectionIndicator: false
			});

			// Enable world terrain
			Cesium.createWorldTerrainAsync()
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				.then((terrainProvider: any) => {
					if (viewer && !viewer.isDestroyed()) {
						viewer.terrainProvider = terrainProvider;

						// If we already have a user position, re-place camera with terrain
						if (userPosition) {
							placeCamera(userPosition.latitude, userPosition.longitude);
						}
					}
				})
				.catch((error: Error) => {
					logger.warn('Failed to load terrain: {error}', { error });
				});

			// Enable depth testing and atmosphere
			viewer.scene.globe.depthTestAgainstTerrain = true;
			viewer.scene.fog.enabled = true;
			viewer.scene.fog.density = 0.0004;
			viewer.scene.skyAtmosphere.show = true;

			viewerReady = true;

			// If we already have a location from URL params, place camera
			if (userPosition) {
				placeCamera(userPosition.latitude, userPosition.longitude);
			}
		} catch (error) {
			logger.error('Failed to initialize Cesium: {error}', { error });
			cesiumLoading = false;
			cesiumError = error instanceof Error ? error.message : 'Failed to load 3D viewer';
		}
	}

	onMount(async () => {
		updateDimensions();
		window.addEventListener('resize', updateDimensions);

		// Load watchlist
		watchlist.load();

		// Connect fix feed
		fixFeed.connect();

		// Init Cesium
		await initCesium();
	});

	onDestroy(() => {
		if (currentSubscription) {
			fixFeed.sendWebSocketMessage({
				...currentSubscription,
				action: 'unsubscribe'
			});
		}

		unsubscribeRegistry();
		window.removeEventListener('resize', updateDimensions);

		if (viewer && !viewer.isDestroyed()) {
			viewer.destroy();
		}
	});

	// Re-subscribe when range changes
	$effect(() => {
		// Access settings.rangeNm to track changes reactively
		if (userPosition && settings.rangeNm) {
			updateAircraftSubscription();
		}
	});
</script>

<svelte:head>
	<title>Spotter View - SOAR</title>
</svelte:head>

<div class="spotter-page">
	{#if cesiumError}
		<div class="loading-container">
			<div class="error-message">
				<h2>Failed to Load Viewer</h2>
				<p>{cesiumError}</p>
				<button onclick={() => window.location.reload()} class="btn-retry">Retry</button>
			</div>
		</div>
	{:else if cesiumLoading}
		<div class="loading-container">
			<div class="loading-spinner">
				<div class="spinner"></div>
				<p>Loading 3D Viewer...</p>
			</div>
		</div>
	{:else}
		<!-- Cesium container -->
		<div bind:this={cesiumContainer} class="cesium-container"></div>

		<!-- Aircraft overlay layer (on top of Cesium) -->
		{#if viewerReady && userPosition && deviceOrientation}
			<div class="aircraft-layer">
				{#each [...aircraftPositions.entries()] as [aircraftId, { aircraft, screen }] (aircraftId)}
					<AircraftMarker
						{aircraft}
						screenPosition={screen}
						watched={watchedIds.has(aircraftId)}
						rangeNm={settings.rangeNm}
						onclick={() => handleAircraftClick(aircraftId)}
					/>
				{/each}
			</div>

			<!-- Compass overlay -->
			<CompassOverlay heading={deviceOrientation.heading} />

			<!-- Controls -->
			<SpotterControls
				bind:settings
				onSettingsClick={() => (settings.showDebug = !settings.showDebug)}
				onListClick={handleListClick}
				onLocationClick={() => (showLocationPicker = true)}
			/>

			<!-- Debug panel -->
			{#if settings.showDebug}
				<DebugPanel
					position={userPosition}
					orientation={deviceOrientation}
					aircraftCount={aircraftPositions.size}
					visibleCount={visibleAircraftCount}
				/>
			{/if}

			<!-- Direction indicator for off-screen target -->
			{#if targetAircraft && deviceOrientation && !targetScreenPosition?.visible}
				<DirectionIndicator
					{targetAircraft}
					{deviceOrientation}
					{settings}
					onDismiss={dismissTarget}
				/>
			{/if}
		{/if}

		<!-- Location label -->
		{#if locationLabel && userPosition}
			<div class="location-badge">
				{locationLabel}
			</div>
		{/if}
	{/if}

	<!-- Close button -->
	<button class="btn-close" onclick={() => goto(resolve('/'))}>
		<X size={24} />
	</button>

	<!-- Location picker modal -->
	{#if showLocationPicker}
		<LocationPicker
			onSelect={handleLocationSelect}
			onClose={() => {
				if (userPosition) {
					showLocationPicker = false;
				} else {
					goto(resolve('/'));
				}
			}}
			initialLat={userPosition?.latitude ?? null}
			initialLon={userPosition?.longitude ?? null}
		/>
	{/if}

	<!-- Aircraft list modal -->
	{#if showAircraftList}
		<AircraftListModal
			aircraft={allAircraftList}
			onSelect={handleAircraftSelect}
			onClose={() => (showAircraftList = false)}
		/>
	{/if}
</div>

<style>
	.spotter-page {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: black;
		overflow: hidden;
	}

	.cesium-container {
		width: 100%;
		height: 100%;
	}

	.aircraft-layer {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
		z-index: 5;
	}

	.aircraft-layer :global(button) {
		pointer-events: auto;
	}

	.loading-container {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
	}

	.loading-spinner {
		text-align: center;
		color: white;
	}

	.loading-spinner p {
		margin-top: 1rem;
		font-size: 1.125rem;
		opacity: 0.8;
	}

	.spinner {
		border: 4px solid rgba(255, 255, 255, 0.1);
		border-left-color: rgb(var(--color-primary-500));
		border-radius: 50%;
		width: 48px;
		height: 48px;
		animation: spin 1s linear infinite;
		margin: 0 auto;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.error-message {
		text-align: center;
		padding: 2rem;
		max-width: 400px;
		color: white;
	}

	.error-message h2 {
		font-size: 1.5rem;
		font-weight: 600;
		margin-bottom: 1rem;
		color: #ef4444;
	}

	.error-message p {
		margin-bottom: 1.5rem;
		opacity: 0.8;
	}

	.btn-retry {
		background: rgb(var(--color-primary-500));
		border: none;
		border-radius: 0.5rem;
		padding: 0.625rem 1.5rem;
		color: white;
		font-size: 0.875rem;
		font-weight: 600;
		cursor: pointer;
	}

	.btn-close {
		position: fixed;
		top: 1rem;
		right: 1rem;
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(8px);
		border: none;
		border-radius: 50%;
		width: 48px;
		height: 48px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		cursor: pointer;
		z-index: 100;
	}

	.location-badge {
		position: fixed;
		top: 1rem;
		left: 1rem;
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(8px);
		padding: 0.5rem 0.75rem;
		border-radius: 0.5rem;
		color: white;
		font-size: 0.8125rem;
		font-weight: 600;
		z-index: 90;
		max-width: 200px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	/* Style Cesium credits to be unobtrusive */
	:global(.cesium-viewer-bottom) {
		opacity: 0.5;
		font-size: 0.625rem;
	}
</style>
