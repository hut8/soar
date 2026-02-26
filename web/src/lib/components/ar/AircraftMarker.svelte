<script lang="ts">
	import type { ARAircraftPosition, ARScreenPosition } from '$lib/ar/types';

	let { aircraft, screenPosition, onclick } = $props<{
		aircraft: ARAircraftPosition;
		screenPosition: ARScreenPosition;
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

	// Scale crosshair based on distance (closer = larger)
	const crosshairSize = $derived(() => {
		if (aircraft.distance < 5) return 64;
		if (aircraft.distance < 15) return 56;
		if (aircraft.distance < 30) return 48;
		return 44;
	});
</script>

{#if screenPosition.visible}
	<button
		class="ar-marker"
		style:left="{screenPosition.x}px"
		style:top="{screenPosition.y}px"
		{onclick}
	>
		<!-- Crosshair reticle -->
		<svg
			class="crosshair"
			width={crosshairSize()}
			height={crosshairSize()}
			viewBox="0 0 64 64"
			fill="none"
			xmlns="http://www.w3.org/2000/svg"
		>
			<!-- Outer ring -->
			<circle cx="32" cy="32" r="28" stroke="white" stroke-width="2" opacity="0.8" />
			<circle cx="32" cy="32" r="28" stroke="cyan" stroke-width="1" opacity="0.6" />

			<!-- Crosshair lines (with gap in center) -->
			<!-- Top -->
			<line x1="32" y1="2" x2="32" y2="20" stroke="white" stroke-width="2" opacity="0.9" />
			<line x1="32" y1="2" x2="32" y2="20" stroke="cyan" stroke-width="1" opacity="0.5" />
			<!-- Bottom -->
			<line x1="32" y1="44" x2="32" y2="62" stroke="white" stroke-width="2" opacity="0.9" />
			<line x1="32" y1="44" x2="32" y2="62" stroke="cyan" stroke-width="1" opacity="0.5" />
			<!-- Left -->
			<line x1="2" y1="32" x2="20" y2="32" stroke="white" stroke-width="2" opacity="0.9" />
			<line x1="2" y1="32" x2="20" y2="32" stroke="cyan" stroke-width="1" opacity="0.5" />
			<!-- Right -->
			<line x1="44" y1="32" x2="62" y2="32" stroke="white" stroke-width="2" opacity="0.9" />
			<line x1="44" y1="32" x2="62" y2="32" stroke="cyan" stroke-width="1" opacity="0.5" />

			<!-- Center dot -->
			<circle cx="32" cy="32" r="3" fill="cyan" opacity="0.9" />
			<circle cx="32" cy="32" r="3" stroke="white" stroke-width="1" opacity="0.6" />

			<!-- Corner tick marks for extra visibility -->
			<!-- Top-left -->
			<line x1="10" y1="10" x2="16" y2="10" stroke="white" stroke-width="1.5" opacity="0.6" />
			<line x1="10" y1="10" x2="10" y2="16" stroke="white" stroke-width="1.5" opacity="0.6" />
			<!-- Top-right -->
			<line x1="48" y1="10" x2="54" y2="10" stroke="white" stroke-width="1.5" opacity="0.6" />
			<line x1="54" y1="10" x2="54" y2="16" stroke="white" stroke-width="1.5" opacity="0.6" />
			<!-- Bottom-left -->
			<line x1="10" y1="48" x2="10" y2="54" stroke="white" stroke-width="1.5" opacity="0.6" />
			<line x1="10" y1="54" x2="16" y2="54" stroke="white" stroke-width="1.5" opacity="0.6" />
			<!-- Bottom-right -->
			<line x1="54" y1="48" x2="54" y2="54" stroke="white" stroke-width="1.5" opacity="0.6" />
			<line x1="48" y1="54" x2="54" y2="54" stroke="white" stroke-width="1.5" opacity="0.6" />
		</svg>

		<!-- Info label below crosshair -->
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

	.crosshair {
		filter: drop-shadow(0 0 6px rgba(0, 0, 0, 1)) drop-shadow(0 0 2px rgba(0, 0, 0, 1))
			drop-shadow(0 0 12px rgba(0, 0, 0, 0.6));
		animation: crosshair-pulse 2s ease-in-out infinite;
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

	@keyframes crosshair-pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.7;
		}
	}
</style>
