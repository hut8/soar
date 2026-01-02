<script lang="ts">
	import { Settings, Minus, Plus } from '@lucide/svelte';
	import type { ARSettings } from '$lib/ar/types';

	let { settings = $bindable(), onSettingsClick } = $props<{
		settings: ARSettings;
		onSettingsClick?: () => void;
	}>();

	function increaseRange() {
		settings.rangeKm = Math.min(100, settings.rangeKm + 5);
	}

	function decreaseRange() {
		settings.rangeKm = Math.max(5, settings.rangeKm - 5);
	}
</script>

<div class="ar-controls">
	<div class="controls-panel">
		<!-- Range control -->
		<div class="control-group">
			<label class="control-label">Range: {settings.rangeKm}km</label>
			<div class="range-buttons">
				<button class="btn-icon" onclick={decreaseRange} disabled={settings.rangeKm <= 5}>
					<Minus size={20} />
				</button>
				<input
					type="range"
					min="5"
					max="100"
					step="5"
					bind:value={settings.rangeKm}
					class="range-slider"
				/>
				<button class="btn-icon" onclick={increaseRange} disabled={settings.rangeKm >= 100}>
					<Plus size={20} />
				</button>
			</div>
		</div>

		<!-- Settings button -->
		<button class="btn-settings" onclick={onSettingsClick}>
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

	.control-label {
		color: white;
		font-size: 0.875rem;
		font-weight: 600;
	}

	.range-buttons {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.range-slider {
		flex: 1;
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

	.btn-settings {
		background: rgba(255, 255, 255, 0.2);
		border: none;
		border-radius: 0.5rem;
		padding: 0.75rem;
		color: white;
		cursor: pointer;
	}
</style>
