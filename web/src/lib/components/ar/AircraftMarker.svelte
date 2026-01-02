<script lang="ts">
	import { Plane } from '@lucide/svelte';
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
	const distance = $derived(`${aircraft.distance.toFixed(1)}km`);
	const speed = $derived(
		aircraft.groundSpeedKnots ? `${Math.round(aircraft.groundSpeedKnots)}kt` : 'N/A'
	);
	const climb = $derived(
		aircraft.climbFpm
			? `${aircraft.climbFpm > 0 ? '+' : ''}${Math.round(aircraft.climbFpm)}fpm`
			: '0fpm'
	);
</script>

{#if screenPosition.visible}
	<button
		class="ar-marker"
		style:left="{screenPosition.x}px"
		style:top="{screenPosition.y}px"
		{onclick}
	>
		<!-- Aircraft icon -->
		<div class="marker-icon">
			<Plane size={20} class="text-primary-500" />
		</div>

		<!-- Info card -->
		<div class="marker-info">
			<div class="marker-registration">{registration}</div>
			<div class="marker-stats">
				<span>{altitude}</span>
				<span>•</span>
				<span>{distance}</span>
			</div>
			<div class="marker-stats">
				<span>{speed}</span>
				<span>•</span>
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
		transition: transform 0.1s ease-out;
		z-index: 10;
	}

	.ar-marker:active {
		transform: translate(-50%, -50%) scale(1.1);
	}

	.marker-icon {
		background: rgba(255, 255, 255, 0.95);
		backdrop-filter: blur(8px);
		border: 2px solid rgb(var(--color-primary-500));
		border-radius: 50%;
		padding: 0.5rem;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
	}

	.marker-info {
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(8px);
		color: white;
		padding: 0.375rem 0.625rem;
		border-radius: 0.5rem;
		font-size: 0.75rem;
		line-height: 1.2;
		text-align: center;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.5);
		min-width: 100px;
	}

	.marker-registration {
		font-weight: 700;
		font-size: 0.875rem;
		margin-bottom: 0.125rem;
	}

	.marker-stats {
		display: flex;
		gap: 0.25rem;
		justify-content: center;
		font-size: 0.625rem;
		opacity: 0.9;
	}
</style>
