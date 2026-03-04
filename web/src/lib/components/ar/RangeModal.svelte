<script lang="ts">
	import { Minus, Plus, X } from '@lucide/svelte';

	const MIN_RANGE = 10;
	const MAX_RANGE = 250;
	const STEP = 10;
	const TICK_MARKS = [10, 50, 100, 150, 200, 250];

	let { rangeNm = $bindable(), onClose } = $props<{
		rangeNm: number;
		onClose: () => void;
	}>();

	function increaseRange() {
		rangeNm = Math.min(MAX_RANGE, rangeNm + STEP);
	}

	function decreaseRange() {
		rangeNm = Math.max(MIN_RANGE, rangeNm - STEP);
	}

	const SLIDER_PADDING_PX = 10;
	const THUMB_WIDTH_PX = 20;
	const THUMB_FRACTION = THUMB_WIDTH_PX / 100;

	function valuePosition(value: number): number {
		return ((value - MIN_RANGE) / (MAX_RANGE - MIN_RANGE)) * 100;
	}

	const currentPosition = $derived(valuePosition(rangeNm));

	const indicatorLeftOffset = $derived(
		`calc(${currentPosition}% + ${SLIDER_PADDING_PX}px - ${currentPosition * THUMB_FRACTION}px)`
	);

	function handleBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) {
			onClose();
		}
	}

	function handleBackdropKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			onClose();
		}
	}
</script>

<div
	class="backdrop"
	role="dialog"
	aria-modal="true"
	aria-labelledby="range-modal-title"
	onclick={handleBackdropClick}
	onkeydown={handleBackdropKeydown}
>
	<div class="modal-panel">
		<div class="modal-header">
			<span id="range-modal-title" class="modal-title">Range</span>
			<button class="btn-close" onclick={onClose} aria-label="Close">
				<X size={18} />
			</button>
		</div>

		<div class="range-control">
			<button
				class="btn-icon"
				onclick={decreaseRange}
				disabled={rangeNm <= MIN_RANGE}
				aria-label="Decrease range"
			>
				<Minus size={20} />
			</button>
			<div class="slider-container">
				<input
					type="range"
					min={MIN_RANGE}
					max={MAX_RANGE}
					step={STEP}
					bind:value={rangeNm}
					class="range-slider"
					aria-label="Range in nautical miles"
				/>
				<div class="current-indicator" style:left={indicatorLeftOffset}>
					<div class="indicator-line"></div>
					<div class="indicator-label">{rangeNm} nm</div>
				</div>
				<div class="tick-marks">
					{#each TICK_MARKS as tick (tick)}
						<div class="tick" class:active={tick === rangeNm} style:left="{valuePosition(tick)}%">
							<span class="tick-label">{tick}</span>
						</div>
					{/each}
				</div>
			</div>
			<button
				class="btn-icon"
				onclick={increaseRange}
				disabled={rangeNm >= MAX_RANGE}
				aria-label="Increase range"
			>
				<Plus size={20} />
			</button>
		</div>
	</div>
</div>

<style>
	.backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.6);
		z-index: 200;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.modal-panel {
		background: rgba(30, 30, 30, 0.95);
		backdrop-filter: blur(16px);
		border-radius: 1rem;
		padding: 1.25rem;
		width: calc(100% - 3rem);
		max-width: 420px;
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 1rem;
	}

	.modal-title {
		color: white;
		font-size: 1rem;
		font-weight: 600;
	}

	.btn-close {
		background: rgba(255, 255, 255, 0.15);
		border: none;
		border-radius: 50%;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		cursor: pointer;
	}

	.range-control {
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
</style>
