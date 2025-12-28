<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { Viewer, Entity } from 'cesium';
	import {
		JulianDate as CesiumJulianDate,
		ClockRange,
		ClockStep,
		Cartesian3,
		Math as CesiumMath
	} from 'cesium';
	import { Play, Pause, RotateCcw, Camera } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { createAircraftEntity } from '$lib/cesium/entities';
	import type { Flight, Fix, DataListResponse } from '$lib/types';

	// Props
	let {
		viewer,
		flightId = $bindable<string | null>(null),
		onClose
	}: {
		viewer: Viewer;
		flightId?: string | null;
		onClose?: () => void;
	} = $props();

	// State
	let flight = $state<Flight | null>(null);
	let fixes = $state<Fix[]>([]);
	let isLoading = $state(false);
	let isPlaying = $state(false);
	let playbackSpeed = $state(1);
	let followCamera = $state(true);
	let currentTime = $state<Date | null>(null);
	let currentFixIndex = $state(0);

	// Playback entity
	let playbackEntity: Entity | null = null;

	// Clock listener
	let clockTickListener: (() => void) | null = null;

	// Speed options
	const speedOptions = [
		{ value: 0.5, label: '0.5x' },
		{ value: 1, label: '1x' },
		{ value: 2, label: '2x' },
		{ value: 5, label: '5x' },
		{ value: 10, label: '10x' },
		{ value: 30, label: '30x' }
	];

	/**
	 * Load flight data and fixes
	 */
	async function loadFlightData(): Promise<void> {
		if (!flightId) return;

		isLoading = true;
		try {
			// Load flight details
			const flightResponse = await serverCall<Flight>(`/flights/${flightId}`);
			flight = flightResponse;

			// Load all fixes for the flight
			const fixesResponse = await serverCall<DataListResponse<Fix>>(`/flights/${flightId}/fixes`);
			fixes = fixesResponse.data || [];

			if (fixes.length === 0) {
				console.warn('No fixes found for flight');
				return;
			}

			// Initialize timeline
			initializeTimeline();
		} catch (error) {
			console.error('Error loading flight data:', error);
		} finally {
			isLoading = false;
		}
	}

	/**
	 * Initialize Cesium timeline with flight data
	 */
	function initializeTimeline(): void {
		if (fixes.length === 0) return;

		// Get start and end times from fixes
		const startTime = CesiumJulianDate.fromIso8601(fixes[0].timestamp);
		const endTime = CesiumJulianDate.fromIso8601(fixes[fixes.length - 1].timestamp);

		// Configure clock
		viewer.clock.startTime = startTime.clone();
		viewer.clock.stopTime = endTime.clone();
		viewer.clock.currentTime = startTime.clone();
		viewer.clock.clockRange = ClockRange.LOOP_STOP; // Loop when reaching end
		viewer.clock.clockStep = ClockStep.SYSTEM_CLOCK_MULTIPLIER;
		viewer.clock.multiplier = playbackSpeed;
		viewer.clock.shouldAnimate = false; // Start paused

		// Update current time
		currentTime = new Date(fixes[0].timestamp);
		currentFixIndex = 0;

		// Create initial playback entity
		updatePlaybackEntity(0);

		// Listen for clock ticks
		clockTickListener = viewer.clock.onTick.addEventListener(() => {
			handleClockTick();
		});
	}

	/**
	 * Handle clock tick - update aircraft position based on current time
	 */
	function handleClockTick(): void {
		if (fixes.length === 0) return;

		const currentJulian = viewer.clock.currentTime;
		const currentTimeMillis = CesiumJulianDate.toDate(currentJulian).getTime();

		// Find the closest fix to current time
		let closestIndex = 0;
		let minDiff = Math.abs(new Date(fixes[0].timestamp).getTime() - currentTimeMillis);

		for (let i = 1; i < fixes.length; i++) {
			const diff = Math.abs(new Date(fixes[i].timestamp).getTime() - currentTimeMillis);
			if (diff < minDiff) {
				minDiff = diff;
				closestIndex = i;
			}
		}

		// Update playback entity if fix changed
		if (closestIndex !== currentFixIndex) {
			currentFixIndex = closestIndex;
			currentTime = new Date(fixes[closestIndex].timestamp);
			updatePlaybackEntity(closestIndex);
		}
	}

	/**
	 * Update playback entity position
	 */
	function updatePlaybackEntity(fixIndex: number): void {
		const fix = fixes[fixIndex];
		if (!fix || !flight) return;

		// Remove old entity
		if (playbackEntity) {
			viewer.entities.remove(playbackEntity);
		}

		// Create aircraft entity at this position
		playbackEntity = createAircraftEntity(
			{
				...flight,
				id: `playback-${flight.id}`,
				registration: flight.registration || flight.deviceAddress,
				addressType: '',
				address: '',
				aircraftModel: flight.aircraftModel || '',
				competitionNumber: '',
				tracked: false,
				identified: false,
				createdAt: flight.createdAt || '',
				updatedAt: flight.updatedAt || '',
				fromOgnDdb: false,
				fromAdsbxDdb: false
			},
			fix
		);

		viewer.entities.add(playbackEntity);

		// Follow camera if enabled
		if (followCamera) {
			const altitude = fix.altitudeMslFeet || 0;
			const altitudeMeters = altitude * 0.3048;

			viewer.camera.flyTo({
				destination: Cartesian3.fromDegrees(
					fix.longitude,
					fix.latitude,
					altitudeMeters + 5000 // 5km above aircraft
				),
				orientation: {
					heading: 0.0,
					pitch: -CesiumMath.PI_OVER_FOUR, // 45 degree angle
					roll: 0.0
				},
				duration: 0.5
			});
		}
	}

	/**
	 * Toggle playback
	 */
	function togglePlayback(): void {
		viewer.clock.shouldAnimate = !viewer.clock.shouldAnimate;
		isPlaying = viewer.clock.shouldAnimate;
	}

	/**
	 * Reset playback to start
	 */
	function resetPlayback(): void {
		if (fixes.length === 0) return;

		const startTime = CesiumJulianDate.fromIso8601(fixes[0].timestamp);
		viewer.clock.currentTime = startTime.clone();
		viewer.clock.shouldAnimate = false;
		isPlaying = false;

		currentFixIndex = 0;
		currentTime = new Date(fixes[0].timestamp);
		updatePlaybackEntity(0);
	}

	/**
	 * Change playback speed
	 */
	function changeSpeed(newSpeed: number): void {
		playbackSpeed = newSpeed;
		viewer.clock.multiplier = newSpeed;
	}

	/**
	 * Toggle camera follow
	 */
	function toggleCameraFollow(): void {
		followCamera = !followCamera;
	}

	/**
	 * Close timeline controller
	 */
	function handleClose(): void {
		// Remove playback entity
		if (playbackEntity) {
			viewer.entities.remove(playbackEntity);
			playbackEntity = null;
		}

		// Stop animation
		viewer.clock.shouldAnimate = false;

		// Remove clock listener
		if (clockTickListener) {
			clockTickListener();
			clockTickListener = null;
		}

		// Reset state
		flight = null;
		fixes = [];
		isPlaying = false;
		currentTime = null;
		currentFixIndex = 0;

		// Call onClose callback
		if (onClose) {
			onClose();
		}
	}

	// Watch for flightId changes
	$effect(() => {
		if (flightId) {
			loadFlightData();
		} else {
			handleClose();
		}
	});

	onDestroy(() => {
		// Cleanup
		if (playbackEntity) {
			viewer.entities.remove(playbackEntity);
		}
		if (clockTickListener) {
			clockTickListener();
		}
	});
