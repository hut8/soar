<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { auth } from '$lib/stores/auth';
	import { Camera, AlertCircle, X, Info } from '@lucide/svelte';
	import { SvelteMap } from 'svelte/reactivity';
	import { ARTracker } from '$lib/services/arTracker';
	import { FixFeed } from '$lib/services/FixFeed';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { calculateBoundingBox, fixToARPosition } from '$lib/ar/calculations';
	import { projectToScreen, throttle } from '$lib/ar/projection';
	import type {
		ARSettings,
		ARUserPosition,
		ARDeviceOrientation,
		ARAircraftPosition,
		ARScreenPosition
	} from '$lib/ar/types';
	import type { BulkAreaSubscriptionMessage } from '$lib/services/FixFeed';
	import AircraftMarker from '$lib/components/ar/AircraftMarker.svelte';
	import CompassOverlay from '$lib/components/ar/CompassOverlay.svelte';
	import ARControls from '$lib/components/ar/ARControls.svelte';
	import DebugPanel from '$lib/components/ar/DebugPanel.svelte';

	// State
	let videoElement: HTMLVideoElement | undefined = $state();
	let cameraReady = $state(false);
	let cameraError = $state<string | null>(null);
	let permissionDenied = $state<'camera' | 'location' | 'orientation' | null>(null);

	let userPosition: ARUserPosition | null = $state(null);
	let deviceOrientation: ARDeviceOrientation | null = $state(null);

	let settings: ARSettings = $state({
		rangeKm: 25,
		filterAirborne: false,
		showDebug: false,
		fovHorizontal: 60,
		fovVertical: 45
	});

	let aircraftPositions = new SvelteMap<
		string,
		{ aircraft: ARAircraftPosition; screen: ARScreenPosition }
	>();
	let currentSubscription: BulkAreaSubscriptionMessage | null = null;

	// Screen dimensions
	let screenWidth = $state(0);
	let screenHeight = $state(0);

	// Services
	const arTracker = ARTracker.getInstance();
	const fixFeed = FixFeed.getInstance();
	const aircraftRegistry = AircraftRegistry.getInstance();

	// Redirect if not authenticated
	$effect(() => {
		if (!$auth.isAuthenticated) {
			goto(resolve('/login'));
		}
	});

	// Update screen dimensions
	function updateDimensions() {
		screenWidth = window.innerWidth;
		screenHeight = window.innerHeight;
	}

	// Handle AR tracker events
	const unsubscribeTracker = arTracker.subscribe((event) => {
		switch (event.type) {
			case 'camera_ready':
				if (videoElement) {
					videoElement.srcObject = event.stream;
					cameraReady = true;
				}
				break;

			case 'camera_error':
				cameraError = event.error;
				break;

			case 'permission_denied':
				permissionDenied = event.permission;
				break;

			case 'position_updated':
				userPosition = event.position;
				updateAircraftSubscription();
				break;

			case 'orientation_updated':
				deviceOrientation = event.orientation;
				updateAircraftProjections();
				break;
		}
	});

	// Handle aircraft registry events
	const unsubscribeRegistry = aircraftRegistry.subscribe((event) => {
		if (event.type === 'fix_added' || event.type === 'aircraft_updated') {
			updateAircraftProjections();
		}
	});

	// Update aircraft subscription based on user position and range
	const updateAircraftSubscription = throttle(() => {
		if (!userPosition) return;

		const bounds = calculateBoundingBox(
			userPosition.latitude,
			userPosition.longitude,
			settings.rangeKm
		);

		// Unsubscribe from old area if exists
		if (currentSubscription) {
			fixFeed.sendWebSocketMessage({
				...currentSubscription,
				action: 'unsubscribe'
			});
		}

		// Subscribe to new area
		const subscription: BulkAreaSubscriptionMessage = {
			action: 'subscribe',
			type: 'area_bulk',
			bounds
		};

		fixFeed.sendWebSocketMessage(subscription);
		currentSubscription = subscription;
	}, 2000);

	// Update aircraft screen positions
	const updateAircraftProjections = throttle(() => {
		if (!userPosition || !deviceOrientation) return;

		const allAircraft = aircraftRegistry.getAllAircraft();
		const newPositions = new SvelteMap<
			string,
			{ aircraft: ARAircraftPosition; screen: ARScreenPosition }
		>();

		for (const aircraft of allAircraft) {
			const currentFix = aircraft.fixes?.[0];
			if (!currentFix) continue;

			const arPosition = fixToARPosition(currentFix, userPosition, aircraft.registration);
			if (!arPosition) continue;

			// Filter by range
			if (arPosition.distance > settings.rangeKm) continue;

			// Filter airborne only if enabled
			if (settings.filterAirborne && arPosition.altitudeFeet < 100) continue;

			const screenPosition = projectToScreen(
				arPosition,
				deviceOrientation,
				settings,
				screenWidth,
				screenHeight
			);

			newPositions.set(aircraft.id, {
				aircraft: arPosition,
				screen: screenPosition
			});
		}

		aircraftPositions = newPositions;
	}, 100);

	// Handle aircraft marker click
	function handleAircraftClick(aircraftId: string) {
		// For now, just log - we'd show a modal here
		console.log('Aircraft clicked:', aircraftId);
		const aircraft = aircraftRegistry.getAircraft(aircraftId);
		if (aircraft) {
			console.log('Aircraft details:', aircraft);
		}
	}

	// Initialize AR on mount
	onMount(async () => {
		updateDimensions();
		window.addEventListener('resize', updateDimensions);

		// Start camera
		await arTracker.startCamera();

		// Start location tracking
		arTracker.startLocation();

		// Start orientation tracking
		await arTracker.startOrientation();

		// Connect to fix feed
		fixFeed.connect();
	});

	// Cleanup on destroy
	onDestroy(() => {
		arTracker.stop();

		// Unsubscribe from area
		if (currentSubscription) {
			fixFeed.sendWebSocketMessage({
				...currentSubscription,
				action: 'unsubscribe'
			});
		}

		unsubscribeTracker();
		unsubscribeRegistry();
		window.removeEventListener('resize', updateDimensions);
	});

	// Watch settings changes
	$effect(() => {
		// Re-subscribe when range changes
		if (userPosition) {
			updateAircraftSubscription();
		}

		// Re-project when filter changes
		updateAircraftProjections();
	});
