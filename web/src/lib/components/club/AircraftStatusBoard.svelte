<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Plane, ChevronDown, ChevronUp, Pencil, MoveUp, Navigation } from '@lucide/svelte';
	import { FixFeed } from '$lib/services/FixFeed';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';
	import { toaster } from '$lib/toaster';
	import { SvelteSet } from 'svelte/reactivity';
	import type { Aircraft, Flight, User } from '$lib/types';
	import PilotAssignmentEditor from './PilotAssignmentEditor.svelte';

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
	let showAll = $state(false);
	let expandedAircraftId = $state<string | null>(null);

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

	function isActiveToday(ac: Aircraft): boolean {
		if (!ac.lastFixAt) return false;
		const today = new Date();
		const fixDate = new Date(ac.lastFixAt);
		return (
			fixDate.getFullYear() === today.getFullYear() &&
			fixDate.getMonth() === today.getMonth() &&
			fixDate.getDate() === today.getDate()
		);
	}

	function getFlightForAircraft(ac: Aircraft): Flight | null {
		return flightsInProgress.find((f) => f.aircraftId === ac.id) || null;
	}

	function isAirborne(ac: Aircraft): boolean {
		const flight = getFlightForAircraft(ac);
		return flight != null && flight.state === 'active';
	}

	// Sort aircraft: airborne first, then by most recent activity
	let sortedAircraft = $derived(() => {
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

	let activeToday = $derived(sortedAircraft().filter((ac) => isActiveToday(ac)));
	let inactive = $derived(sortedAircraft().filter((ac) => !isActiveToday(ac)));
	let displayedAircraft = $derived(showAll ? sortedAircraft() : activeToday);

	function haversineNm(lat1: number, lon1: number, lat2: number, lon2: number): number {
		const R = 3440.065;
		const dLat = ((lat2 - lat1) * Math.PI) / 180;
		const dLon = ((lon2 - lon1) * Math.PI) / 180;
		const a =
			Math.sin(dLat / 2) * Math.sin(dLat / 2) +
			Math.cos((lat1 * Math.PI) / 180) *
				Math.cos((lat2 * Math.PI) / 180) *
				Math.sin(dLon / 2) *
				Math.sin(dLon / 2);
		const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));
		return R * c;
	}

	function getDistanceNm(ac: Aircraft): number | null {
		if (!userLocation || ac.latitude == null || ac.longitude == null) return null;
		return haversineNm(userLocation.lat, userLocation.lng, ac.latitude, ac.longitude);
	}

	function formatElapsed(takeoffTime: string): string {
		const minutes = Math.floor((Date.now() - new Date(takeoffTime).getTime()) / 60000);
		const h = Math.floor(minutes / 60);
		const m = minutes % 60;
		return h > 0 ? `${h}h ${m}m` : `${m}m`;
	}

	function getTowInfo(
		ac: Aircraft,
		flight: Flight | null
	): { type: 'towing' | 'towed_by'; registration: string } | null {
		if (!flight) return null;
		const towedFlight = flightsInProgress.find((f) => f.towedByAircraftId === ac.id);
		if (towedFlight) {
			return { type: 'towing', registration: towedFlight.registration || 'unknown' };
		}
		if (flight.towedByAircraftId) {
			const towPlane = flightsInProgress.find((f) => f.aircraftId === flight.towedByAircraftId);
			return { type: 'towed_by', registration: towPlane?.registration || 'unknown' };
		}
		return null;
	}

	function toggleExpanded(id: string) {
		expandedAircraftId = expandedAircraftId === id ? null : id;
	}
</script>