</script>

{#if flightId && flight}
	<div class="timeline-container">
		<div class="timeline-panel space-y-3 card p-4">
			<!-- Header -->
			<div class="flex items-center justify-between">
				<div>
					<h3 class="h5 font-bold">Flight Playback</h3>
					<p class="text-sm opacity-75">
						{flight.registration || flight.deviceAddress}
						{#if currentTime}
							• {currentTime.toLocaleTimeString()}
						{/if}
					</p>
				</div>
				<button onclick={handleClose} class="preset-tonal-surface-500 btn btn-sm">×</button>
			</div>

			{#if isLoading}
				<div class="flex items-center justify-center py-4">
					<span class="text-sm opacity-75">Loading flight data...</span>
				</div>
			{:else if fixes.length > 0}
				<!-- Playback Controls -->
				<div class="flex items-center gap-2">
					<!-- Play/Pause -->
					<button
						onclick={togglePlayback}
						class="btn btn-sm {isPlaying
							? 'preset-filled-warning-500'
							: 'preset-filled-primary-500'}"
						title={isPlaying ? 'Pause' : 'Play'}
					>
						{#if isPlaying}
							<Pause size={16} />
						{:else}
							<Play size={16} />
						{/if}
					</button>

					<!-- Reset -->
					<button
						onclick={resetPlayback}
						class="preset-tonal-surface-500 btn btn-sm"
						title="Reset to start"
					>
						<RotateCcw size={16} />
					</button>

					<!-- Speed Selector -->
					<div class="flex items-center gap-1">
						<span class="text-xs opacity-75">Speed:</span>
						{#each speedOptions as option (option.value)}
							<button
								onclick={() => changeSpeed(option.value)}
								class="btn btn-sm {playbackSpeed === option.value
									? 'preset-filled-primary-500'
									: 'preset-tonal-surface-500'}"
							>
								{option.label}
							</button>
						{/each}
					</div>

					<!-- Camera Follow -->
					<button
						onclick={toggleCameraFollow}
						class="btn btn-sm {followCamera
							? 'preset-filled-primary-500'
							: 'preset-tonal-surface-500'}"
						title={followCamera ? 'Camera following' : 'Camera fixed'}
					>
						<Camera size={16} />
					</button>
				</div>

				<!-- Progress Bar -->
				<div class="space-y-1">
					<div class="flex items-center justify-between text-xs opacity-75">
						<span>{fixes[0] ? new Date(fixes[0].timestamp).toLocaleTimeString() : ''}</span>
						<span
							>{fixes[fixes.length - 1]
								? new Date(fixes[fixes.length - 1].timestamp).toLocaleTimeString()
								: ''}</span
						>
					</div>
					<div class="relative h-2 rounded bg-surface-400">
						<div
							class="absolute h-full rounded bg-primary-500"
							style="width: {fixes.length > 0 ? (currentFixIndex / (fixes.length - 1)) * 100 : 0}%"
						></div>
					</div>
					<div class="text-center text-xs opacity-75">
						Fix {currentFixIndex + 1} of {fixes.length}
					</div>
				</div>

				<!-- Flight Info -->
				<div class="space-y-1 text-sm opacity-75">
					<p>
						<strong>Duration:</strong>
						{flight.durationSeconds ? Math.round(flight.durationSeconds / 60) : '---'} min
					</p>
					<p>
						<strong>Takeoff:</strong>
						{flight.takeoffTime ? new Date(flight.takeoffTime).toLocaleString() : 'Unknown'}
					</p>
					<p>
						<strong>Landing:</strong>
						{flight.landingTime ? new Date(flight.landingTime).toLocaleString() : 'In Progress'}
					</p>
				</div>
			{:else}
				<div class="py-4 text-center text-sm opacity-75">No fixes available for playback</div>
			{/if}
		</div>
	</div>
{/if}

<style>
	.timeline-container {
		position: fixed;
		bottom: 10px;
		left: 50%;
		transform: translateX(-50%);
		z-index: 1000;
		max-width: 800px;
		width: calc(100% - 20px);
	}

	.timeline-panel {
		background: rgba(255, 255, 255, 0.95);
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
	}

	:global(.dark) .timeline-panel {
		background: rgba(30, 30, 30, 0.95);
	}

	/* Mobile responsive */
	@media (max-width: 640px) {
		.timeline-container {
			bottom: 70px; /* Account for mobile nav */
		}
	}
</style>
