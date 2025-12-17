<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Viewer, Entity } from 'cesium';
	import { Math as CesiumMath } from 'cesium';
	import { serverCall } from '$lib/api/server';
	import { createReceiverEntity } from '$lib/cesium/entities';
	import type { Receiver } from '$lib/types';

	// Props
	let { viewer, enabled = $bindable(true) }: { viewer: Viewer; enabled?: boolean } = $props();

	// State
	let receiverEntities = $state<Map<string, Entity>>(new Map()); // Map of receiver ID -> Entity
	let isLoading = $state(false);

	// Debounce timer
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Maximum camera height to show receivers (in meters)
	const MAX_CAMERA_HEIGHT = 100000; // 100km

	/**
	 * Check if camera is zoomed in enough to show receivers
	 */
	function shouldShowReceivers(): boolean {
		const cameraHeight = viewer.camera.positionCartographic.height;
		return cameraHeight < MAX_CAMERA_HEIGHT;
	}

	/**
	 * Get current camera viewport bounds
	 */
	function getVisibleBounds(): {
		latMin: number;
		latMax: number;
		lonMin: number;
		lonMax: number;
	} | null {
		try {
			const rectangle = viewer.camera.computeViewRectangle();
			if (!rectangle) return null;

			return {
				latMin: CesiumMath.toDegrees(rectangle.south),
				latMax: CesiumMath.toDegrees(rectangle.north),
				lonMin: CesiumMath.toDegrees(rectangle.west),
				lonMax: CesiumMath.toDegrees(rectangle.east)
			};
		} catch (error) {
			console.error('Error computing viewport bounds:', error);
			return null;
		}
	}

	/**
	 * Load receivers in current viewport
	 */
	async function loadReceiversInViewport(): Promise<void> {
		if (!enabled || !shouldShowReceivers()) {
			// Clear receivers if disabled or zoomed out
			clearReceivers();
			return;
		}

		const bounds = getVisibleBounds();
		if (!bounds) return;

		isLoading = true;

		try {
			const receivers = await serverCall<Receiver[]>('/receivers', {
				params: {
					latitude_min: bounds.latMin,
					latitude_max: bounds.latMax,
					longitude_min: bounds.lonMin,
					longitude_max: bounds.lonMax
				}
			});

			// Update receiver entities
			// eslint-disable-next-line svelte/prefer-svelte-reactivity
			const newReceiverIds = new Set<string>();

			for (const receiver of receivers) {
				// Skip if already rendered
				if (receiverEntities.has(receiver.id)) {
					newReceiverIds.add(receiver.id);
					continue;
				}

				// Create receiver entity
				try {
					const entity = createReceiverEntity(receiver);
					viewer.entities.add(entity);
					receiverEntities.set(receiver.id, entity);
					newReceiverIds.add(receiver.id);
				} catch (error) {
					console.error(`Error creating receiver entity for ${receiver.callsign}:`, error);
				}
			}

			// Remove receivers no longer in viewport
			for (const [receiverId, entity] of receiverEntities.entries()) {
				if (!newReceiverIds.has(receiverId)) {
					viewer.entities.remove(entity);
					receiverEntities.delete(receiverId);
				}
			}

			console.log(`Loaded ${receivers.length} receivers in viewport`);
		} catch (error) {
			console.error('Error loading receivers:', error);
		} finally {
			isLoading = false;
		}
	}

	/**
	 * Clear all receiver markers
	 */
	function clearReceivers(): void {
		for (const entity of receiverEntities.values()) {
			viewer.entities.remove(entity);
		}
		receiverEntities.clear();
	}

	/**
	 * Handle camera movement (debounced)
	 */
	function handleCameraMove(): void {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}

		debounceTimer = setTimeout(() => {
			loadReceiversInViewport();
		}, 300); // 300ms debounce
	}

	// Watch for enabled state changes
	$effect(() => {
		if (enabled) {
			loadReceiversInViewport();
		} else {
			clearReceivers();
		}
	});

	onMount(() => {
		// Initial load
		loadReceiversInViewport();

		// Listen for camera movement
		viewer.camera.moveEnd.addEventListener(handleCameraMove);

		return () => {
			viewer.camera.moveEnd.removeEventListener(handleCameraMove);
			if (debounceTimer) {
				clearTimeout(debounceTimer);
			}
		};
	});

	onDestroy(() => {
		clearReceivers();
	});
</script>

<!-- No visual output - this component manages entities in the Cesium viewer -->

{#if isLoading && enabled}
	<div class="loading-indicator">
		<span>Loading receivers...</span>
	</div>
{/if}

<style>
	.loading-indicator {
		position: fixed;
		top: 130px;
		left: 50%;
		transform: translateX(-50%);
		background: rgba(0, 0, 0, 0.7);
		color: white;
		padding: 8px 16px;
		border-radius: 4px;
		font-size: 14px;
		z-index: 1000;
		pointer-events: none;
	}
</style>
