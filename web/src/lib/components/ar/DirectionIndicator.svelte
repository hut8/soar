<script lang="ts">
	import { ChevronUp, ChevronDown, ChevronLeft, ChevronRight, X } from '@lucide/svelte';
	import type { ARAircraftPosition, ARDeviceOrientation, ARSettings } from '$lib/ar/types';

	let { targetAircraft, deviceOrientation, settings, onDismiss } = $props<{
		targetAircraft: ARAircraftPosition;
		deviceOrientation: ARDeviceOrientation;
		settings: ARSettings;
		onDismiss: () => void;
	}>();

	// Calculate relative bearing from device heading to aircraft
	// Normalize to -180 to 180
	const relativeBearing = $derived(() => {
		let bearing = targetAircraft.bearing - deviceOrientation.heading;
		while (bearing > 180) bearing -= 360;
		while (bearing < -180) bearing += 360;
		return bearing;
	});

	// Calculate adjusted elevation accounting for device pitch
	const adjustedElevation = $derived(() => {
		return targetAircraft.elevation - deviceOrientation.pitch;
	});

	// Determine which direction to show based on relative bearing and elevation
	// Use FOV settings for accurate thresholds
	const direction = $derived(() => {
		const rb = relativeBearing();
		const elev = adjustedElevation();
		const hFovHalf = settings.fovHorizontal / 2;
		const vFovHalf = settings.fovVertical / 2;

		// Add hysteresis to prevent flickering (use slightly larger threshold for "exit")
		const hThreshold = hFovHalf * 0.9; // Slightly inside FOV edge
		const vThreshold = vFovHalf * 0.9;

		// Check if aircraft is within horizontal FOV
		const withinHorizontalFov = Math.abs(rb) <= hThreshold;
		// Check if aircraft is within vertical FOV
		const withinVerticalFov = Math.abs(elev) <= vThreshold;

		// If within both, aircraft should be visible
		if (withinHorizontalFov && withinVerticalFov) {
			return 'in-view';
		}

		// If within horizontal FOV but outside vertical, show up/down
		if (withinHorizontalFov) {
			return elev > 0 ? 'up' : 'down';
		}

		// Otherwise show left/right
		return rb > 0 ? 'right' : 'left';
	});

	// Calculate how far off-screen (for intensity of indicator)
	const intensity = $derived(() => {
		const rb = Math.abs(relativeBearing());
		const hFovHalf = settings.fovHorizontal / 2;
		if (rb < hFovHalf) return 0;
		if (rb < hFovHalf * 2) return 1;
		if (rb < hFovHalf * 3) return 2;
		return 3;
	});

	function formatDistance(nm: number): string {
		return `${nm.toFixed(1)} nm`;
	}
</script>

{#if direction() !== 'in-view'}
	<div class="direction-indicator {direction()}" class:intense={intensity() >= 2}>
		<div class="indicator-content">
			{#if direction() === 'up'}
				<ChevronUp size={40} class="arrow" />
				<ChevronUp size={40} class="arrow arrow-2" />
			{:else if direction() === 'down'}
				<ChevronDown size={40} class="arrow" />
				<ChevronDown size={40} class="arrow arrow-2" />
			{:else if direction() === 'left'}
				<ChevronLeft size={40} class="arrow" />
				<ChevronLeft size={40} class="arrow arrow-2" />
			{:else if direction() === 'right'}
				<ChevronRight size={40} class="arrow" />
				<ChevronRight size={40} class="arrow arrow-2" />
			{/if}
		</div>

		<div class="target-info">
			<span class="target-reg">{targetAircraft.registration || 'Unknown'}</span>
			<span class="target-distance">{formatDistance(targetAircraft.distance)}</span>
		</div>

		<button class="btn-dismiss" onclick={onDismiss}>
			<X size={16} />
		</button>
	</div>
{/if}

<style>
	.direction-indicator {
		position: fixed;
		z-index: 150;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		background: rgba(var(--color-primary-500), 0.9);
		backdrop-filter: blur(8px);
		padding: 0.5rem 0.75rem;
		border-radius: 0.75rem;
		color: white;
		animation: pulse-glow 1.5s ease-in-out infinite;
	}

	.direction-indicator.up {
		top: 7rem;
		left: 50%;
		transform: translateX(-50%);
		flex-direction: column;
	}

	.direction-indicator.down {
		bottom: 8rem;
		left: 50%;
		transform: translateX(-50%);
		flex-direction: column-reverse;
	}

	.direction-indicator.left {
		left: 1rem;
		top: 50%;
		transform: translateY(-50%);
		flex-direction: row;
	}

	.direction-indicator.right {
		right: 1rem;
		top: 50%;
		transform: translateY(-50%);
		flex-direction: row-reverse;
	}

	.direction-indicator.intense {
		background: rgba(var(--color-primary-500), 1);
	}

	.indicator-content {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.indicator-content :global(.arrow) {
		opacity: 0.9;
	}

	.indicator-content :global(.arrow-2) {
		position: absolute;
		opacity: 0.4;
		animation: arrow-pulse 1s ease-in-out infinite;
	}

	.direction-indicator.up :global(.arrow-2) {
		transform: translateY(-8px);
	}

	.direction-indicator.down :global(.arrow-2) {
		transform: translateY(8px);
	}

	.direction-indicator.left :global(.arrow-2) {
		transform: translateX(-8px);
	}

	.direction-indicator.right :global(.arrow-2) {
		transform: translateX(8px);
	}

	.target-info {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
	}

	.direction-indicator.left .target-info,
	.direction-indicator.right .target-info {
		align-items: flex-start;
	}

	.target-reg {
		font-weight: 700;
		font-size: 0.875rem;
	}

	.target-distance {
		font-size: 0.75rem;
		opacity: 0.9;
	}

	.btn-dismiss {
		position: absolute;
		top: -0.5rem;
		right: -0.5rem;
		background: rgba(0, 0, 0, 0.6);
		border: none;
		border-radius: 50%;
		width: 24px;
		height: 24px;
		display: flex;
		align-items: center;
		justify-content: center;
		color: white;
		cursor: pointer;
	}

	@keyframes pulse-glow {
		0%,
		100% {
			box-shadow: 0 0 10px rgba(var(--color-primary-500), 0.5);
		}
		50% {
			box-shadow: 0 0 20px rgba(var(--color-primary-500), 0.8);
		}
	}

	@keyframes arrow-pulse {
		0%,
		100% {
			opacity: 0.2;
		}
		50% {
			opacity: 0.6;
		}
	}
</style>
