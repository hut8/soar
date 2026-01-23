<script lang="ts">
	import { MapPin, Clock, ExternalLink, MoveUp, AlertCircle } from '@lucide/svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import {
		getAircraftCategoryDescription,
		getAircraftCategoryColor,
		getFlagPath
	} from '$lib/formatters';
	import type { Flight, FlightDetails, Aircraft, DataResponse } from '$lib/types';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';
	import AircraftLink from '$lib/components/AircraftLink.svelte';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'FlightsList']);

	dayjs.extend(relativeTime);

	interface Props {
		flights: Flight[];
		showEnd?: boolean; // If true, show End column (for completed flights)
		showAircraft?: boolean; // If true, show aircraft column
	}

	let { flights, showEnd = false, showAircraft = true }: Props = $props();

	// Internal state for enriched flight details
	let flightDetails: FlightDetails[] = $state([]);

	// Fetch aircraft data whenever flights change (only if showAircraft is true)
	$effect(() => {
		async function fetchAircraftData() {
			// Skip fetching aircraft data if we're not showing the aircraft column
			if (!showAircraft) {
				flightDetails = flights.map((flight) => ({ flight, aircraft: null }));
				return;
			}

			// Debug: Log flight IDs and aircraft IDs
			logger.debug('Flights received: {count} firstFlight: {firstFlight}', {
				count: flights.length,
				firstFlight: flights[0]
					? {
							id: flights[0].id,
							aircraftId: flights[0].aircraftId
						}
					: null
			});

			// Extract unique aircraft IDs from flights
			const aircraftIds = Array.from(
				new Set(flights.filter((f) => f.aircraftId).map((f) => f.aircraftId!))
			);

			if (aircraftIds.length === 0) {
				// No aircraft IDs, just create FlightDetails with null aircraft
				flightDetails = flights.map((flight) => ({ flight, aircraft: null }));
				return;
			}

			try {
				// Fetch bulk aircraft data
				const response = await serverCall<DataResponse<Record<string, Aircraft>>>(
					`/aircraft/bulk?ids=${aircraftIds.join(',')}`
				);

				const aircraftMap = response.data;

				// Decorate flights with aircraft data
				flightDetails = flights.map((flight) => ({
					flight,
					aircraft: flight.aircraftId ? aircraftMap[flight.aircraftId] || null : null
				}));
			} catch (err) {
				logger.error('Failed to fetch aircraft data: {error}', { error: err });
				// On error, create FlightDetails with null aircraft
				flightDetails = flights.map((flight) => ({ flight, aircraft: null }));
			}
		}

		fetchAircraftData();
	});

	function formatAircraftAddress(address: string, addressType: string): string {
		const typePrefix = addressType === 'Flarm' ? 'F' : addressType === 'Ogn' ? 'O' : 'I';
		return `${typePrefix}-${address}`;
	}

	function formatRelativeTime(dateString: string | null | undefined): string {
		if (!dateString) return '—';
		return dayjs(dateString).fromNow();
	}

	function formatLocalTime(dateString: string | null | undefined): string {
		if (!dateString) return '';
		return dayjs(dateString).format('HH:mm');
	}

	function calculateFlightDuration(
		createdAt: string | null | undefined,
		latestFixTimestamp: string | null | undefined,
		landing: string | null | undefined
	): string {
		// Always use createdAt as start time
		if (!createdAt) return '—';

		const startTime = new Date(createdAt).getTime();
		let endTime: number;

		// Determine end time: landing if complete, latest fix if active/timed out, or now
		if (landing) {
			endTime = new Date(landing).getTime();
		} else if (latestFixTimestamp) {
			endTime = new Date(latestFixTimestamp).getTime();
		} else {
			// No fixes yet - use now
			endTime = new Date().getTime();
		}

		const durationMs = endTime - startTime;
		const hours = Math.floor(durationMs / (1000 * 60 * 60));
		const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	}

	function formatDistance(meters: number | null | undefined): string {
		if (meters === null || meters === undefined) return '—';
		const nm = meters / 1852;
		const km = meters / 1000;
		if (nm >= 1) {
			return `${nm.toFixed(1)} nm`;
		} else {
			return `${km.toFixed(1)} km`;
		}
	}

	function formatAltitude(
		mslFeet: number | null | undefined,
		aglFeet: number | null | undefined
	): string {
		if (mslFeet === null && aglFeet === null) return '—';
		const parts: string[] = [];
		if (mslFeet !== null && mslFeet !== undefined) {
			parts.push(`${mslFeet.toLocaleString()} ft MSL`);
		}
		if (aglFeet !== null && aglFeet !== undefined) {
			parts.push(`${aglFeet.toLocaleString()} ft AGL`);
		}
		return parts.join(' / ');
	}

	// Check if flight was first seen airborne (no takeoff time)
	function isAirborne(flight: Flight): boolean {
		return !flight.takeoffTime;
	}

	// Check if flight timed out
	function isTimedOut(flight: Flight): boolean {
		return !!flight.timedOutAt && !flight.landingTime;
	}

	// Check if flight is still active
	function isActive(flight: Flight): boolean {
		return !flight.landingTime && !flight.timedOutAt;
	}
