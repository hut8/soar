<script lang="ts">
	import { ChevronUp, X } from '@lucide/svelte';
	import type { ARAircraftPosition, ARDeviceOrientation, ARSettings } from '$lib/ar/types';

	// Hysteresis factor to prevent flickering at FOV edges (0.9 = 90% of FOV edge)
	// Using a slightly smaller threshold than actual FOV prevents rapid on/off toggling
	const FOV_EDGE_HYSTERESIS = 0.9;

	type Direction =
		| 'in-view'
		| 'up'
		| 'down'
		| 'left'
		| 'right'
		| 'up-left'
		| 'up-right'
		| 'down-left'
		| 'down-right';

	let { targetAircraft, deviceOrientation, settings, onDismiss } = $props<{
		targetAircraft: ARAircraftPosition;
		deviceOrientation: ARDeviceOrientation;
		settings: ARSettings;
		onDismiss: () => void;
	}>();

	// Calculate relative bearing from device heading to aircraft
	// Normalize to -180 to 180
	const relativeBearing = $derived.by(() => {
		let bearing = targetAircraft.bearing - deviceOrientation.heading;
		while (bearing > 180) bearing -= 360;
		while (bearing < -180) bearing += 360;
		return bearing;
	});

	// Calculate adjusted elevation accounting for device pitch
	const adjustedElevation = $derived.by(() => {
		return targetAircraft.elevation - deviceOrientation.pitch;
	});

	// Determine which direction to show based on relative bearing and elevation
	// Use FOV settings for accurate thresholds
	// Supports diagonal directions when both horizontal and vertical are off-screen
	const direction = $derived.by((): Direction => {
		const rb = relativeBearing;
		const elev = adjustedElevation;
		const hFovHalf = settings.fovHorizontal / 2;
		const vFovHalf = settings.fovVertical / 2;

		// Add hysteresis to prevent flickering (use slightly smaller threshold for visibility)
		const hThreshold = hFovHalf * FOV_EDGE_HYSTERESIS;
		const vThreshold = vFovHalf * FOV_EDGE_HYSTERESIS;

		// Check if aircraft is within horizontal FOV
		const withinHorizontalFov = Math.abs(rb) <= hThreshold;
		// Check if aircraft is within vertical FOV
		const withinVerticalFov = Math.abs(elev) <= vThreshold;

		// If within both, aircraft should be visible
		if (withinHorizontalFov && withinVerticalFov) {
			return 'in-view';
		}

		// Determine horizontal and vertical direction components
		const horizontalDir = rb > 0 ? 'right' : 'left';
		const verticalDir = elev > 0 ? 'up' : 'down';

		// If within horizontal FOV but outside vertical, show up/down only
		if (withinHorizontalFov) {
			return verticalDir;
		}

		// If within vertical FOV but outside horizontal, show left/right only
		if (withinVerticalFov) {
			return horizontalDir;
		}

		// Both are outside FOV - show diagonal direction
		return `${verticalDir}-${horizontalDir}` as Direction;
	});

	// Calculate the arrow rotation angle for diagonal/angled directions
	// Returns angle in degrees (0 = up, 90 = right, etc.)
	const arrowAngle = $derived.by(() => {
		const rb = relativeBearing;
		const elev = adjustedElevation;

		// Calculate angle from center to target position
		// atan2 gives angle in radians, convert to degrees
		// Note: In our coordinate system, positive rb = right, positive elev = up
		// We want 0° to be "up", so we adjust the atan2 parameters
		const angleRad = Math.atan2(rb, elev);
		return angleRad * (180 / Math.PI);
	});

	// Calculate how far off-screen (for intensity of indicator)
	const intensity = $derived.by(() => {
		const rb = Math.abs(relativeBearing);
		const elev = Math.abs(adjustedElevation);
		const hFovHalf = settings.fovHorizontal / 2;
		const vFovHalf = settings.fovVertical / 2;

		// Use the larger of horizontal or vertical offset for intensity
		const hOffset = rb / hFovHalf;
		const vOffset = elev / vFovHalf;
		const maxOffset = Math.max(hOffset, vOffset);

		if (maxOffset < 1) return 0;
		if (maxOffset < 2) return 1;
		if (maxOffset < 3) return 2;
		return 3;
	});

	function formatDistance(nm: number): string {
		return `${nm.toFixed(1)} nm`;
	}
</script>

{#if direction !== 'in-view'}
	<div class="direction-indicator {direction}" class:intense={intensity >= 2}>
		<div class="indicator-content" style:--arrow-angle="{arrowAngle}deg">
			<!-- Use ChevronUp rotated to point in the actual direction -->
			<ChevronUp size={40} class="arrow rotated-arrow" />
			<ChevronUp size={40} class="arrow rotated-arrow arrow-2" />
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

	/* Cardinal directions - positioned at edges */
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

	/* Diagonal directions - positioned at corners */
	.direction-indicator.up-left {
		top: 7rem;
		left: 1rem;
		flex-direction: column;
	}

	.direction-indicator.up-right {
		top: 7rem;
		right: 1rem;
		flex-direction: column;
	}

	.direction-indicator.down-left {
		bottom: 8rem;
		left: 1rem;
		flex-direction: column-reverse;
	}

	.direction-indicator.down-right {
		bottom: 8rem;
		right: 1rem;
		flex-direction: column-reverse;
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

	/* Rotate arrows to point in the actual direction */
	.indicator-content :global(.rotated-arrow) {
		opacity: 0.9;
		transform: rotate(var(--arrow-angle, 0deg));
	}

	.indicator-content :global(.arrow-2) {
		position: absolute;
		opacity: 0.4;
		animation: arrow-pulse 1s ease-in-out infinite;
	}

	/* Secondary arrow offset - animates outward in the direction of the arrow */
	.indicator-content :global(.arrow-2.rotated-arrow) {
		/* Use transform to offset in the direction the arrow points */
		/* The arrow points "up" by default, so we translate along the rotated Y axis */
		transform: rotate(var(--arrow-angle, 0deg)) translateY(-8px);
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
