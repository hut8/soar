<script lang="ts">
	import { loading } from '$lib/stores/loading';
	import { fade } from 'svelte/transition';

	// Reactive subscription to loading state
	let isLoading = $derived($loading.activeRequests > 0);
</script>

{#if isLoading}
	<div class="loading-bar-overlay" transition:fade={{ duration: 300 }}></div>
{/if}

<style>
	.loading-bar-overlay {
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		pointer-events: none;
		z-index: 1;
		animation: slide 2.5s ease-in-out infinite;
	}

	/* Light mode - blue to orange gradient */
	.loading-bar-overlay {
		background: linear-gradient(
			90deg,
			rgba(59, 130, 246, 0.3) 0%,
			rgba(251, 146, 60, 0.3) 50%,
			rgba(59, 130, 246, 0.3) 100%
		);
		background-size: 200% 100%;
	}

	/* Dark mode - blue to orange gradient with adjusted opacity */
	:global(.dark) .loading-bar-overlay {
		background: linear-gradient(
			90deg,
			rgba(59, 130, 246, 0.25) 0%,
			rgba(251, 146, 60, 0.25) 50%,
			rgba(59, 130, 246, 0.25) 100%
		);
		background-size: 200% 100%;
	}

	@keyframes slide {
		0% {
			background-position: 0% 0;
		}
		50% {
			background-position: 100% 0;
		}
		100% {
			background-position: 0% 0;
		}
	}
</style>
