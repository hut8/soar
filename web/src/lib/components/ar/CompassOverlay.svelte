<script lang="ts">
	import { Navigation } from '@lucide/svelte';

	let { heading = 0 } = $props<{ heading: number }>();

	const compassDirections = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
</script>

<div class="compass-overlay">
	<div class="compass-ring" style:transform="rotate({-heading}deg)">
		<div class="compass-north">
			<Navigation size={32} class="text-primary-500" />
		</div>
		{#each compassDirections as direction, i (direction)}
			<div class="compass-label" style:transform="rotate({i * 45}deg) translateY(-60px)">
				<span style:transform="rotate({heading}deg)">{direction}</span>
			</div>
		{/each}
	</div>
	<div class="compass-heading">
		{Math.round(heading)}°
	</div>
</div>

<style>
	.compass-overlay {
		position: fixed;
		top: 1rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 100;
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
		transition: transform 0.3s ease-out;
	}

	.compass-north {
		position: absolute;
		top: 8px;
		left: 50%;
		transform: translateX(-50%);
	}

	.compass-label {
		position: absolute;
		top: 50%;
		left: 50%;
		color: white;
		font-weight: 600;
		font-size: 0.875rem;
		display: flex;
		justify-content: center;
		align-items: center;
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
