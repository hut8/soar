<script lang="ts">
	import type { Viewer } from 'cesium';
	import { Cartesian3, Math as CesiumMath } from 'cesium';
	import { MapPin, Antenna, Home, Eye, EyeOff, ChevronDown, ChevronUp, Play } from '@lucide/svelte';

	// Props
	let {
		viewer,
		showAirports = $bindable(true),
		showReceivers = $bindable(true),
		flightColorScheme = $bindable<'altitude' | 'time'>('altitude'),
		playbackFlightId = $bindable<string | null>(null)
	}: {
		viewer: Viewer;
		showAirports?: boolean;
		showReceivers?: boolean;
		flightColorScheme?: 'altitude' | 'time';
		playbackFlightId?: string | null;
	} = $props();

	// State
	let showControls = $state(true);
	let showLegend = $state(false);
	let flightIdInput = $state('');

	/**
	 * Reset camera to CONUS center
	 */
	function goHome(): void {
		viewer.camera.flyTo({
			destination: Cartesian3.fromDegrees(-98.5795, 39.8283, 5000000),
			orientation: {
				heading: 0.0,
				pitch: -CesiumMath.PI_OVER_TWO,
				roll: 0.0
			},
			duration: 2.0
		});
	}

	/**
	 * Start flight playback
	 */
	function startPlayback(): void {
		if (flightIdInput.trim()) {
			playbackFlightId = flightIdInput.trim();
		}
	}
</script>

<!-- Controls Panel -->
<div class="controls-container">
	<!-- Toggle Button -->
	<button
		onclick={() => (showControls = !showControls)}
		class="toggle-btn btn preset-filled-surface-500 btn-sm"
		title={showControls ? 'Hide controls' : 'Show controls'}
	>
		{#if showControls}
			<EyeOff size={16} />
		{:else}
			<Eye size={16} />
		{/if}
	</button>

	<!-- Controls Panel (collapsible) -->
	{#if showControls}
		<div class="controls-panel space-y-4 card p-4">
			<h3 class="h4 font-bold">Globe Controls</h3>

			<!-- Home Button -->
			<button onclick={goHome} class="btn w-full justify-start preset-filled-primary-500">
				<Home size={16} />
				Reset View
			</button>

			<!-- Layer Toggles -->
			<div class="space-y-2">
				<h4 class="text-sm font-semibold">Layers</h4>

				<!-- Airports Toggle -->
				<label class="flex cursor-pointer items-center space-x-2">
					<input type="checkbox" bind:checked={showAirports} class="checkbox" />
					<MapPin size={16} />
					<span>Airports (zoom in)</span>
				</label>

				<!-- Receivers Toggle -->
				<label class="flex cursor-pointer items-center space-x-2">
					<input type="checkbox" bind:checked={showReceivers} class="checkbox" />
					<Antenna size={16} />
					<span>Receivers (zoom in)</span>
				</label>
			</div>

			<!-- Flight Color Scheme -->
			<div class="space-y-2">
				<h4 class="text-sm font-semibold">Flight Path Colors</h4>

				<div class="flex space-x-2">
					<button
						onclick={() => (flightColorScheme = 'altitude')}
						class="btn flex-1 btn-sm {flightColorScheme === 'altitude'
							? 'preset-filled-primary-500'
							: 'preset-filled-surface-500'}"
					>
						Altitude
					</button>
					<button
						onclick={() => (flightColorScheme = 'time')}
						class="btn flex-1 btn-sm {flightColorScheme === 'time'
							? 'preset-filled-primary-500'
							: 'preset-filled-surface-500'}"
					>
						Time
					</button>
				</div>
			</div>

			<!-- Flight Playback -->
			<div class="space-y-2">
				<h4 class="text-sm font-semibold">Flight Playback</h4>
				<div class="flex space-x-2">
					<input
						type="text"
						bind:value={flightIdInput}
						placeholder="Enter Flight ID"
						class="input flex-1"
						onkeydown={(e) => e.key === 'Enter' && startPlayback()}
					/>
					<button
						onclick={startPlayback}
						class="btn preset-filled-primary-500"
						title="Start playback"
					>
						<Play size={16} />
					</button>
				</div>
				<p class="text-xs opacity-75">Enter a flight ID to replay the flight path</p>
			</div>

			<!-- Legend Toggle -->
			<button
				onclick={() => (showLegend = !showLegend)}
				class="btn w-full justify-between preset-filled-surface-500"
			>
				<span>Legend</span>
				{#if showLegend}
					<ChevronUp size={16} />
				{:else}
					<ChevronDown size={16} />
				{/if}
			</button>

			<!-- Legend (collapsible) -->
			{#if showLegend}
				<div class="legend space-y-2 card p-3 text-sm">
					<div>
						<h5 class="mb-1 font-semibold">Aircraft Colors (Altitude)</h5>
						<div class="flex items-center space-x-2">
							<div class="h-4 w-4 rounded" style="background: rgb(239, 68, 68);"></div>
							<span>Low (500 ft)</span>
						</div>
						<div class="flex items-center space-x-2">
							<div class="h-4 w-4 rounded" style="background: rgb(149, 99, 157);"></div>
							<span>Medium (9,000 ft)</span>
						</div>
						<div class="flex items-center space-x-2">
							<div class="h-4 w-4 rounded" style="background: rgb(59, 130, 246);"></div>
							<span>High (18,000+ ft)</span>
						</div>
					</div>

					{#if flightColorScheme === 'time'}
						<div>
							<h5 class="mb-1 font-semibold">Flight Path Colors (Time)</h5>
							<div class="flex items-center space-x-2">
								<div class="h-4 w-4 rounded" style="background: rgb(147, 51, 234);"></div>
								<span>Takeoff (purple)</span>
							</div>
							<div class="flex items-center space-x-2">
								<div class="h-4 w-4 rounded" style="background: rgb(251, 146, 60);"></div>
								<span>Landing (orange)</span>
							</div>
						</div>
					{/if}

					<div>
						<h5 class="mb-1 font-semibold">Markers</h5>
						<div class="flex items-center space-x-2">
							<div class="h-4 w-4 rounded-full bg-green-500"></div>
							<span>Takeoff</span>
						</div>
						<div class="flex items-center space-x-2">
							<div class="h-4 w-4 rounded-full bg-red-500"></div>
							<span>Landing</span>
						</div>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>

<style>
	.controls-container {
		position: fixed;
		top: 10px;
		right: 10px;
		z-index: 1000;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 8px;
		max-width: 280px;
	}

	.toggle-btn {
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
	}

	.controls-panel {
		background: rgba(255, 255, 255, 0.95);
		box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3);
		max-height: calc(100vh - 100px);
		overflow-y: auto;
	}

	:global(.dark) .controls-panel {
		background: rgba(30, 30, 30, 0.95);
	}

	.legend {
		background: rgba(240, 240, 240, 0.95);
	}

	:global(.dark) .legend {
		background: rgba(40, 40, 40, 0.95);
	}

	.checkbox {
		width: 18px;
		height: 18px;
		cursor: pointer;
	}

	/* Mobile responsive */
	@media (max-width: 640px) {
		.controls-container {
			top: 60px;
			right: 8px;
			max-width: calc(100vw - 16px);
		}

		.controls-panel {
			max-height: calc(100vh - 140px);
		}
	}
</style>
