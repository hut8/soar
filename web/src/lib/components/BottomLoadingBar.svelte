<script lang="ts">
	import { loading } from '$lib/stores/loading';
	import { fade } from 'svelte/transition';

	// Reactive subscription to loading state
	let isLoading = $derived($loading.activeRequests > 0);
</script>

{#if isLoading}
	<div class="bottom-loading-bar" transition:fade={{ duration: 300 }}></div>
{/if}

<style>
	.bottom-loading-bar {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		height: 10px;
		pointer-events: none;
		z-index: 9999;
		animation: slide 2.5s ease-in-out infinite;
	}

	/* Light mode - dramatic blue to orange gradient */
	.bottom-loading-bar {
		background: linear-gradient(
			90deg,
			rgba(59, 130, 246, 0.7) 0%,
			rgba(251, 146, 60, 0.7) 50%,
			rgba(59, 130, 246, 0.7) 100%
		);
		background-size: 200% 100%;
	}

	/* Dark mode - dramatic blue to orange gradient */
	:global(.dark) .bottom-loading-bar {
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
