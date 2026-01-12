<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { CESIUM_ION_TOKEN } from '$lib/config';
	import AircraftLayer from '$lib/components/cesium/AircraftLayer.svelte';
	import FlightPathLayer from '$lib/components/cesium/FlightPathLayer.svelte';
	import AirportLayer from '$lib/components/cesium/AirportLayer.svelte';
	import ReceiverLayer from '$lib/components/cesium/ReceiverLayer.svelte';
	import GlobeControls from '$lib/components/cesium/GlobeControls.svelte';
	import TimelineController from '$lib/components/cesium/TimelineController.svelte';
	import 'cesium/Build/Cesium/Widgets/widgets.css';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'Globe']);

	let cesiumContainer: HTMLDivElement | undefined = $state();
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	let viewer = $state<any>(null);
	let viewerReady = $state(false);
	let cesiumLoading = $state(true);
	let cesiumError = $state<string | null>(null);

	// Layer visibility state
	let showAirports = $state(true);
	let showReceivers = $state(true);

	// Flight path state
	let selectedFlightIds = $state<string[]>([]);
	let flightColorScheme = $state<'altitude' | 'time'>('altitude');

	// Timeline playback state - read from URL parameter if present
	let playbackFlightId = $state<string | null>(page.url.searchParams.get('flight'));

	// Function to dynamically load Cesium script
	function loadCesiumScript(): Promise<void> {
		return new Promise((resolve, reject) => {
			// Check if Cesium is already loaded
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

	onMount(() => {
		let observer: MutationObserver | null = null;

		const initCesium = async () => {
			try {
				// Dynamically load Cesium
				await loadCesiumScript();
				cesiumLoading = false;

				// Wait for DOM to update and container to be rendered
				// Using requestAnimationFrame twice ensures the DOM has updated
				await new Promise((resolve) => {
					requestAnimationFrame(() => {
						requestAnimationFrame(resolve);
					});
				});

				if (!cesiumContainer) {
					throw new Error('Cesium container not found');
				}

				const {
					Ion,
					Viewer,
					Cartesian3,
					Math: CesiumMath,
					createWorldTerrainAsync
				} = window.Cesium;

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
				createWorldTerrainAsync()
					// eslint-disable-next-line @typescript-eslint/no-explicit-any
					.then((terrainProvider: any) => {
						if (viewer) {
							viewer.terrainProvider = terrainProvider;
						}
					})
					.catch((error: Error) => {
						logger.warn('Failed to load Cesium World Terrain, using default ellipsoid: {error}', {
							error
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

				// Apply dark/light mode styles to InfoBox iframe content
				function applyInfoBoxStyles() {
					const infoBoxFrame = document.querySelector(
						'.cesium-infoBox-iframe'
					) as HTMLIFrameElement;
					if (infoBoxFrame && infoBoxFrame.contentDocument) {
						const isDark = document.documentElement.classList.contains('dark');
						const frameDoc = infoBoxFrame.contentDocument;

						// Inject or update style tag in iframe
						let styleTag = frameDoc.getElementById('theme-styles') as HTMLStyleElement;
						if (!styleTag) {
							styleTag = frameDoc.createElement('style');
							styleTag.id = 'theme-styles';
							frameDoc.head.appendChild(styleTag);
						}

						styleTag.textContent = isDark
							? 'body { background-color: rgba(30, 30, 30, 0.95) !important; color: #fff !important; } * { color: #fff !important; }'
							: 'body { background-color: rgba(255, 255, 255, 0.95) !important; color: #000 !important; } * { color: #000 !important; }';
					}
				}

				// Watch for InfoBox changes and theme changes
				observer = new MutationObserver(() => {
					applyInfoBoxStyles();
				});

				// Observe InfoBox visibility changes
				const infoBox = document.querySelector('.cesium-infoBox');
				if (infoBox) {
					observer.observe(infoBox, { attributes: true, childList: true, subtree: true });
				}

				// Observe theme changes on document root
				observer.observe(document.documentElement, {
					attributes: true,
					attributeFilter: ['class']
				});

				// Add listener for entity selection to log aircraft data
				viewer.selectedEntityChanged.addEventListener(() => {
					const selected = viewer.selectedEntity;
					if (selected && selected.properties?.aircraftId) {
						logger.debug('[GLOBE] Selected aircraft entity: {data}', {
							data: {
								id: selected.id,
								name: selected.name,
								properties: {
									aircraftId: selected.properties.aircraftId?.getValue(),
									registration: selected.properties.registration?.getValue(),
									fixId: selected.properties.fixId?.getValue(),
									altitude: selected.properties.altitude?.getValue(),
									timestamp: selected.properties.timestamp?.getValue(),
									isOld: selected.properties.isOld?.getValue()
								},
								position: selected.position,
								description: selected.description
							}
						});
					}
				});

				// Mark viewer as ready for child components
				viewerReady = true;
			} catch (error) {
				logger.error('Failed to load Cesium: {error}', { error });
				cesiumLoading = false;
				cesiumError = error instanceof Error ? error.message : 'Failed to load 3D globe library';
			}
		};

		// Initialize Cesium
		initCesium();

		// Cleanup on component destroy
		return () => {
			if (observer) {
				observer.disconnect();
			}
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
	{#if cesiumError}
		<!-- Error state -->
		<div class="loading-container">
			<div class="error-message">
				<h2>Failed to Load Globe</h2>
				<p>{cesiumError}</p>
				<button onclick={() => window.location.reload()} class="variant-filled-primary btn">
					Retry
				</button>
			</div>
		</div>
	{:else if cesiumLoading}
		<!-- Loading state -->
		<div class="loading-container">
			<div class="loading-spinner">
				<div class="spinner"></div>
				<p>Loading 3D Globe...</p>
			</div>
		</div>
	{:else}
		<!-- Globe loaded successfully -->
		<div bind:this={cesiumContainer} class="cesium-container"></div>

		<!-- Layers - render when viewer is ready -->
		{#if viewerReady && viewer}
			<!-- Only show live aircraft when NOT in playback mode -->
			{#if !playbackFlightId}
				<AircraftLayer {viewer} />
			{/if}
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

	/* Loading and error states */
	.loading-container {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 100%;
		height: 100%;
		background: var(--color-surface-50);
	}

	:global(html.dark) .loading-container {
		background: var(--color-surface-900);
	}

	.loading-spinner {
		text-align: center;
	}

	.loading-spinner p {
		margin-top: 1rem;
		font-size: 1.125rem;
		color: var(--color-surface-700);
	}

	:global(html.dark) .loading-spinner p {
		color: var(--color-surface-300);
	}

	.spinner {
		border: 4px solid rgba(0, 0, 0, 0.1);
		border-left-color: var(--color-primary-500);
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
	}

	.error-message h2 {
		font-size: 1.5rem;
		font-weight: 600;
		margin-bottom: 1rem;
		color: var(--color-error-500);
	}

	.error-message p {
		margin-bottom: 1.5rem;
		color: var(--color-surface-700);
	}

	:global(html.dark) .error-message p {
		color: var(--color-surface-300);
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

	/* Cesium toolbar buttons - light mode */
	:global(.cesium-button) {
		background-color: rgba(48, 51, 54, 0.8);
		border-color: rgba(48, 51, 54, 0.9);
	}

	:global(.cesium-button:hover) {
		background-color: rgba(73, 76, 79, 0.9);
	}

	/* Cesium navigation help button and other toolbar items */
	:global(.cesium-toolbar-button),
	:global(.cesium-navigationHelpButton-wrapper),
	:global(.cesium-sceneModePicker-wrapper),
	:global(.cesium-baseLayerPicker-dropDown) {
		background-color: rgba(48, 51, 54, 0.8);
	}

	/* Cesium SVG icons - ensure they're visible in light mode */
	:global(.cesium-button svg),
	:global(.cesium-toolbar-button svg) {
		filter: brightness(0) invert(1);
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