</script>

<svelte:head>
	<title>AR Aircraft Tracker - SOAR</title>
	<meta
		name="viewport"
		content="width=device-width, initial-scale=1, maximum-scale=1, user-scalable=no"
	/>
</svelte:head>

<div class="ar-page">
	<!-- Camera video background -->
	{#if cameraReady}
		<video bind:this={videoElement} autoplay playsinline class="camera-view"></video>
	{:else if permissionDenied}
		<div class="error-state">
			<AlertCircle size={64} class="text-error-500" />
			<h2>Permission Denied</h2>
			<p>
				{#if permissionDenied === 'camera'}
					Camera access is required for AR mode. Please enable camera permissions in your browser
					settings.
				{:else if permissionDenied === 'location'}
					Location access is required to find aircraft near you. Please enable location permissions.
				{:else if permissionDenied === 'orientation'}
					Device orientation access is required for AR tracking. Please enable motion sensors.
				{/if}
			</p>
			<button class="btn preset-filled-primary-500" onclick={() => goto(resolve('/'))}>
				Return Home
			</button>
		</div>
	{:else if cameraError}
		<div class="error-state">
			<AlertCircle size={64} class="text-error-500" />
			<h2>Camera Error</h2>
			<p>{cameraError}</p>
			<button class="btn preset-filled-primary-500" onclick={() => window.location.reload()}>
				Retry
			</button>
		</div>
	{:else}
		<div class="loading-state">
			<div class="pulse-icon">
				<Camera size={64} />
			</div>
			<p>Starting AR camera...</p>
		</div>
	{/if}

	<!-- AR overlays (only when camera is ready) -->
	{#if cameraReady && userPosition && deviceOrientation}
		<!-- Aircraft markers -->
		<div class="aircraft-layer">
			{#each [...aircraftPositions.entries()] as [aircraftId, { aircraft, screen }] (aircraftId)}
				<AircraftMarker
					{aircraft}
					screenPosition={screen}
					onclick={() => handleAircraftClick(aircraftId)}
				/>
			{/each}
		</div>

		<!-- Compass overlay -->
		<CompassOverlay heading={deviceOrientation.heading} />

		<!-- Controls -->
		<ARControls bind:settings onSettingsClick={() => (settings.showDebug = !settings.showDebug)} />

		<!-- Debug panel -->
		{#if settings.showDebug}
			<DebugPanel
				position={userPosition}
				orientation={deviceOrientation}
				aircraftCount={aircraftPositions.size}
			/>
		{/if}

		<!-- Close button -->
		<button class="btn-close" onclick={() => goto(resolve('/'))}>
			<X size={24} />
		</button>

		<!-- Info hint -->
		<div class="info-hint">
			<Info size={16} />
			<span>Tap aircraft to view details</span>
		</div>
	{/if}
</div>

<style>
	.ar-page {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: black;
		overflow: hidden;
	}

	.camera-view {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.aircraft-layer {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.aircraft-layer :global(button) {
		pointer-events: auto;
	}

	.error-state,
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: 2rem;
		text-align: center;
		color: white;
		gap: 1rem;
	}

	.error-state h2 {
		font-size: 1.5rem;
		font-weight: 700;
	}

	.error-state p {
		max-width: 400px;
		opacity: 0.9;
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

	.info-hint {
		position: fixed;
		top: 160px;
		left: 50%;
		transform: translateX(-50%);
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(8px);
		color: white;
		padding: 0.5rem 1rem;
		border-radius: 1rem;
		font-size: 0.875rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		z-index: 90;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.5;
		}
	}

	.pulse-icon {
		animation: pulse 2s ease-in-out infinite;
	}
</style>
