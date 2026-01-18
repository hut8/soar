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
</script>

<div class="compass-overlay">
	<div class="compass-ring" style:transform="rotate({displayRotation}deg)">
		<svg width="100" height="100" viewBox="0 0 100 100">
			<!-- Outer circle -->
			<circle
				cx="50"
				cy="50"
				r="48"
				fill="rgba(0, 0, 0, 0.7)"
				stroke="rgba(255, 255, 255, 0.8)"
				stroke-width="2"
			/>

			<!-- Cardinal direction markers -->
			<!-- North (red) -->
			<line
				x1="50"
				y1="4"
				x2="50"
				y2="16"
				stroke="#dc2626"
				stroke-width="3"
				stroke-linecap="round"
			/>
			<!-- North arrow -->
			<polygon points="50,6 54,14 50,12 46,14" fill="#dc2626" />

			<!-- South -->
			<line
				x1="50"
				y1="84"
				x2="50"
				y2="96"
				stroke="rgba(255, 255, 255, 0.6)"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- East -->
			<line
				x1="84"
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
				x2="16"
				y2="50"
				stroke="rgba(255, 255, 255, 0.6)"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- Intercardinal ticks (NE, SE, SW, NW) -->
			<line
				x1="74"
				y1="26"
				x2="80"
				y2="20"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
			<line
				x1="74"
				y1="74"
				x2="80"
				y2="80"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
			<line
				x1="26"
				y1="74"
				x2="20"
				y2="80"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>
			<line
				x1="26"
				y1="26"
				x2="20"
				y2="20"
				stroke="rgba(255, 255, 255, 0.4)"
				stroke-width="1.5"
				stroke-linecap="round"
			/>

			<!-- Direction labels that counter-rotate to stay upright -->
			<text
				x="50"
				y="26"
				text-anchor="middle"
				fill="#dc2626"
				font-weight="bold"
				font-size="14"
				transform="rotate({-displayRotation}, 50, 26)"
			>
				N
			</text>
			<text
				x="50"
				y="82"
				text-anchor="middle"
				fill="rgba(255, 255, 255, 0.8)"
				font-weight="600"
				font-size="11"
				transform="rotate({-displayRotation}, 50, 82)"
			>
				S
			</text>
			<text
				x="78"
				y="54"
				text-anchor="middle"
				fill="rgba(255, 255, 255, 0.8)"
				font-weight="600"
				font-size="11"
				transform="rotate({-displayRotation}, 78, 54)"
			>
				E
			</text>
			<text
				x="22"
				y="54"
				text-anchor="middle"
				fill="rgba(255, 255, 255, 0.8)"
				font-weight="600"
				font-size="11"
				transform="rotate({-displayRotation}, 22, 54)"
			>
				W
			</text>
		</svg>
	</div>
	<!-- Heading display inside the compass -->
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