</script>

<!-- Desktop Table -->
<div class="hidden md:block">
	<div class="table-container">
		<table class="table-hover table">
			<thead>
				<tr>
					{#if showAircraft}
						<th>Aircraft</th>
						<th>Type</th>
					{/if}
					<th>Start</th>
					{#if showEnd}
						<th>End</th>
					{/if}
					<th>Duration</th>
					{#if showEnd}
						<th>Distance</th>
					{/if}
					{#if !showEnd}
						<th>Altitude</th>
						<th>Latest Fix</th>
					{/if}
					<th>Tow</th>
					<th></th>
				</tr>
			</thead>
			<tbody>
				{#each flightDetails as { flight, aircraft } (flight.id)}
					<tr>
						{#if showAircraft}
							<td>
								<div class="flex flex-col gap-1">
									<div class="flex items-center gap-2">
										{#if aircraft}
											<AircraftLink {aircraft} size="md" />
										{:else}
											<span class="font-medium"
												>{flight.registration ||
													formatAircraftAddress(
														flight.deviceAddress,
														flight.deviceAddressType
													)}</span
											>
										{/if}
										{#if flight.towedByAircraftId}
											<span
												class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
												title="This aircraft was towed"
											>
												<MoveUp class="h-3 w-3" />
												Towed
											</span>
										{/if}
									</div>
								</div>
							</td>
							<td>
								{#if flight.aircraftCategory}
									<span class="badge {getAircraftCategoryColor(flight.aircraftCategory)} text-xs">
										{getAircraftCategoryDescription(flight.aircraftCategory)}
									</span>
								{:else}
									<span class="text-surface-500">—</span>
								{/if}
							</td>
						{/if}
						<td>
							<div class="flex flex-col gap-1">
								<div class="text-sm">
									{formatRelativeTime(isAirborne(flight) ? flight.createdAt : flight.takeoffTime)}
								</div>
								<div class="flex items-center gap-1 text-xs">
									<span class="text-surface-500-400-token">
										{formatLocalTime(isAirborne(flight) ? flight.createdAt : flight.takeoffTime)}
									</span>
									{#if isAirborne(flight)}
										<span
											class="badge preset-filled-surface-500 text-xs"
											title="First detected while airborne"
										>
											Airborne
										</span>
									{/if}
								</div>
								{#if flight.departureAirport}
									<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
										<MapPin class="h-3 w-3" />
										{#if flight.departureAirportCountry}
											<img
												src={getFlagPath(flight.departureAirportCountry)}
												alt=""
												class="inline-block h-3 rounded-sm"
											/>
										{/if}
										{flight.departureAirport}
									</div>
								{:else if flight.startLocationCity || flight.startLocationState || flight.startLocationCountry}
									<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
										<MapPin class="h-3 w-3" />
										{#if flight.startLocationCountry}
											<img
												src={getFlagPath(flight.startLocationCountry)}
												alt=""
												class="inline-block h-3 rounded-sm"
											/>
										{/if}
										{[
											flight.startLocationCity,
											flight.startLocationState,
											flight.startLocationCountry
										]
											.filter(Boolean)
											.join(', ')}
									</div>
								{/if}
							</div>
						</td>
						{#if showEnd}
							<td>
								<div class="flex flex-col gap-1">
									{#if isTimedOut(flight)}
										<div class="flex items-center gap-1 text-sm">
											<Clock class="h-3 w-3" />
											{formatRelativeTime(flight.latestFixTimestamp)}
										</div>
										{#if flight.latestFixTimestamp}
											<div class="text-surface-500-400-token text-xs">
												{formatLocalTime(flight.latestFixTimestamp)}
											</div>
										{/if}
										<div class="mt-1">
											<span
												class="badge preset-filled-warning-500 text-xs"
												title="No beacons received for 1+ hour"
											>
												<AlertCircle class="mr-1 inline h-3 w-3" />
												Timed out
											</span>
										</div>
									{:else if isActive(flight)}
										<span class="text-surface-500-400-token text-sm">In progress</span>
									{:else}
										<div class="flex items-center gap-1 text-sm">
											<Clock class="h-3 w-3" />
											{formatRelativeTime(flight.landingTime)}
										</div>
										{#if flight.landingTime}
											<div class="text-surface-500-400-token text-xs">
												{formatLocalTime(flight.landingTime)}
											</div>
										{/if}
										{#if flight.arrivalAirport}
											<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
												<MapPin class="h-3 w-3" />
												{#if flight.arrivalAirportCountry}
													<img
														src={getFlagPath(flight.arrivalAirportCountry)}
														alt=""
														class="inline-block h-3 rounded-sm"
													/>
												{/if}
												{flight.arrivalAirport}
											</div>
										{:else if flight.endLocationCity || flight.endLocationState || flight.endLocationCountry}
											<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
												<MapPin class="h-3 w-3" />
												{#if flight.endLocationCountry}
													<img
														src={getFlagPath(flight.endLocationCountry)}
														alt=""
														class="inline-block h-3 rounded-sm"
													/>
												{/if}
												{[
													flight.endLocationCity,
													flight.endLocationState,
													flight.endLocationCountry
												]
													.filter(Boolean)
													.join(', ')}
											</div>
										{/if}
									{/if}
								</div>
							</td>
						{/if}
						<td class="font-semibold">
							{calculateFlightDuration(
								flight.createdAt,
								flight.latestFixTimestamp,
								flight.landingTime
							)}
						</td>
						{#if showEnd}
							<td class="font-semibold">
								{formatDistance(flight.totalDistanceMeters)}
							</td>
						{/if}
						{#if !showEnd}
							<td>
								<div class="text-sm">
									{formatAltitude(flight.latestAltitudeMslFeet, flight.latestAltitudeAglFeet)}
								</div>
							</td>
							<td>
								<div class="flex items-center gap-1 text-sm">
									<Clock class="h-3 w-3" />
									{formatRelativeTime(flight.latestFixTimestamp)}
								</div>
							</td>
						{/if}
						<td>
							{#if flight.towedByAircraftId}
								<TowAircraftLink aircraftId={flight.towedByAircraftId} size="sm" />
							{:else}
								<span class="text-surface-500">—</span>
							{/if}
						</td>
						<td>
							<a
								href={`/flights/${flight.id}`}
								target="_blank"
								rel="noopener noreferrer"
								class="btn flex items-center gap-2 preset-filled-primary-500"
							>
								<ExternalLink class="h-4 w-4" />
								<span class="font-semibold">View Flight</span>
							</a>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>

<!-- Mobile Cards -->
<div class="block space-y-4 md:hidden">
	{#each flightDetails as { flight, aircraft } (flight.id)}
		<div class="relative card p-4 transition-all duration-200 hover:shadow-lg">
			<!-- Aircraft info -->
			<div
				class="border-surface-200-700-token mb-3 flex flex-col gap-3 border-b pb-3 sm:flex-row sm:items-start sm:justify-between"
			>
				<div class="flex flex-wrap items-center gap-2">
					{#if showAircraft}
						{#if aircraft}
							<AircraftLink {aircraft} size="md" />
						{:else}
							<span class="font-semibold"
								>{flight.registration ||
									formatAircraftAddress(flight.deviceAddress, flight.deviceAddressType)}</span
							>
						{/if}
					{/if}
					{#if flight.aircraftCategory}
						<span class="badge {getAircraftCategoryColor(flight.aircraftCategory)} text-xs">
							{getAircraftCategoryDescription(flight.aircraftCategory)}
						</span>
					{/if}
					{#if flight.towedByAircraftId}
						<span
							class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
							title="This aircraft was towed"
						>
							<MoveUp class="h-3 w-3" />
							Towed
						</span>
					{/if}
				</div>
				<a
					href={`/flights/${flight.id}`}
					class="btn flex flex-shrink-0 items-center gap-2 preset-filled-primary-500"
				>
					<ExternalLink class="h-4 w-4" />
					<span class="font-semibold">View Flight</span>
				</a>
			</div>

			<!-- Flight details -->
			<div class="text-surface-600-300-token space-y-2 text-sm">
				<div>
					<span class="text-surface-500-400-token text-xs">Start:</span>
					{#if flight.departureAirport}
						<span class="font-medium">
							{#if flight.departureAirportCountry}
								<img
									src={getFlagPath(flight.departureAirportCountry)}
									alt=""
									class="inline-block h-3 rounded-sm"
								/>
							{/if}
							{flight.departureAirport}{#if flight.takeoffRunwayIdent}/{flight.takeoffRunwayIdent}{/if}
						</span>
					{:else if flight.startLocationCity || flight.startLocationState || flight.startLocationCountry}
						<span class="font-medium">
							{#if flight.startLocationCountry}
								<img
									src={getFlagPath(flight.startLocationCountry)}
									alt=""
									class="inline-block h-3 rounded-sm"
								/>
							{/if}
							{[flight.startLocationCity, flight.startLocationState, flight.startLocationCountry]
								.filter(Boolean)
								.join(', ')}
						</span>
					{/if}
					{formatLocalTime(isAirborne(flight) ? flight.createdAt : flight.takeoffTime)}
					<span class="text-surface-500-400-token text-xs">
						({formatRelativeTime(isAirborne(flight) ? flight.createdAt : flight.takeoffTime)})
					</span>
					{#if isAirborne(flight)}
						<span
							class="badge preset-filled-surface-500 text-xs"
							title="First detected while airborne"
						>
							Airborne
						</span>
					{/if}
				</div>
				{#if showEnd}
					<div>
						<span class="text-surface-500-400-token text-xs">End:</span>
						{#if isTimedOut(flight)}
							{#if flight.latestFixTimestamp}
								{formatLocalTime(flight.latestFixTimestamp)}
								<span class="text-surface-500-400-token text-xs">
									({formatRelativeTime(flight.latestFixTimestamp)})
								</span>
							{/if}
							<span
								class="badge preset-filled-warning-500 text-xs"
								title="No beacons received for 1+ hour"
							>
								<AlertCircle class="mr-1 inline h-3 w-3" />
								Timed out
							</span>
						{:else if isActive(flight)}
							<span class="text-surface-500-400-token">In progress</span>
						{:else}
							{#if flight.arrivalAirport}
								<span class="font-medium">
									{#if flight.arrivalAirportCountry}
										<img
											src={getFlagPath(flight.arrivalAirportCountry)}
											alt=""
											class="inline-block h-3 rounded-sm"
										/>
									{/if}
									{flight.arrivalAirport}{#if flight.landingRunwayIdent}/{flight.landingRunwayIdent}{/if}
								</span>
							{:else if flight.endLocationCity || flight.endLocationState || flight.endLocationCountry}
								<span class="font-medium">
									{#if flight.endLocationCountry}
										<img
											src={getFlagPath(flight.endLocationCountry)}
											alt=""
											class="inline-block h-3 rounded-sm"
										/>
									{/if}
									{[flight.endLocationCity, flight.endLocationState, flight.endLocationCountry]
										.filter(Boolean)
										.join(', ')}
								</span>
							{/if}
							{formatLocalTime(flight.landingTime)}
							<span class="text-surface-500-400-token text-xs">
								({formatRelativeTime(flight.landingTime)})
							</span>
						{/if}
					</div>
				{/if}
				<div class="flex gap-4">
					<div>
						<span class="text-surface-500-400-token text-xs">Duration:</span>
						<span class="font-semibold"
							>{calculateFlightDuration(
								flight.createdAt,
								flight.latestFixTimestamp,
								flight.landingTime
							)}</span
						>
					</div>
					{#if showEnd}
						<div>
							<span class="text-surface-500-400-token text-xs">Distance:</span>
							<span class="font-semibold">{formatDistance(flight.totalDistanceMeters)}</span>
						</div>
					{/if}
				</div>
				{#if !showEnd && (flight.latestAltitudeMslFeet !== null || flight.latestAltitudeAglFeet !== null)}
					<div>
						<span class="text-surface-500-400-token text-xs">Altitude:</span>
						{formatAltitude(flight.latestAltitudeMslFeet, flight.latestAltitudeAglFeet)}
					</div>
				{/if}
				{#if !showEnd && flight.latestFixTimestamp}
					<div>
						<span class="text-surface-500-400-token text-xs">Latest fix:</span>
						{formatRelativeTime(flight.latestFixTimestamp)}
					</div>
				{/if}
			</div>
		</div>
	{/each}
</div>
