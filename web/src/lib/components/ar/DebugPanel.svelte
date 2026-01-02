<script lang="ts">
	import type { ARDeviceOrientation, ARUserPosition } from '$lib/ar/types';

	let { position, orientation, aircraftCount } = $props<{
		position: ARUserPosition | null;
		orientation: ARDeviceOrientation | null;
		aircraftCount: number;
	}>();
</script>

<div class="debug-panel">
	<div class="debug-title">Debug Info</div>

	<div class="debug-section">
		<div class="debug-label">Position:</div>
		{#if position}
			<div class="debug-value">
				{position.latitude.toFixed(6)}, {position.longitude.toFixed(6)}
			</div>
			<div class="debug-value">
				Alt: {position.altitude.toFixed(1)}m (±{position.accuracy.toFixed(0)}m)
			</div>
		{:else}
			<div class="debug-value">No GPS lock</div>
		{/if}
	</div>

	<div class="debug-section">
		<div class="debug-label">Orientation:</div>
		{#if orientation}
			<div class="debug-value">
				Heading: {orientation.heading.toFixed(1)}° {orientation.absolute ? '(abs)' : '(rel)'}
			</div>
			<div class="debug-value">
				Pitch: {orientation.pitch.toFixed(1)}° Roll: {orientation.roll.toFixed(1)}°
			</div>
		{:else}
			<div class="debug-value">No orientation data</div>
		{/if}
	</div>

	<div class="debug-section">
		<div class="debug-label">Aircraft:</div>
		<div class="debug-value">{aircraftCount} visible</div>
	</div>
</div>

<style>
	.debug-panel {
		position: fixed;
		top: 180px;
		right: 1rem;
		background: rgba(0, 0, 0, 0.9);
		backdrop-filter: blur(8px);
		color: white;
		padding: 1rem;
		border-radius: 0.5rem;
		font-size: 0.75rem;
		font-family: monospace;
		z-index: 90;
		max-width: 300px;
	}

	.debug-title {
		font-weight: 700;
		margin-bottom: 0.5rem;
		font-size: 0.875rem;
	}

	.debug-section {
		margin-bottom: 0.75rem;
	}

	.debug-label {
		color: rgb(var(--color-primary-500));
		margin-bottom: 0.25rem;
	}

	.debug-value {
		opacity: 0.9;
		line-height: 1.4;
	}
</style>
