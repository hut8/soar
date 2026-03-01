<script lang="ts">
	import type { ARAircraftPosition, ARScreenPosition } from '$lib/ar/types';
	import {
		getIconShapeForCategory,
		getIconDefinition,
		getAltitudeColor
	} from '$lib/utils/aircraftIcons';

	let {
		aircraft,
		screenPosition,
		watched = false,
		rangeNm = 50,
		viewHeading = 0,
		onclick
	} = $props<{
		aircraft: ARAircraftPosition;
		screenPosition: ARScreenPosition;
		watched?: boolean;
		rangeNm?: number;
		viewHeading?: number;
		onclick?: () => void;
	}>();

	// Format values for display
	const registration = $derived(aircraft.registration || 'Unknown');
	const altitude = $derived(
		aircraft.altitudeFeet ? `${Math.round(aircraft.altitudeFeet)}ft` : 'N/A'
	);
	const distance = $derived(`${aircraft.distance.toFixed(1)} nm`);
	const speed = $derived(
		aircraft.groundSpeedKnots ? `${Math.round(aircraft.groundSpeedKnots)}kt` : 'N/A'
	);
	const climb = $derived(
		aircraft.climbFpm
			? `${aircraft.climbFpm > 0 ? '+' : ''}${Math.round(aircraft.climbFpm)}fpm`
			: '0fpm'
	);

	const MIN_ICON = 24;
	const MAX_ICON = 72;

	// Scale icon inversely proportional to distance within the current range
	const iconSize = $derived.by(() => {
		const t = Math.min(aircraft.distance / rangeNm, 1);
		return Math.round(MAX_ICON - t * (MAX_ICON - MIN_ICON));
	});

	// Get the aircraft icon shape and definition based on category
	const iconShape = $derived(
		getIconShapeForCategory(aircraft.aircraftCategory, aircraft.adsbEmitterCategory)
	);
	const iconDef = $derived(getIconDefinition(iconShape));

	// Altitude-based color (same gradient as the live map)
	const altColor = $derived(getAltitudeColor(aircraft.altitudeFeet));

	// Accent color: red for watched aircraft, altitude-based otherwise
	const fillColor = $derived(watched ? '#ef4444' : altColor);

	// Rotate the icon to show aircraft track relative to the camera's heading.
	// trackDegrees is absolute (0=north); subtracting viewHeading gives screen-relative rotation.
	const iconRotation = $derived(
		aircraft.trackDegrees != null ? aircraft.trackDegrees - viewHeading : 0
	);

	// Climb/descent indicator
	const climbIndicator = $derived.by(() => {
		if (!aircraft.climbFpm) return 'level';
		if (aircraft.climbFpm > 100) return 'climbing';
		if (aircraft.climbFpm < -100) return 'descending';
		return 'level';
	});
</script>

{#if screenPosition.visible}
	<button
		class="ar-marker"
		style:left="{screenPosition.x}px"
		style:top="{screenPosition.y}px"
		{onclick}
	>
		<!-- Aircraft icon -->
		<div class="icon-container" style:width="{iconSize}px" style:height="{iconSize}px">
			<svg
				class="aircraft-icon"
				width={iconSize}
				height={iconSize}
				viewBox={iconDef.viewBox}
				xmlns="http://www.w3.org/2000/svg"
				style:transform="rotate({iconRotation}deg)"
			>
				<path d={iconDef.path} fill={fillColor} stroke="white" stroke-width="1" />
			</svg>

			<!-- Climb/descent arrow -->
			{#if climbIndicator === 'climbing'}
				<div class="climb-arrow climbing">&#9650;</div>
			{:else if climbIndicator === 'descending'}
				<div class="climb-arrow descending">&#9660;</div>
			{/if}
		</div>

		<!-- Info label below icon -->
		<div class="marker-info">
			<div class="marker-registration">{registration}</div>
			<div class="marker-stats">
				<span>{altitude}</span>
				<span class="sep">&bull;</span>
				<span>{distance}</span>
			</div>
			<div class="marker-stats">
				<span>{speed}</span>
				<span class="sep">&bull;</span>
				<span>{climb}</span>
			</div>
		</div>
	</button>
{/if}

<style>
	.ar-marker {
		position: absolute;
		transform: translate(-50%, -50%);
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.25rem;
		cursor: pointer;
		z-index: 10;
		background: none;
		border: none;
		padding: 0;
		color: white;
	}

	.ar-marker:active {
		transform: translate(-50%, -50%) scale(1.1);
	}

	.icon-container {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.aircraft-icon {
		filter: drop-shadow(0 0 4px rgba(0, 0, 0, 1)) drop-shadow(0 0 8px rgba(0, 0, 0, 0.6));
	}

	.climb-arrow {
		position: absolute;
		right: -12px;
		font-size: 0.625rem;
		text-shadow:
			0 0 4px rgba(0, 0, 0, 1),
			0 0 2px rgba(0, 0, 0, 1);
	}

	.climb-arrow.climbing {
		top: -2px;
		color: #4ade80;
	}

	.climb-arrow.descending {
		bottom: -2px;
		color: #f87171;
	}

	.marker-info {
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(8px);
		color: white;
		padding: 0.25rem 0.5rem;
		border-radius: 0.375rem;
		font-size: 0.6875rem;
		line-height: 1.2;
		text-align: center;
		border: 1px solid rgba(255, 255, 255, 0.3);
		white-space: nowrap;
		box-shadow:
			0 2px 8px rgba(0, 0, 0, 0.8),
			0 0 4px rgba(0, 0, 0, 0.6);
	}

	.marker-registration {
		font-weight: 700;
		font-size: 0.8125rem;
		margin-bottom: 0.0625rem;
	}

	.marker-stats {
		display: flex;
		gap: 0.25rem;
		justify-content: center;
		font-size: 0.625rem;
		opacity: 0.9;
	}

	.sep {
		opacity: 0.5;
	}
</style>
