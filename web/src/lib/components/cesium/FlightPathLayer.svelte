<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { Viewer, Entity } from 'cesium';
	import { serverCall } from '$lib/api/server';
	import {
		createFlightPathEntity,
		createTakeoffMarker,
		createLandingMarker
	} from '$lib/cesium/entities';
	import type { Flight, Fix } from '$lib/types';

	// Props
	let {
		viewer,
		flightIds = $bindable([]),
		colorScheme = $bindable<'altitude' | 'time'>('altitude')
	}: {
		viewer: Viewer;
		flightIds?: string[];
		colorScheme?: 'altitude' | 'time';
	} = $props();

	// State
	let flightEntities = $state<Map<string, Entity[]>>(new Map()); // Map of flight ID -> [path entity, takeoff marker, landing marker]
	let flightData = $state<Map<string, { flight: Flight; fixes: Fix[] }>>(new Map());
	let isLoading = $state(false);

	/**
	 * Load flight path data from API
	 */
	async function loadFlightPath(flightId: string): Promise<void> {
		try {
			// Fetch flight info and fixes in parallel
			interface FlightResponse {
				flight: Flight;
			}

			interface FlightFixesResponse {
				fixes: Fix[];
				count: number;
			}

			const [flightResponse, fixesResponse] = await Promise.all([
				serverCall<FlightResponse>(`/flights/${flightId}`),
				serverCall<FlightFixesResponse>(`/flights/${flightId}/fixes`)
			]);

			const flight = flightResponse.flight;
			const fixes = fixesResponse.fixes;

			// Store flight data
			flightData.set(flightId, { flight, fixes });

			// Render flight path
			renderFlightPath(flightId, flight, fixes);

			console.log(`Loaded flight path for ${flightId}: ${fixes.length} fixes`);
		} catch (error) {
			console.error(`Error loading flight path ${flightId}:`, error);
		}
	}

	/**
	 * Render flight path on globe
	 */
	function renderFlightPath(flightId: string, flight: Flight, fixes: Fix[]): void {
		// Remove existing entities for this flight
		const existingEntities = flightEntities.get(flightId);
		if (existingEntities) {
			existingEntities.forEach((entity: Entity) => viewer.entities.remove(entity));
		}

		if (fixes.length === 0) {
			console.warn(`No fixes to render for flight ${flightId}`);
			return;
		}

		const entities: Entity[] = [];

		// Create flight path polyline
		try {
			const pathEntity = createFlightPathEntity(flight, fixes, colorScheme);
			viewer.entities.add(pathEntity);
			entities.push(pathEntity);
		} catch (error) {
			console.error('Error creating flight path entity:', error);
		}

		// Create takeoff marker (first fix)
		const firstFix = fixes[0];
		if (firstFix) {
			const takeoffMarker = createTakeoffMarker(
				firstFix.latitude,
				firstFix.longitude,
				firstFix.altitude_msl_feet || 0
			);
			viewer.entities.add(takeoffMarker);
			entities.push(takeoffMarker);
		}

		// Create landing marker (last fix, if flight is complete)
		if (flight.landing_time && fixes.length > 0) {
			const lastFix = fixes[fixes.length - 1];
			const landingMarker = createLandingMarker(
				lastFix.latitude,
				lastFix.longitude,
				lastFix.altitude_msl_feet || 0
			);
			viewer.entities.add(landingMarker);
			entities.push(landingMarker);
		}

		// Store entities
		flightEntities.set(flightId, entities);
	}

	/**
	 * Remove flight path from globe
	 */
	function removeFlightPath(flightId: string): void {
		const entities = flightEntities.get(flightId);
		if (entities) {
			entities.forEach((entity: Entity) => viewer.entities.remove(entity));
			flightEntities.delete(flightId);
		}
		flightData.delete(flightId);
	}

	/**
	 * Update all flight paths when color scheme changes
	 */
	function updateColorScheme(): void {
		for (const [flightId, data] of flightData.entries()) {
			renderFlightPath(flightId, data.flight, data.fixes);
		}
	}

	/**
	 * Add flight to display
	 */
	export function addFlight(flightId: string): void {
		if (flightEntities.has(flightId)) {
			console.log(`Flight ${flightId} already displayed`);
			return;
		}

		loadFlightPath(flightId);
	}

	/**
	 * Remove flight from display
	 */
	export function removeFlight(flightId: string): void {
		removeFlightPath(flightId);
	}

	/**
	 * Clear all flight paths
	 */
	export function clearAllFlights(): void {
		for (const flightId of flightEntities.keys()) {
			removeFlightPath(flightId);
		}
	}

	// Watch for flight ID changes
	$effect(() => {
		// Load new flights
		for (const flightId of flightIds) {
			if (!flightEntities.has(flightId)) {
				loadFlightPath(flightId);
			}
		}

		// Remove flights no longer in list
		const currentIds = new Set(flightIds);
		for (const flightId of flightEntities.keys()) {
			if (!currentIds.has(flightId)) {
				removeFlightPath(flightId);
			}
		}
	});

	// Watch for color scheme changes
	$effect(() => {
		updateColorScheme();
	});

	onDestroy(() => {
		// Remove all flight paths
		clearAllFlights();
	});
</script>

<!-- No visual output - this component manages entities in the Cesium viewer -->

{#if isLoading}
	<div class="loading-indicator">
		<span>Loading flight paths...</span>
	</div>
{/if}

<style>
	.loading-indicator {
		position: fixed;
		top: 50px;
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
