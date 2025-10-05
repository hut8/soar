<script lang="ts">
	import { loading } from '$lib/stores/loading';

	// Reactive subscription to loading state
	let isLoading = $derived($loading.activeRequests > 0);
</script>

{#if isLoading}
	<div class="loading-bar-container">
		<div class="loading-bar"></div>
	</div>
{/if}

<style>
	.loading-bar-container {
		position: absolute;
		bottom: 0;
		left: 0;
		right: 0;
		height: 3px;
		overflow: hidden;
		animation: fadeIn 150ms ease-in;
	}

	.loading-bar {
		width: 100%;
		height: 100%;
		background: linear-gradient(90deg, #3b82f6 0%, #f97316 50%, #3b82f6 100%);
		background-size: 200% 100%;
		animation: slide 1.5s ease-in-out infinite;
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

	/* Svelte's transition will handle fade out */
</style>
