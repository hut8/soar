<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Viewer, Entity } from 'cesium';
	import { Math as CesiumMath } from 'cesium';
	import { serverCall } from '$lib/api/server';
	import { createAircraftEntity } from '$lib/cesium/entities';
	import { FixFeed, type FixFeedEvent } from '$lib/services/FixFeed';
	import type { Aircraft, Fix } from '$lib/types';

	// Props
	let { viewer }: { viewer: Viewer } = $props();

	// State
	let aircraftEntities = $state<Map<string, Entity>>(new Map()); // Map of aircraft ID -> Entity
	let aircraftData = $state<Map<string, Aircraft>>(new Map()); // Map of aircraft ID -> Aircraft data
	let isLoading = $state(false);
	let lastBounds: { latMin: number; latMax: number; lonMin: number; lonMax: number } | null = null;
	let subscribedAreas = $state<Set<string>>(new Set()); // Set of subscribed lat/lon squares

	// Services
	let fixFeed = FixFeed.getInstance();
	let unsubscribeFixFeed: (() => void) | null = null;

	// Debounce timer for camera movement
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

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
	 * Check if bounds have changed significantly (>10% of current view)
	 */
	function boundsChangedSignificantly(
		newBounds: { latMin: number; latMax: number; lonMin: number; lonMax: number } | null
	): boolean {
		if (!newBounds || !lastBounds) return true;

		const latDiff = Math.abs(newBounds.latMin - lastBounds.latMin);
		const lonDiff = Math.abs(newBounds.lonMin - lastBounds.lonMin);
		const latRange = newBounds.latMax - newBounds.latMin;
		const lonRange = newBounds.lonMax - newBounds.lonMin;

		// Reload if moved more than 10% of current view
		return latDiff > latRange * 0.1 || lonDiff > lonRange * 0.1;
	}

	/**
	 * Load aircraft in current viewport
	 */
	async function loadAircraftInViewport(): Promise<void> {
		const bounds = getVisibleBounds();
		if (!bounds) return;

		// Skip if bounds haven't changed significantly
		if (!boundsChangedSignificantly(bounds)) {
			return;
		}

		isLoading = true;
		lastBounds = bounds;

		try {
			// Fetch aircraft in bounding box (last hour, with up to 10 recent fixes each)
			// API returns array directly, not wrapped in object
			const aircraftList = await serverCall<Aircraft[]>('/aircraft', {
				params: {
					latitude_min: bounds.latMin,
					latitude_max: bounds.latMax,
					longitude_min: bounds.lonMin,
					longitude_max: bounds.lonMax,
					after: new Date(Date.now() - 60 * 60 * 1000).toISOString() // 1 hour ago
				}
			});

			// Update aircraft entities
			// eslint-disable-next-line svelte/prefer-svelte-reactivity
			const newAircraftIds = new Set<string>();

			for (const aircraft of aircraftList) {
				// Get latest fix from currentFix or fall back to fixes array
				const latestFix = aircraft.currentFix || aircraft.fixes?.[0];

				// Skip if no fix data available
				if (!latestFix) continue;

				// Update or create entity
				const existingEntity = aircraftEntities.get(aircraft.id);
				if (existingEntity) {
					// Update existing entity (position, label, etc.)
					viewer.entities.remove(existingEntity);
				}

				// Create new entity
				const entity = createAircraftEntity(aircraft, latestFix);
				viewer.entities.add(entity);
				aircraftEntities.set(aircraft.id, entity);
				newAircraftIds.add(aircraft.id);
			}

			// Remove aircraft that are no longer in viewport
			for (const [aircraftId, entity] of aircraftEntities.entries()) {
				if (!newAircraftIds.has(aircraftId)) {
					viewer.entities.remove(entity);
					aircraftEntities.delete(aircraftId);
				}
			}

			console.log(`Loaded ${aircraftList.length} aircraft in viewport`);
		} catch (error) {
			console.error('Error loading aircraft:', error);
		} finally {
			isLoading = false;
		}
	}

	/**
	 * Handle camera movement (debounced)
	 */
	function handleCameraMove(): void {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}

		debounceTimer = setTimeout(() => {
			loadAircraftInViewport();
		}, 200); // 200ms debounce
	}

	/**
	 * Get lat/lon squares for area-based subscriptions
	 * Returns array of {lat, lon} degree squares
	 */
	function getVisibleLatLonSquares(): { lat: number; lon: number }[] {
		const bounds = getVisibleBounds();
		if (!bounds) return [];

		const squares: { lat: number; lon: number }[] = [];

		// Round to 1-degree squares
		const minLat = Math.floor(bounds.latMin);
		const maxLat = Math.ceil(bounds.latMax);
		const minLon = Math.floor(bounds.lonMin);
		const maxLon = Math.ceil(bounds.lonMax);

		for (let lat = minLat; lat <= maxLat; lat++) {
			for (let lon = minLon; lon <= maxLon; lon++) {
				squares.push({ lat, lon });
			}
		}

		return squares;
	}

	/**
	 * Update area subscriptions based on visible viewport
	 */
	function updateAreaSubscriptions(): void {
		const bounds = getVisibleBounds();
		if (!bounds) return;

		const geoBounds = {
			north: bounds.latMax,
			south: bounds.latMin,
			east: bounds.lonMax,
			west: bounds.lonMin
		};

		fixFeed.sendWebSocketMessage({
			action: 'subscribe',
			type: 'area_bulk',
			bounds: geoBounds
		});

		// Update subscribed areas for tracking
		const visibleSquares = getVisibleLatLonSquares();
		subscribedAreas.clear();
		for (const square of visibleSquares) {
			subscribedAreas.add(`${square.lat},${square.lon}`);
		}
	}

	/**
	 * Handle WebSocket events (fix updates, aircraft data)
	 */
	function handleFixFeedEvent(event: FixFeedEvent): void {
		if (event.type === 'fix_received') {
			// Update aircraft position from real-time fix
			const fix = event.fix;
			const aircraftId = fix.aircraft_id;

			if (!aircraftId) return;

			// Get or create aircraft data
			let aircraft = aircraftData.get(aircraftId);
			if (!aircraft) {
				// Create minimal aircraft data from fix
				aircraft = {
					id: aircraftId,
					addressType: '',
					address: fix.device_address_hex || '',
					aircraftModel: fix.model || '',
					registration: fix.registration || null,
					competitionNumber: '',
					tracked: false,
					identified: false,
					createdAt: new Date().toISOString(),
					updatedAt: new Date().toISOString(),
					fromOgnDdb: false,
					fromAdsbxDdb: false,
					currentFix: fix,
					fixes: []
				};
				aircraftData.set(aircraftId, aircraft);
			} else {
				// Push old currentFix to fixes array if it exists
				if (aircraft.currentFix) {
					aircraft.fixes = aircraft.fixes || [];
					aircraft.fixes.unshift(aircraft.currentFix);
					// Limit to 100 fixes
					if (aircraft.fixes.length > 100) {
						aircraft.fixes = aircraft.fixes.slice(0, 100);
					}
				}
				// Set new currentFix
				aircraft.currentFix = fix;
			}

			// Update aircraft entity
			updateAircraftPosition(aircraft, fix);
		} else if (event.type === 'aircraft_received') {
			// Full aircraft data with recent fixes
			const aircraft = event.aircraft;
			aircraftData.set(aircraft.id, aircraft);

			// Update entity if aircraft has currentFix or fixes
			const latestFix = aircraft.currentFix || aircraft.fixes?.[0];
			if (latestFix) {
				updateAircraftPosition(aircraft, latestFix);
			}
		} else if (event.type === 'connection_opened') {
			console.log('WebSocket connected - updating area subscriptions');
			updateAreaSubscriptions();
		}
	}

	/**
	 * Update aircraft position (from WebSocket or initial load)
	 */
	function updateAircraftPosition(aircraft: Aircraft, fix: Fix): void {
		const existingEntity = aircraftEntities.get(aircraft.id);

		if (existingEntity) {
			// Remove old entity
			viewer.entities.remove(existingEntity);
		}

		// Create updated entity
		const entity = createAircraftEntity(aircraft, fix);
		viewer.entities.add(entity);
		aircraftEntities.set(aircraft.id, entity);
	}

	onMount(() => {
		// Start WebSocket connection
		fixFeed.startLiveFixesFeed();

		// Subscribe to fix feed events
		unsubscribeFixFeed = fixFeed.subscribe(handleFixFeedEvent);

		// Initial load of aircraft from REST API
		loadAircraftInViewport();

		// Subscribe to area-based updates
		updateAreaSubscriptions();

		// Listen for camera movement (updates both REST API loads and WebSocket subscriptions)
		const handleCameraMoveWithSubscriptions = () => {
			handleCameraMove(); // Load aircraft from API
			if (debounceTimer) clearTimeout(debounceTimer);
			debounceTimer = setTimeout(() => {
				updateAreaSubscriptions(); // Update WebSocket area subscriptions
			}, 200);
		};

		viewer.camera.moveEnd.addEventListener(handleCameraMoveWithSubscriptions);

		// Cleanup
		return () => {
			viewer.camera.moveEnd.removeEventListener(handleCameraMoveWithSubscriptions);
			if (debounceTimer) {
				clearTimeout(debounceTimer);
			}
			if (unsubscribeFixFeed) {
				unsubscribeFixFeed();
			}
		};
	});

	onDestroy(() => {
		// Unsubscribe from all areas
		fixFeed.sendWebSocketMessage({
			action: 'unsubscribe',
			type: 'area_bulk',
			bounds: {
				north: 0,
				south: 0,
				east: 0,
				west: 0
			}
		});
		subscribedAreas.clear();

		// Remove all aircraft entities
		for (const entity of aircraftEntities.values()) {
			viewer.entities.remove(entity);
		}
		aircraftEntities.clear();
		aircraftData.clear();

		// Unsubscribe from fix feed
		if (unsubscribeFixFeed) {
			unsubscribeFixFeed();
		}
	});
</script>

<!-- No visual output - this component manages entities in the Cesium viewer -->

{#if isLoading}
	<div class="loading-indicator">
		<span>Loading aircraft...</span>
	</div>
{/if}

<style>
	.loading-indicator {
		position: fixed;
		top: 10px;
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
