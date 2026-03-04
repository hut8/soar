<script lang="ts">
	import { Settings, List } from '@lucide/svelte';
	import type { ARSettings } from '$lib/ar/types';
	import ConnectionIndicator from './ConnectionIndicator.svelte';
	import RangeModal from './RangeModal.svelte';

	let {
		settings = $bindable(),
		onSettingsClick,
		onListClick
	} = $props<{
		settings: ARSettings;
		onSettingsClick?: () => void;
		onListClick?: () => void;
	}>();

	let showRangeModal = $state(false);
</script>

<div class="ar-controls">
	<div class="controls-panel">
		<!-- Range badge -->
		<button class="range-badge" onclick={() => (showRangeModal = true)}>
			{settings.rangeNm} nm
		</button>

		<!-- Connection indicator -->
		<ConnectionIndicator />

		<!-- List button -->
		<button class="btn-action" onclick={onListClick}>
			<List size={24} />
		</button>

		<!-- Settings button -->
		<button class="btn-action" onclick={onSettingsClick}>
			<Settings size={24} />
		</button>
	</div>
</div>

{#if showRangeModal}
	<RangeModal bind:rangeNm={settings.rangeNm} onClose={() => (showRangeModal = false)} />
{/if}

<style>
	.ar-controls {
		position: fixed;
		bottom: 0;
		left: 0;
		right: 0;
		z-index: 100;
	}

	.controls-panel {
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(12px);
		border-radius: 1rem 1rem 0 0;
		padding-top: 0.75rem;
		padding-bottom: calc(0.75rem + env(safe-area-inset-bottom, 0px));
		padding-left: calc(1.5rem + env(safe-area-inset-left, 0px));
		padding-right: calc(1.5rem + env(safe-area-inset-right, 0px));
		display: flex;
		gap: 0.75rem;
		align-items: center;
	}

	.range-badge {
		background: #22c55e;
		color: black;
		border: none;
		border-radius: 9999px;
		padding: 0.375rem 0.75rem;
		font-size: 0.8125rem;
		font-weight: 700;
		cursor: pointer;
		white-space: nowrap;
		flex-shrink: 0;
	}

	.range-badge:active {
		background: #16a34a;
	}

	.btn-action {
		background: rgba(255, 255, 255, 0.2);
		border: none;
		border-radius: 0.5rem;
		padding: 0.75rem;
		color: white;
		cursor: pointer;
		flex-shrink: 0;
	}

	.btn-action:active {
		background: rgba(255, 255, 255, 0.3);
	}
</style>
