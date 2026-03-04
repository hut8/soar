<script lang="ts">
	import { Settings, List, MapPin } from '@lucide/svelte';
	import type { ARSettings } from '$lib/ar/types';
	import ConnectionIndicator from '$lib/components/ar/ConnectionIndicator.svelte';
	import RangeModal from '$lib/components/ar/RangeModal.svelte';

	let {
		settings = $bindable(),
		onSettingsClick,
		onListClick,
		onLocationClick
	} = $props<{
		settings: ARSettings;
		onSettingsClick?: () => void;
		onListClick?: () => void;
		onLocationClick?: () => void;
	}>();

	let showRangeModal = $state(false);
</script>

<div class="spotter-controls">
	<div class="controls-panel">
		<!-- Range badge -->
		<button class="range-badge" onclick={() => (showRangeModal = true)}>
			{settings.rangeNm} nm
		</button>

		<!-- Connection indicator -->
		<ConnectionIndicator />

		<!-- Location button -->
		<button class="btn-action" onclick={onLocationClick} title="Change location">
			<MapPin size={24} />
		</button>

		<!-- List button -->
		<button class="btn-action" onclick={onListClick} title="Aircraft list">
			<List size={24} />
		</button>

		<!-- Settings button -->
		<button class="btn-action" onclick={onSettingsClick} title="Debug info">
			<Settings size={24} />
		</button>
	</div>
</div>

{#if showRangeModal}
	<RangeModal bind:rangeNm={settings.rangeNm} onClose={() => (showRangeModal = false)} />
{/if}

<style>
	.spotter-controls {
		position: fixed;
		bottom: 1rem;
		left: 50%;
		transform: translateX(-50%);
		z-index: 100;
	}

	.controls-panel {
		background: rgba(0, 0, 0, 0.85);
		backdrop-filter: blur(12px);
		border-radius: 1rem;
		padding: 0.75rem;
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
