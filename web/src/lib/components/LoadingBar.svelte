<script lang="ts">
	import { loading } from '$lib/stores/loading';

	// Reactive subscription to loading state
	let isLoading = $derived($loading.activeRequests > 0);
</script>

{#if isLoading}
	<div class="loading-bar-overlay"></div>
{/if}

<style>
	.loading-bar-overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		pointer-events: none;
		animation: fadeIn 150ms ease-in;
		z-index: -1;
	}

	/* Light mode - soft blue gradient */
	.loading-bar-overlay {
		background: linear-gradient(
			90deg,
			rgba(96, 165, 250, 0.15) 0%,
			rgba(147, 197, 253, 0.25) 50%,
			rgba(96, 165, 250, 0.15) 100%
		);
		background-size: 200% 100%;
		animation:
			slide 2s ease-in-out infinite,
			fadeIn 150ms ease-in;
	}

	/* Dark mode - subtle cyan/blue gradient */
	:global(.dark) .loading-bar-overlay {
		background: linear-gradient(
			90deg,
			rgba(34, 211, 238, 0.08) 0%,
			rgba(56, 189, 248, 0.12) 50%,
			rgba(34, 211, 238, 0.08) 100%
		);
		background-size: 200% 100%;
	}

	@keyframes slide {
		0% {
			background-position: 100% 0;
		}
		100% {
			background-position: -100% 0;
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
</style>
