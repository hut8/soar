<script lang="ts">
	let { heading = 0 } = $props<{ heading: number }>();

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

	// Get the normalized heading for display
	const displayHeading = $derived(Math.round(((heading % 360) + 360) % 360));

	// Cardinal directions with their rotation positions
	const directions = [
		{ label: 'N', angle: 0, isNorth: true },
		{ label: 'NE', angle: 45, isNorth: false },
		{ label: 'E', angle: 90, isNorth: false },
		{ label: 'SE', angle: 135, isNorth: false },
		{ label: 'S', angle: 180, isNorth: false },
		{ label: 'SW', angle: 225, isNorth: false },
		{ label: 'W', angle: 270, isNorth: false },
		{ label: 'NW', angle: 315, isNorth: false }
	];
</script>

<div class="compass-overlay">
	<div class="compass-ring" style:transform="rotate({displayRotation}deg)">
		<!-- SVG for the compass circle and tick marks -->
		<svg width="100" height="100" viewBox="0 0 100 100" class="compass-svg">
			<!-- Outer circle -->
			<circle
				cx="50"
				cy="50"
				r="48"
				fill="rgba(0, 0, 0, 0.7)"
				stroke="rgba(255, 255, 255, 0.8)"
				stroke-width="2"
			/>

			<!-- Cardinal direction tick marks -->
			<!-- North (red) -->
			<line
				x1="50"
				y1="4"
				x2="50"
				y2="14"
				stroke="#dc2626"
				stroke-width="3"
				stroke-linecap="round"
			/>
			<!-- North arrow -->
			<polygon points="50,6 54,14 50,11 46,14" fill="#dc2626" />

			<!-- South -->
			<line
				x1="50"
				y1="86"
				x2="50"
				y2="96"
				stroke="rgba(255, 255, 255, 0.6)"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- East -->
			<line
				x1="86"
				y1="50"
				x2="96"
				y2="50"
				stroke="rgba(255, 255, 255, 0.6)"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- West -->
			<line
				x1="4"
				y1="50"
				x2="14"
				y2="50"
				stroke="rgba(255, 255, 255, 0.6)"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- Intercardinal ticks -->
			<line
				x1="82"
				y1="18"
				x2="88"
				y2="12"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
			<line
				x1="82"
				y1="82"
				x2="88"
				y2="88"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
			<line
				x1="18"
				y1="82"
				x2="12"
				y2="88"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
			<line
				x1="18"
				y1="18"
				x2="12"
				y2="12"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
		</svg>

		<!-- Direction labels using HTML for proper counter-rotation -->
		{#each directions as dir (dir.label)}
			<div class="direction-label" style:transform="rotate({dir.angle}deg)">
				<span
					class="label-text"
					class:north={dir.isNorth}
					style:transform="rotate({-displayRotation - dir.angle}deg)"
				>
					{dir.label}
				</span>
			</div>
		{/each}
	</div>

	<!-- Heading display inside the compass (doesn't rotate) -->
	<div class="heading-display">
		<span class="heading-value">{displayHeading}°</span>
	</div>
</div>

<style>
	.compass-overlay {
		position: fixed;
		top: 1rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 90;
		display: flex;
		flex-direction: column;
		align-items: center;
		filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.5));
	}

	.compass-ring {
		position: relative;
		width: 100px;
		height: 100px;
		transition: transform 0.15s ease-out;
	}

	.compass-svg {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
	}

	.direction-label {
		position: absolute;
		top: 50%;
		left: 50%;
		width: 0;
		height: 0;
		display: flex;
		justify-content: center;
		align-items: center;
	}

	.label-text {
		position: absolute;
		top: -42px;
		color: rgba(255, 255, 255, 0.9);
		font-weight: 600;
		font-size: 11px;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.8);
		white-space: nowrap;
	}

	.label-text.north {
		color: #dc2626;
		font-weight: bold;
		font-size: 13px;
	}

	.heading-display {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		text-align: center;
		pointer-events: none;
	}

	.heading-value {
		font-size: 18px;
		font-weight: bold;
		color: white;
		text-shadow: 0 1px 3px rgba(0, 0, 0, 0.8);
	}
</style>
