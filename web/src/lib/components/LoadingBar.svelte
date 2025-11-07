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
		width: 100%;
		height: 100%;
		pointer-events: none;
		z-index: 1;
		animation: slide 2.5s ease-in-out infinite;
	}

	/* Light mode - dramatic blue to orange gradient */
	.loading-bar-overlay {
		background: linear-gradient(
			90deg,
			rgba(59, 130, 246, 0.7) 0%,
			rgba(251, 146, 60, 0.7) 50%,
			rgba(59, 130, 246, 0.7) 100%
		);
		background-size: 200% 100%;
	}

	/* Dark mode - dramatic blue to orange gradient */
	:global(.dark) .loading-bar-overlay {
		background: linear-gradient(
			90deg,
			rgba(59, 130, 246, 0.6) 0%,
			rgba(251, 146, 60, 0.6) 50%,
			rgba(59, 130, 246, 0.6) 100%
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
