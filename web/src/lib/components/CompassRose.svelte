<script lang="ts">
	/**
	 * CompassRose - A visual compass indicator for map orientation
	 *
	 * Displays a rotating compass rose that shows the device's magnetic heading.
	 * The rose rotates so that north always points north on the display.
	 */

	interface Props {
		/** The rotation angle for the compass rose (opposite of device heading) */
		rotation: number;
		/** The magnetic heading to display in degrees (0-359) */
		displayHeading: number;
	}

	let { rotation, displayHeading }: Props = $props();
</script>

<div class="compass-container">
	<div class="compass-rose" style="transform: rotate({rotation}deg)">
		<svg width="80" height="80" viewBox="0 0 80 80">
			<!-- Outer circle -->
			<circle
				cx="40"
				cy="40"
				r="38"
				fill="rgba(255, 255, 255, 0.9)"
				stroke="rgba(0, 0, 0, 0.3)"
				stroke-width="2"
			/>

			<!-- Cardinal direction markers -->
			<!-- North -->
			<line
				x1="40"
				y1="4"
				x2="40"
				y2="16"
				stroke="#dc2626"
				stroke-width="3"
				stroke-linecap="round"
			/>

			<!-- South -->
			<line
				x1="40"
				y1="64"
				x2="40"
				y2="76"
				stroke="#374151"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- East -->
			<line
				x1="64"
				y1="40"
				x2="76"
				y2="40"
				stroke="#374151"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- West -->
			<line
				x1="4"
				y1="40"
				x2="16"
				y2="40"
				stroke="#374151"
				stroke-width="2"
				stroke-linecap="round"
			/>

			<!-- Intercardinal directions -->
			<!-- Northeast -->
			<line
				x1="56.57"
				y1="23.43"
				x2="64.85"
				y2="15.15"
				stroke="#6b7280"
				stroke-width="1.5"
				stroke-linecap="round"
			/>

			<!-- Southeast -->
			<line
				x1="56.57"
				y1="56.57"
				x2="64.85"
				y2="64.85"
				stroke="#6b7280"
				stroke-width="1.5"
				stroke-linecap="round"
			/>

			<!-- Southwest -->
			<line
				x1="23.43"
				y1="56.57"
				x2="15.15"
				y2="64.85"
				stroke="#6b7280"
				stroke-width="1.5"
				stroke-linecap="round"
			/>

			<!-- Northwest -->
			<line
				x1="23.43"
				y1="23.43"
				x2="15.15"
				y2="15.15"
				stroke="#6b7280"
				stroke-width="1.5"
				stroke-linecap="round"
			/>

			<!-- North arrow (pointing up) -->
			<polygon points="40,8 44,18 40,16 36,18" fill="#dc2626" stroke="#dc2626" stroke-width="1" />
		</svg>
	</div>
	<div class="heading-display">
		<div class="heading-value">{displayHeading}°</div>
		<div class="heading-label">Magnetic</div>
	</div>
</div>

<style>
	.compass-container {
		position: relative;
		filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.3));
	}

	.compass-rose {
		transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
	}

	.heading-display {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		text-align: center;
	}

	.heading-value {
		font-size: 24px;
		font-weight: bold;
		color: #374151;
	}

	.heading-label {
		font-size: 10px;
		font-weight: 600;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: #6b7280;
	}
</style>
