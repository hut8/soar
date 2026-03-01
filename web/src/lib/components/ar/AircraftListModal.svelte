<script lang="ts">
	import { X, Plane, ArrowUp } from '@lucide/svelte';
	import type { ARAircraftPosition } from '$lib/ar/types';

	let {
		aircraft,
		onSelect,
		onClose,
		watchedIds = new Set()
	} = $props<{
		aircraft: ARAircraftPosition[];
		onSelect: (aircraft: ARAircraftPosition) => void;
		onClose: () => void;
		watchedIds?: Set<string>;
	}>();

	// Sort by distance and take nearest 10
	const nearestAircraft = $derived(
		[...aircraft].sort((a, b) => a.distance - b.distance).slice(0, 10)
	);

	function formatAltitude(feet: number): string {
		if (feet >= 1000) {
			return `${(feet / 1000).toFixed(1)}k ft`;
		}
		return `${Math.round(feet)} ft`;
	}

	function formatDistance(nm: number): string {
		return `${nm.toFixed(1)} nm`;
	}

	function formatBearing(bearing: number): string {
		const directions = ['N', 'NE', 'E', 'SE', 'S', 'SW', 'W', 'NW'];
		const index = Math.round(bearing / 45) % 8;
		return `${Math.round(bearing)}° ${directions[index]}`;
	}

	function handleSelect(ac: ARAircraftPosition) {
		onSelect(ac);
	}
</script>

<div class="modal-backdrop" onclick={onClose} role="presentation">
	<div
		class="modal-content"
		onclick={(e) => e.stopPropagation()}
		role="dialog"
		aria-labelledby="aircraft-list-title"
	>
		<div class="modal-header">
			<h2 id="aircraft-list-title">Nearby Aircraft</h2>
			<button class="btn-close" onclick={onClose}>
				<X size={20} />
			</button>
		</div>

		<div class="aircraft-list">
			{#if nearestAircraft.length === 0}
				<div class="empty-state">
					<Plane size={32} class="opacity-50" />
					<p>No aircraft in range</p>
				</div>
			{:else}
				{#each nearestAircraft as ac (ac.aircraftId)}
					<button
						class="aircraft-item"
						class:watched={watchedIds.has(ac.aircraftId)}
						onclick={() => handleSelect(ac)}
					>
						<div class="aircraft-icon">
							<Plane size={20} />
						</div>
						<div class="aircraft-info">
							<div class="aircraft-reg">{ac.registration || 'Unknown'}</div>
							{#if ac.clubName}
								<div class="aircraft-club">{ac.clubName}</div>
							{/if}
							<div class="aircraft-details">
								<span>{formatAltitude(ac.altitudeFeet)}</span>
								<span class="separator">•</span>
								<span>{formatDistance(ac.distance)}</span>
								<span class="separator">•</span>
								<span class="bearing">
									<ArrowUp
										size={12}
										style="transform: rotate({ac.bearing}deg); display: inline-block;"
									/>
									{formatBearing(ac.bearing)}
								</span>
							</div>
						</div>
						<div class="aircraft-distance">
							{formatDistance(ac.distance)}
						</div>
					</button>
				{/each}
			{/if}
		</div>
	</div>
</div>

<style>
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgba(0, 0, 0, 0.7);
		backdrop-filter: blur(4px);
		z-index: 200;
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 1rem;
	}

	.modal-content {
		background: rgba(20, 20, 20, 0.95);
		backdrop-filter: blur(12px);
		border-radius: 1rem;
		width: 100%;
		max-width: 400px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.1);
	}

	.modal-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 1rem;
		border-bottom: 1px solid rgba(255, 255, 255, 0.1);
	}

	.modal-header h2 {
		color: white;
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	.btn-close {
		background: rgba(255, 255, 255, 0.1);
		border: none;
		border-radius: 0.5rem;
		padding: 0.5rem;
		color: white;
		cursor: pointer;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.btn-close:active {
		background: rgba(255, 255, 255, 0.2);
	}

	.aircraft-list {
		overflow-y: auto;
		flex: 1;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 1rem;
		color: rgba(255, 255, 255, 0.5);
		gap: 0.5rem;
	}

	.aircraft-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem 1rem;
		width: 100%;
		background: transparent;
		border: none;
		border-bottom: 1px solid rgba(255, 255, 255, 0.05);
		color: white;
		cursor: pointer;
		text-align: left;
	}

	.aircraft-item.watched {
		background: rgba(239, 68, 68, 0.15);
		border-left: 3px solid #ef4444;
	}

	.aircraft-item:active {
		background: rgba(255, 255, 255, 0.1);
	}

	.aircraft-item:last-child {
		border-bottom: none;
	}

	.aircraft-icon {
		flex-shrink: 0;
		width: 36px;
		height: 36px;
		background: rgba(var(--color-primary-500), 0.2);
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		color: rgb(var(--color-primary-500));
	}

	.aircraft-item.watched .aircraft-icon {
		background: rgba(239, 68, 68, 0.2);
		color: #ef4444;
	}

	.aircraft-info {
		flex: 1;
		min-width: 0;
	}

	.aircraft-reg {
		font-weight: 600;
		font-size: 0.9375rem;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.aircraft-club {
		font-size: 0.6875rem;
		color: rgba(255, 255, 255, 0.5);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.aircraft-details {
		font-size: 0.75rem;
		color: rgba(255, 255, 255, 0.6);
		display: flex;
		align-items: center;
		gap: 0.375rem;
		margin-top: 0.125rem;
	}

	.separator {
		opacity: 0.4;
	}

	.bearing {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.aircraft-distance {
		flex-shrink: 0;
		font-size: 0.875rem;
		font-weight: 600;
		color: rgb(var(--color-primary-500));
	}
</style>
