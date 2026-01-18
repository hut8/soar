<script lang="ts">
	import { Navigation } from '@lucide/svelte';

	let { heading = 0 } = $props<{ heading: number }>();

	const compassDirections = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];

	// Track display rotation for smooth transitions around 0/360 boundary
	let displayRotation = $state(0);

	// Calculate shortest rotation path to avoid spinning around at 0/360 boundary
	$effect(() => {
		// Normalize incoming heading to 0-360
		const normalizedHeading = ((heading % 360) + 360) % 360;
		// Target rotation (negative because we rotate compass opposite to heading)
		const targetRotation = -normalizedHeading;

		// Calculate the delta from current display rotation
		let delta = targetRotation - displayRotation;

		// Normalize delta to -180 to 180 range for shortest path
		while (delta > 180) delta -= 360;
		while (delta < -180) delta += 360;

		// Apply the delta to get smooth rotation
		displayRotation = displayRotation + delta;
	});
</script>

<div class="compass-overlay">
	<div class="compass-ring" style:transform="rotate({displayRotation}deg)">
		<div class="compass-north">
			<Navigation size={28} class="text-primary-500" />
		</div>
		{#each compassDirections as direction, i (direction)}
			<div class="compass-label" style:transform="rotate({i * 45}deg)">
				<span class="label-text" style:transform="rotate({-displayRotation}deg)">{direction}</span>
			</div>
		{/each}
	</div>
	<div class="compass-heading">
		{Math.round(((heading % 360) + 360) % 360)}°
	</div>
</div>

<style>
	.compass-overlay {
		position: fixed;
		top: 4.5rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 90;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.5rem;
	}

	.compass-ring {
		position: relative;
		width: 120px;
		height: 120px;
		border: 3px solid rgba(255, 255, 255, 0.8);
		border-radius: 50%;
		background: rgba(0, 0, 0, 0.6);
		backdrop-filter: blur(8px);
		transition: transform 0.15s ease-out;
	}

	.compass-north {
		position: absolute;
		top: 6px;
		left: 50%;
		transform: translateX(-50%);
	}

	.compass-label {
		position: absolute;
		top: 50%;
		left: 50%;
		width: 0;
		height: 0;
		display: flex;
		justify-content: center;
		align-items: center;
		transform-origin: center center;
	}

	.label-text {
		position: absolute;
		top: -52px;
		color: white;
		font-weight: 600;
		font-size: 0.75rem;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
		white-space: nowrap;
	}

	.compass-heading {
		background: rgba(0, 0, 0, 0.85);
		color: white;
		padding: 0.5rem 1rem;
		border-radius: 0.5rem;
		font-weight: 700;
		font-size: 1.125rem;
		backdrop-filter: blur(8px);
	}
</style>
