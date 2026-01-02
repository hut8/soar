<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { Viewer, Entity } from 'cesium';
	import { Math as CesiumMath } from 'cesium';
	import { createAircraftEntity } from '$lib/cesium/entities';
	import { FixFeed, type FixFeedEvent } from '$lib/services/FixFeed';
	import type { Aircraft, Fix, FixWithExtras } from '$lib/types';
	import { isAircraftItem } from '$lib/types';
	import type { JsonValue } from '../../../../../bindings/serde_json/JsonValue';

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

	// Debounce timers
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let areaSubscriptionDebounceTimer: ReturnType<typeof setTimeout> | null = null;

	// Rendering limits (like operations page)
	const MAX_AIRCRAFT_DISPLAY = 50;

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
	 * Get camera height above terrain to determine zoom level
	 */
	function getCameraHeight(): number {
		try {
			return viewer.camera.positionCartographic.height;
		} catch (error) {
			console.error('Error getting camera height:', error);
			return 10000000; // Default to very high (zoomed out)
		}
	}

	/**
	 * Check if labels should be shown based on zoom level
	 * Only show labels when zoomed in fairly close (< 500km altitude)
	 */
	function shouldShowLabels(): boolean {
		const height = getCameraHeight();
		return height < 500000; // 500km
	}

	/**
	 * Load aircraft in current viewport using clustering logic from operations page
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
			// Fetch aircraft in bounding box using clustering API (like operations page)
			// Uses south/north/west/east parameters with limit for clustering
			const response = await fixFeed.fetchAircraftInBoundingBox(
				bounds.latMin, // south
				bounds.latMax, // north
				bounds.lonMin, // west
				bounds.lonMax, // east
				undefined, // no after timestamp filter
				MAX_AIRCRAFT_DISPLAY // limit to trigger clustering if needed
			);

			const { items, total, clustered } = response;

			console.log(
				`[GLOBE] Loaded ${items.length} items (total: ${total}, clustered: ${clustered})`
			);

			// Update aircraft entities
			// eslint-disable-next-line svelte/prefer-svelte-reactivity
			const newAircraftIds = new Set<string>();

			// Check if we should show labels based on zoom
			const showLabels = shouldShowLabels();

			for (const item of items) {
				// Only handle individual aircraft, not clusters (globe doesn't support cluster display yet)
				if (!isAircraftItem(item)) {
					console.log('[GLOBE] Skipping cluster item (not yet supported on globe)');
					continue;
				}

				const aircraft = item.data;

				// Get latest fix from currentFix or fall back to fixes array
				// currentFix is JsonValue, fixes array contains Fix objects
				const latestFix = aircraft.fixes?.[0] || aircraft.currentFix;

				// Skip if no fix data available
				if (!latestFix) continue;

				// Cast to Fix type (currentFix should be serialized Fix data)
				const fixData = latestFix as Fix;

				// Update or create entity
				const existingEntity = aircraftEntities.get(aircraft.id);
				if (existingEntity) {
					// Update existing entity (position, label, etc.)
					viewer.entities.remove(existingEntity);
				}

				// Create new entity with label visibility based on zoom
				const entity = createAircraftEntity(aircraft, fixData, showLabels);
				viewer.entities.add(entity);
				aircraftEntities.set(aircraft.id, entity);
				newAircraftIds.add(aircraft.id);

				// Store aircraft data for WebSocket updates
				aircraftData.set(aircraft.id, aircraft);
			}

			// Remove aircraft that are no longer in viewport
			for (const [aircraftId, entity] of aircraftEntities.entries()) {
				if (!newAircraftIds.has(aircraftId)) {
					viewer.entities.remove(entity);
					aircraftEntities.delete(aircraftId);
					aircraftData.delete(aircraftId);
				}
			}

			console.log(`[GLOBE] ${aircraftEntities.size} aircraft entities on globe`);
		} catch (error) {
			console.error('Error loading aircraft:', error);
		} finally {
			isLoading = false;
		}
	}

	/**
	 * Handle camera movement (debounced)
	 * Refreshes aircraft from REST API
	 */
	function handleCameraMove(): void {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}

		debounceTimer = setTimeout(() => {
			loadAircraftInViewport();
		}, 500); // 500ms debounce
	}

	/**
	 * Handle camera movement for area subscriptions (debounced separately)
	 * Updates WebSocket area subscriptions with reasonable debounce
	 */
	function handleCameraMoveForAreaSubscriptions(): void {
		if (areaSubscriptionDebounceTimer) {
			clearTimeout(areaSubscriptionDebounceTimer);
		}

		areaSubscriptionDebounceTimer = setTimeout(() => {
			updateAreaSubscriptions();
		}, 1000); // 1 second debounce for area subscriptions
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
			const aircraftId = fix.aircraftId;

			if (!aircraftId) return;

			// Get or create aircraft data
			let aircraft = aircraftData.get(aircraftId);
			if (!aircraft) {
				// Create minimal aircraft data from fix
				const fixWithExtras = fix as FixWithExtras;
				aircraft = {
					id: aircraftId,
					addressType: '',
					address: fixWithExtras.deviceAddressHex || '',
					aircraftModel: fixWithExtras.model || '',
					registration: fixWithExtras.registration || null,
					competitionNumber: '',
					tracked: false,
					identified: false,
					clubId: null,
					createdAt: new Date().toISOString(),
					updatedAt: new Date().toISOString(),
					fromOgnDdb: false,
					fromAdsbxDdb: false,
					frequencyMhz: null,
					pilotName: null,
					homeBaseAirportIdent: null,
					aircraftTypeOgn: null,
					lastFixAt: null,
					trackerDeviceType: null,
					icaoModelCode: null,
					countryCode: null,
					ownerOperator: null,
					addressCountry: null,
					latitude: null,
					longitude: null,
					adsbEmitterCategory: null,
					currentFix: null,
					fixes: []
				};
				aircraftData.set(aircraftId, aircraft);
			} else {
				// Push old currentFix to fixes array if it exists
				// Note: currentFix is JsonValue, not Fix in the typed interface
				if (aircraft.currentFix && aircraft.fixes) {
					aircraft.fixes.unshift(fix);
					// Limit to 100 fixes
					if (aircraft.fixes.length > 100) {
						aircraft.fixes = aircraft.fixes.slice(0, 100);
					}
				}
				// Set new currentFix (cast to JsonValue as it's stored as JSON)
				aircraft.currentFix = fix as unknown as JsonValue;
			}

			// Update aircraft entity
			updateAircraftPosition(aircraft, fix);
		} else if (event.type === 'aircraft_received') {
			// Full aircraft data with recent fixes
			const aircraft = event.aircraft;
			aircraftData.set(aircraft.id, aircraft);

			// Update entity if aircraft has currentFix or fixes
			const latestFix = aircraft.fixes?.[0] || aircraft.currentFix;
			if (latestFix) {
				updateAircraftPosition(aircraft, latestFix as Fix);
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

		// Create updated entity with label visibility based on current zoom
		const showLabels = shouldShowLabels();
		const entity = createAircraftEntity(aircraft, fix, showLabels);
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

		// Listen for camera movement - use separate handlers with different debounce times
		const handleCameraMoveComplete = () => {
			handleCameraMove(); // Load aircraft from API (500ms debounce)
			handleCameraMoveForAreaSubscriptions(); // Update WebSocket subscriptions (1s debounce)
		};

		viewer.camera.moveEnd.addEventListener(handleCameraMoveComplete);

		// Cleanup
		return () => {
			viewer.camera.moveEnd.removeEventListener(handleCameraMoveComplete);
			if (debounceTimer) {
				clearTimeout(debounceTimer);
			}
			if (areaSubscriptionDebounceTimer) {
				clearTimeout(areaSubscriptionDebounceTimer);
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
