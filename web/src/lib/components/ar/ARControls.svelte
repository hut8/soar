<script lang="ts">
	import { Settings, Minus, Plus, List } from '@lucide/svelte';
	import type { ARSettings } from '$lib/ar/types';

	const MIN_RANGE = 10;
	const MAX_RANGE = 250;
	const STEP = 10;

	// Fixed tick marks for the slider
	const TICK_MARKS = [10, 50, 100, 150, 200, 250];

	let {
		settings = $bindable(),
		onSettingsClick,
		onListClick
	} = $props<{
		settings: ARSettings;
		onSettingsClick?: () => void;
		onListClick?: () => void;
	}>();

	function increaseRange() {
		settings.rangeNm = Math.min(MAX_RANGE, settings.rangeNm + STEP);
	}

	function decreaseRange() {
		settings.rangeNm = Math.max(MIN_RANGE, settings.rangeNm - STEP);
	}

	// Calculate percentage position for a value
	function valuePosition(value: number): number {
		return ((value - MIN_RANGE) / (MAX_RANGE - MIN_RANGE)) * 100;
	}

	// Current value position as a derived value
	const currentPosition = $derived(valuePosition(settings.rangeNm));
</script>

<div class="ar-controls">
	<div class="controls-panel">
		<!-- Range control -->
		<div class="control-group">
			<div class="range-buttons">
				<button class="btn-icon" onclick={decreaseRange} disabled={settings.rangeNm <= MIN_RANGE}>
					<Minus size={20} />
				</button>
				<div class="slider-container">
					<input
						type="range"
						min={MIN_RANGE}
						max={MAX_RANGE}
						step={STEP}
						bind:value={settings.rangeNm}
						class="range-slider"
					/>
					<!-- Current value indicator line -->
					<div
						class="current-indicator"
						style:left="calc({currentPosition}% + 10px - {currentPosition * 0.2}px)"
					>
						<div class="indicator-line"></div>
						<div class="indicator-label">{settings.rangeNm} nm</div>
					</div>
					<div class="tick-marks">
						{#each TICK_MARKS as tick (tick)}
							<div
								class="tick"
								class:active={tick === settings.rangeNm}
								style:left="{valuePosition(tick)}%"
							>
								<span class="tick-label">{tick}</span>
							</div>
						{/each}
					</div>
				</div>
				<button class="btn-icon" onclick={increaseRange} disabled={settings.rangeNm >= MAX_RANGE}>
					<Plus size={20} />
				</button>
			</div>
		</div>

		<!-- List button -->
		<button class="btn-action" onclick={onListClick}>
			<List size={24} />
		</button>

		<!-- Settings button -->
		<button class="btn-action" onclick={onSettingsClick}>
			<Settings size={24} />
		</button>
	</div>
</div>

<style>
	.ar-controls {
		position: fixed;
		bottom: 1rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 100;
		width: calc(100% - 2rem);
		max-width: 500px;
	}

	.controls-panel {
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(12px);
		border-radius: 1rem;
		padding: 1rem;
		display: flex;
		gap: 1rem;
		align-items: center;
	}

	.control-group {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.range-buttons {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.slider-container {
		flex: 1;
		position: relative;
		padding-bottom: 1.5rem;
		padding-top: 1.75rem;
	}

	.range-slider {
		width: 100%;
		height: 4px;
		background: rgba(255, 255, 255, 0.2);
		border-radius: 2px;
		outline: none;
		-webkit-appearance: none;
		appearance: none;
	}

	.range-slider::-webkit-slider-thumb {
		-webkit-appearance: none;
		appearance: none;
		width: 20px;
		height: 20px;
		background: rgb(var(--color-primary-500));
		border-radius: 50%;
		cursor: pointer;
	}

	/* Current value indicator */
	.current-indicator {
		position: absolute;
		top: 0;
		transform: translateX(-50%);
		display: flex;
		flex-direction: column;
		align-items: center;
		pointer-events: none;
	}

	.indicator-line {
		width: 2px;
		height: 50px;
		background: linear-gradient(to bottom, #22c55e, #22c55e 80%, transparent);
		border-radius: 1px;
	}

	.indicator-label {
		position: absolute;
		top: -2px;
		background: #22c55e;
		color: black;
		font-size: 0.75rem;
		font-weight: 700;
		padding: 2px 6px;
		border-radius: 4px;
		white-space: nowrap;
	}

	.tick-marks {
		position: absolute;
		top: calc(1.75rem + 8px);
		left: 10px;
		right: 10px;
		height: 20px;
	}

	.tick {
		position: absolute;
		transform: translateX(-50%);
		display: flex;
		flex-direction: column;
		align-items: center;
	}

	.tick::before {
		content: '';
		width: 1px;
		height: 6px;
		background: rgba(255, 255, 255, 0.4);
	}

	.tick.active::before {
		background: #22c55e;
		width: 2px;
	}

	.tick-label {
		color: rgba(255, 255, 255, 0.6);
		font-size: 0.625rem;
		margin-top: 2px;
	}

	.tick.active .tick-label {
		color: #22c55e;
		font-weight: 600;
	}

	.btn-icon {
		background: rgba(255, 255, 255, 0.2);
		border: none;
		border-radius: 0.5rem;
		padding: 0.5rem;
		color: white;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.btn-icon:disabled {
		opacity: 0.3;
		cursor: not-allowed;
	}

	.btn-action {
		background: rgba(255, 255, 255, 0.2);
		border: none;
		border-radius: 0.5rem;
		padding: 0.75rem;
		color: white;
		cursor: pointer;
		flex-shrink: 0;
	}

	.btn-action:active {
		background: rgba(255, 255, 255, 0.3);
	}
</style>