{#if aircraft.length === 0}
	<div class="card p-6 text-center">
		<Plane class="mx-auto mb-2 h-8 w-8 text-surface-400" />
		<p class="text-sm text-surface-500">No club aircraft</p>
	</div>
{:else}
	<div class="card">
		<header class="card-header flex items-center justify-between">
			<div>
				<h3 class="flex items-center gap-2 text-lg font-semibold">
					<Plane class="h-5 w-5" />
					Aircraft Active Today
				</h3>
				<p class="text-surface-500-400-token text-sm">
					{activeToday.length} of {aircraft.length} aircraft heard from today
				</p>
			</div>
		</header>

		{#if activeToday.length === 0 && !showAll}
			<div class="px-6 pt-2 pb-2 text-center text-sm text-surface-500">
				No aircraft have been heard from today.
			</div>
		{/if}

		<!-- Compact aircraft rows -->
		<div class="divide-y divide-surface-200 dark:divide-surface-700">
			{#each displayedAircraft as ac (ac.id)}
				{@const flight = getFlightForAircraft(ac)}
				{@const airborne = isAirborne(ac)}
				{@const tow = getTowInfo(ac, flight)}
				{@const dist = getDistanceNm(ac)}
				{@const active = isActiveToday(ac)}

				<div class={!active ? 'opacity-50' : ''}>
					<!-- Compact row -->
					<button
						onclick={() => toggleExpanded(ac.id)}
						class="flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors hover:bg-surface-100 dark:hover:bg-surface-700/50"
					>
						<!-- Status dot -->
						<span
							class="h-2.5 w-2.5 flex-shrink-0 rounded-full {airborne
								? 'animate-pulse bg-success-500'
								: active
									? 'bg-surface-400'
									: 'bg-surface-300 dark:bg-surface-600'}"
						></span>

						<!-- Registration -->
						<span class="min-w-[5rem] text-sm font-semibold">
							{ac.registration || '???'}
						</span>

						<!-- Status badge -->
						<span
							class="rounded-full px-2 py-0.5 text-xs font-medium {airborne
								? 'bg-success-500 text-white'
								: 'bg-surface-200 text-surface-600 dark:bg-surface-600 dark:text-surface-300'}"
						>
							{airborne ? 'Airborne' : 'Ground'}
						</span>

						<!-- Brief info -->
						<span class="flex-1 truncate text-xs text-surface-500">
							{#if airborne && flight?.takeoffTime}
								{formatElapsed(flight.takeoffTime)}
							{:else if ac.aircraftModel}
								{ac.aircraftModel}
							{/if}
						</span>

						<!-- Tow indicator -->
						{#if tow}
							<span class="flex items-center gap-0.5 text-xs text-primary-500">
								<MoveUp class="h-3 w-3" />
								{tow.type === 'towing' ? 'Towing' : 'Towed'}
							</span>
						{/if}

						<!-- Distance -->
						{#if dist != null}
							<span class="text-xs text-surface-400 tabular-nums">
								{dist.toFixed(0)} nm
							</span>
						{/if}
					</button>

					<!-- Expanded detail panel -->
					{#if expandedAircraftId === ac.id}
						<div
							class="border-t border-surface-200 bg-surface-50 px-4 py-3 dark:border-surface-700 dark:bg-surface-800/50"
						>
							<div class="flex flex-col gap-2 text-sm">
								{#if ac.aircraftModel}
									<div class="text-surface-500">
										{ac.aircraftModel}
										{#if ac.competitionNumber}
											<span class="ml-1 text-xs">({ac.competitionNumber})</span>
										{/if}
									</div>
								{/if}

								{#if airborne && flight?.takeoffTime}
									<div class="text-success-600 dark:text-success-400">
										Airborne for {formatElapsed(flight.takeoffTime)}
									</div>
								{/if}

								{#if tow}
									<div class="flex items-center gap-1 text-xs">
										<MoveUp class="h-3 w-3" />
										{tow.type === 'towing' ? 'Towing' : 'Towed by'}: {tow.registration}
									</div>
								{/if}

								{#if dist != null}
									<div class="flex items-center gap-1 text-xs text-surface-500">
										<Navigation class="h-3 w-3" />
										{dist.toFixed(1)} nm away
									</div>
								{/if}

								<PilotAssignmentEditor
									flightId={flight?.id ?? null}
									aircraftId={ac.id}
									aircraftRegistration={ac.registration || ''}
									{members}
									isAirborne={airborne}
									onClose={() => (expandedAircraftId = null)}
									onAssigned={onFlightsChanged}
								/>
							</div>
						</div>
					{/if}
				</div>
			{/each}
		</div>

		<!-- Show All / Show Less toggle -->
		{#if inactive.length > 0}
			<div class="border-t border-surface-200 px-4 py-2 dark:border-surface-700">
				<button
					onclick={() => (showAll = !showAll)}
					class="btn w-full gap-1 text-sm text-surface-500 hover:text-surface-700 dark:hover:text-surface-300"
				>
					{#if showAll}
						<ChevronUp class="h-4 w-4" />
						Hide inactive aircraft
					{:else}
						<ChevronDown class="h-4 w-4" />
						Show all aircraft ({inactive.length} not heard today)
					{/if}
				</button>
			</div>
		{/if}
	</div>
{/if}
