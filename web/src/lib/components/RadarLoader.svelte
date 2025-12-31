<script lang="ts">
	import { loading } from '$lib/stores/loading';

	// Reactive subscription to loading state
	let isLoading = $derived($loading.activeRequests > 0);

	// Debounced visibility state - stays visible for at least 1 second
	let showRadar = $state(false);
	let hideTimer: ReturnType<typeof setTimeout> | null = null;

	$effect(() => {
		if (isLoading) {
			// Show radar immediately when loading starts
			showRadar = true;

			// Cancel any pending hide timer
			if (hideTimer) {
				clearTimeout(hideTimer);
				hideTimer = null;
			}
		} else {
			// When loading stops, wait at least 1 second before hiding
			if (hideTimer) {
				clearTimeout(hideTimer);
			}

			hideTimer = setTimeout(() => {
				showRadar = false;
				hideTimer = null;
			}, 1000); // 1 second delay
		}

		// Cleanup on unmount
		return () => {
			if (hideTimer) {
				clearTimeout(hideTimer);
			}
		};
	});
</script>

{#if showRadar}
	<div class="radar-scope">
		<svg viewBox="0 0 24 24" class="radar-svg">
			<!-- Radar circles -->
			<circle cx="12" cy="12" r="10" class="radar-ring" />
			<circle cx="12" cy="12" r="7" class="radar-ring" />
			<circle cx="12" cy="12" r="4" class="radar-ring" />
			<!-- Center dot -->
			<circle cx="12" cy="12" r="1.5" class="radar-center" />
			<!-- Sweeping line -->
			<line x1="12" y1="12" x2="12" y2="2" class="radar-sweep" />
		</svg>
	</div>
{/if}

<style>
	.radar-scope {
		width: 20px;
		height: 20px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.radar-svg {
		width: 100%;
		height: 100%;
		animation: fadeIn 150ms ease-in;
	}

	.radar-ring {
		fill: none;
		stroke: currentColor;
		stroke-width: 0.5;
		opacity: 0.3;
	}

	.radar-center {
		fill: currentColor;
		opacity: 0.8;
	}

	.radar-sweep {
		stroke: currentColor;
		stroke-width: 1.5;
		stroke-linecap: round;
		transform-origin: center;
		animation: sweep 2s linear infinite;
		opacity: 0.9;
	}

	@keyframes sweep {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes fadeIn {
		from {
			opacity: 0;
		}
		to {
			opacity: 1;
		}
	}

	/* Light mode - use darker colors for visibility on light backgrounds */
	:global(:not(.dark)) .radar-ring,
	:global(:not(.dark)) .radar-center,
	:global(:not(.dark)) .radar-sweep {
		color: rgb(21, 94, 117); /* Darker cyan for light mode */
	}

	/* Dark mode - use brighter colors */
	:global(.dark) .radar-ring,
	:global(.dark) .radar-center,
	:global(.dark) .radar-sweep {
		color: rgb(103, 232, 249); /* Bright cyan for dark mode */
	}
</style>
