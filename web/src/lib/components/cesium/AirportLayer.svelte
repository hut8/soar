<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Viewer, Entity } from 'cesium';
	import { Math as CesiumMath } from 'cesium';
	import { serverCall } from '$lib/api/server';
	import { createAirportEntity } from '$lib/cesium/entities';
	import type { Airport, DataListResponse } from '$lib/types';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'cesium', 'AirportLayer']);

	// Props
	let { viewer, enabled = $bindable(true) }: { viewer: Viewer; enabled?: boolean } = $props();

	// State
	let airportEntities = $state<Map<number, Entity>>(new Map()); // Map of airport ID -> Entity

	// Debounce timer
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Maximum camera height to show airports (in meters)
	const MAX_CAMERA_HEIGHT = 100000; // 100km

	/**
	 * Check if camera is zoomed in enough to show airports
	 */
	function shouldShowAirports(): boolean {
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
			logger.error('Error computing viewport bounds: {error}', { error });
			return null;
		}
	}

	/**
	 * Load airports in current viewport
	 */
	async function loadAirportsInViewport(): Promise<void> {
		if (!enabled || !shouldShowAirports()) {
			// Clear airports if disabled or zoomed out
			clearAirports();
			return;
		}

		const bounds = getVisibleBounds();
		if (!bounds) return;

		try {
			const response = await serverCall<DataListResponse<Airport>>('/airports', {
				params: {
					north: bounds.latMax,
					west: bounds.lonMin,
					south: bounds.latMin,
					east: bounds.lonMax
				}
			});

			const airports = response.data || [];

			// Update airport entities
			// eslint-disable-next-line svelte/prefer-svelte-reactivity
			const newAirportIds = new Set<number>();

			for (const airport of airports) {
				// Skip if already rendered
				if (airportEntities.has(airport.id)) {
					newAirportIds.add(airport.id);
					continue;
				}

				// Create airport entity
				try {
					const entity = createAirportEntity(airport);
					viewer.entities.add(entity);
					airportEntities.set(airport.id, entity);
					newAirportIds.add(airport.id);
				} catch (error) {
					logger.error('Error creating airport entity for {ident}: {error}', {
						ident: airport.ident,
						error
					});
				}
			}

			// Remove airports no longer in viewport
			for (const [airportId, entity] of airportEntities.entries()) {
				if (!newAirportIds.has(airportId)) {
					viewer.entities.remove(entity);
					airportEntities.delete(airportId);
				}
			}

			logger.debug('Loaded {count} airports in viewport', { count: airports.length });
		} catch (error) {
			logger.error('Error loading airports: {error}', { error });
		}
	}

	/**
	 * Clear all airport markers
	 */
	function clearAirports(): void {
		for (const entity of airportEntities.values()) {
			viewer.entities.remove(entity);
		}
		airportEntities.clear();
	}

	/**
	 * Handle camera movement (debounced)
	 */
	function handleCameraMove(): void {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}

		debounceTimer = setTimeout(() => {
			loadAirportsInViewport();
		}, 300); // 300ms debounce
	}

	// Watch for enabled state changes
	$effect(() => {
		if (enabled) {
			loadAirportsInViewport();
		} else {
			clearAirports();
		}
	});

	onMount(() => {
		// Initial load
		loadAirportsInViewport();

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
		clearAirports();
	});
</script>

<!-- No visual output - this component manages entities in the Cesium viewer -->
