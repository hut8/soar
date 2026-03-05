<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Pencil, Plane, MoveUp, Navigation } from '@lucide/svelte';
	import type { Aircraft, Flight, User } from '$lib/types';
	import PilotAssignmentEditor from './PilotAssignmentEditor.svelte';

	let {
		aircraft,
		flight = null,
		flightsInProgress,
		userLocation = null,
		members,
		onPilotsChanged
	}: {
		aircraft: Aircraft;
		flight: Flight | null;
		flightsInProgress: Flight[];
		userLocation: { lat: number; lng: number } | null;
		members: User[];
		onPilotsChanged: () => void;
	} = $props();

	let showEditor = $state(false);
	let elapsedMinutes = $state(0);

	// Airborne detection: has an active (not timed-out/stale) flight in progress
	let isAirborne = $derived(flight != null && flight.state === 'active');

	// Tow info
	let towInfo = $derived(() => {
		if (!flight) return null;
		// Check if this aircraft is towing something (another flight references this aircraft as towedByAircraftId)
		const towedFlight = flightsInProgress.find((f) => f.towedByAircraftId === aircraft.id);
		if (towedFlight) {
			return { type: 'towing' as const, registration: towedFlight.registration || 'unknown' };
		}
		// Check if this aircraft is being towed
		if (flight.towedByAircraftId) {
			const towPlane = flightsInProgress.find((f) => f.aircraftId === flight!.towedByAircraftId);
			return {
				type: 'towed_by' as const,
				registration: towPlane?.registration || 'unknown'
			};
		}
		return null;
	});

	// Distance from user (nautical miles)
	let distanceNm = $derived(() => {
		if (!userLocation || aircraft.latitude == null || aircraft.longitude == null) return null;
		return haversineNm(userLocation.lat, userLocation.lng, aircraft.latitude, aircraft.longitude);
	});

	function haversineNm(lat1: number, lon1: number, lat2: number, lon2: number): number {
		const R = 3440.065; // Earth radius in nautical miles
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

	// Timer for flight duration
	let timer: ReturnType<typeof setInterval> | null = null;

	function updateElapsed() {
		if (flight?.takeoffTime) {
			const takeoff = new Date(flight.takeoffTime).getTime();
			elapsedMinutes = Math.floor((Date.now() - takeoff) / 60000);
		}
	}

	onMount(() => {
		updateElapsed();
		timer = setInterval(updateElapsed, 60000);
	});

	onDestroy(() => {
		if (timer) clearInterval(timer);
	});

	function formatDuration(minutes: number): string {
		const h = Math.floor(minutes / 60);
		const m = minutes % 60;
		return h > 0 ? `${h}h ${m}m` : `${m}m`;
	}
</script>

<div
	class="card p-4 {isAirborne
		? 'border-l-4 border-l-success-500 bg-success-500/10'
		: 'bg-surface-100 dark:bg-surface-800'}"
>
	<!-- Header: Registration + Status -->
	<div class="mb-2 flex items-center justify-between gap-2">
		<div class="flex items-center gap-2">
			<Plane class="h-4 w-4 {isAirborne ? 'text-success-600' : 'text-surface-500'}" />
			<span class="font-bold">{aircraft.registration || '???'}</span>
			{#if aircraft.aircraftModel}
				<span class="text-xs text-surface-500">{aircraft.aircraftModel}</span>
			{/if}
		</div>
		<span
			class="rounded-full px-2 py-0.5 text-xs font-medium {isAirborne
				? 'bg-success-500 text-white'
				: 'bg-surface-300 text-surface-700 dark:bg-surface-600 dark:text-surface-200'}"
		>
			{isAirborne ? 'Airborne' : 'Ground'}
		</span>
	</div>

	<!-- Flight info -->
	<div class="space-y-1 text-sm">
		{#if isAirborne && flight?.takeoffTime}
			<div class="text-surface-600-300-token">
				Airborne {formatDuration(elapsedMinutes)}
			</div>
		{/if}

		{#if towInfo()}
			<div class="flex items-center gap-1 text-xs">
				<MoveUp class="h-3 w-3" />
				{#if towInfo()!.type === 'towing'}
					Towing: {towInfo()!.registration}
				{:else}
					Towed by: {towInfo()!.registration}
				{/if}
			</div>
		{/if}

		{#if distanceNm() != null}
			<div class="flex items-center gap-1 text-xs text-surface-500">
				<Navigation class="h-3 w-3" />
				{distanceNm()!.toFixed(1)} nm away
			</div>
		{/if}
	</div>

	<!-- Edit Pilots button -->
	<div class="mt-3">
		{#if showEditor}
			<PilotAssignmentEditor
				flightId={flight?.id ?? null}
				aircraftId={aircraft.id}
				aircraftRegistration={aircraft.registration || ''}
				{members}
				{isAirborne}
				onClose={() => (showEditor = false)}
				onAssigned={onPilotsChanged}
			/>
		{:else}
			<button onclick={() => (showEditor = true)} class="btn w-full preset-tonal btn-sm">
				<Pencil class="h-3 w-3" />
				Edit Pilots
			</button>
		{/if}
	</div>
</div>
