<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Plane } from '@lucide/svelte';
	import { FixFeed } from '$lib/services/FixFeed';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';
	import { toaster } from '$lib/toaster';
	import { SvelteSet } from 'svelte/reactivity';
	import type { Aircraft, Flight, User } from '$lib/types';
	import AircraftStatusCard from './AircraftStatusCard.svelte';

	const logger = getLogger(['soar', 'AircraftStatusBoard']);

	let {
		aircraft,
		flightsInProgress,
		members,
		onFlightsChanged
	}: {
		aircraft: Aircraft[];
		flightsInProgress: Flight[];
		members: User[];
		onFlightsChanged: () => void;
	} = $props();

	let userLocation = $state<{ lat: number; lng: number } | null>(null);
	let unsubscribeFeed: (() => void) | null = null;

	// Track which aircraft we've already auto-assigned for (by flightId)
	let autoAssignedFlights = new SvelteSet<string>();

	onMount(() => {
		// Request user geolocation
		if (navigator.geolocation) {
			navigator.geolocation.getCurrentPosition(
				(pos) => {
					userLocation = { lat: pos.coords.latitude, lng: pos.coords.longitude };
				},
				() => {
					// Geolocation denied or unavailable - just skip
				}
			);
		}

		// Subscribe to FixFeed for each club aircraft
		const feed = FixFeed.getInstance();
		for (const ac of aircraft) {
			if (ac.id) {
				feed.subscribeToAircraft(ac.id);
			}
		}

		// Listen for fix events to detect takeoffs for auto-assignment
		unsubscribeFeed = feed.subscribe((event) => {
			if (event.type === 'fix_received') {
				checkAutoAssignment(event.fix.aircraftId, event.fix.groundSpeedKnots ?? 0);
			}
		});

		// Clean stale localStorage entries (older than 24h)
		cleanStaleAssignments();
	});

	onDestroy(() => {
		const feed = FixFeed.getInstance();
		for (const ac of aircraft) {
			if (ac.id) {
				feed.unsubscribeFromAircraft(ac.id);
			}
		}
		if (unsubscribeFeed) {
			unsubscribeFeed();
		}
	});

	function cleanStaleAssignments() {
		const cutoff = Date.now() - 24 * 60 * 60 * 1000;
		const keysToDelete: string[] = [];

		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			if (key?.startsWith('soar:ground-pilots:')) {
				try {
					const val = JSON.parse(localStorage.getItem(key) || '');
					if (val.updatedAt && new Date(val.updatedAt).getTime() < cutoff) {
						keysToDelete.push(key);
					}
				} catch {
					// Ignore malformed entries
				}
			}
		}

		for (const key of keysToDelete) {
			localStorage.removeItem(key);
		}
	}

	async function checkAutoAssignment(aircraftId: string, speedKnots: number) {
		if (speedKnots < 25) return;

		const storageKey = `soar:ground-pilots:${aircraftId}`;
		const stored = localStorage.getItem(storageKey);
		if (!stored) return;

		// Find a flight for this aircraft that we haven't auto-assigned yet
		const flightForAircraft = flightsInProgress.find(
			(f) => f.aircraftId === aircraftId && !autoAssignedFlights.has(f.id)
		);
		if (!flightForAircraft) return;

		try {
			const parsed = JSON.parse(stored);
			const assignments = parsed.pilots || [];

			for (const assignment of assignments) {
				await serverCall(`/flights/${flightForAircraft.id}/pilots`, {
					method: 'POST',
					body: JSON.stringify({
						pilot_id: assignment.pilotId,
						is_tow_pilot: assignment.role === 'tow_pilot',
						is_student: assignment.role === 'student',
						is_instructor: assignment.role === 'instructor'
					})
				});
			}

			autoAssignedFlights.add(flightForAircraft.id);
			localStorage.removeItem(storageKey);

			const ac = aircraft.find((a) => a.id === aircraftId);
			toaster.success({
				title: `Pilots assigned to flight for ${ac?.registration || 'aircraft'}`
			});

			onFlightsChanged();
		} catch (err) {
			logger.error('Auto-assignment failed for aircraft {id}: {error}', {
				id: aircraftId,
				error: err
			});
		}
	}

	// Sort aircraft: airborne first (most recent takeoff at top), then on-ground (most recent activity at top)
	let sortedAircraft = $derived(() => {
		// Precompute flight lookup map and timestamps to avoid repeated work in comparator
		const flightByAircraftId = new Map(
			flightsInProgress.filter((f) => f.aircraftId).map((f) => [f.aircraftId!, f])
		);
		const takeoffMs = new Map(
			flightsInProgress
				.filter((f) => f.aircraftId && f.takeoffTime)
				.map((f) => [f.aircraftId!, new Date(f.takeoffTime!).getTime()])
		);
		const lastFixMs = new Map(
			aircraft.filter((a) => a.lastFixAt).map((a) => [a.id, new Date(a.lastFixAt!).getTime()])
		);

		return [...aircraft].sort((a, b) => {
			const aFlight = flightByAircraftId.get(a.id);
			const bFlight = flightByAircraftId.get(b.id);
			const aAirborne = aFlight != null && aFlight.state === 'active';
			const bAirborne = bFlight != null && bFlight.state === 'active';

			if (aAirborne !== bAirborne) return aAirborne ? -1 : 1;

			if (aAirborne && bAirborne) {
				return (takeoffMs.get(b.id) ?? 0) - (takeoffMs.get(a.id) ?? 0);
			}

			return (lastFixMs.get(b.id) ?? 0) - (lastFixMs.get(a.id) ?? 0);
		});
	});

	function getFlightForAircraft(ac: Aircraft): Flight | null {
		return flightsInProgress.find((f) => f.aircraftId === ac.id) || null;
	}
</script>

{#if aircraft.length === 0}
	<div class="card p-6 text-center">
		<Plane class="mx-auto mb-2 h-8 w-8 text-surface-400" />
		<p class="text-sm text-surface-500">No club aircraft</p>
	</div>
{:else}
	<!-- Desktop: vertical scrollable column -->
	<div class="hidden flex-col gap-3 md:flex">
		<h3 class="flex items-center gap-2 text-lg font-semibold">
			<Plane class="h-5 w-5" />
			Aircraft Status
		</h3>
		<div class="max-h-[calc(100vh-300px)] space-y-3 overflow-y-auto pr-1">
			{#each sortedAircraft() as ac (ac.id)}
				<AircraftStatusCard
					aircraft={ac}
					flight={getFlightForAircraft(ac)}
					{flightsInProgress}
					{userLocation}
					{members}
					onPilotsChanged={onFlightsChanged}
				/>
			{/each}
		</div>
	</div>

	<!-- Mobile: horizontal scroll row -->
	<div class="md:hidden">
		<h3 class="mb-2 flex items-center gap-2 text-lg font-semibold">
			<Plane class="h-5 w-5" />
			Aircraft Status
		</h3>
		<div class="flex gap-3 overflow-x-auto pb-2">
			{#each sortedAircraft() as ac (ac.id)}
				<div class="w-64 flex-shrink-0">
					<AircraftStatusCard
						aircraft={ac}
						flight={getFlightForAircraft(ac)}
						{flightsInProgress}
						{userLocation}
						{members}
						onPilotsChanged={onFlightsChanged}
					/>
				</div>
			{/each}
		</div>
	</div>
{/if}
