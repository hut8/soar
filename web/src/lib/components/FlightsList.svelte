<script lang="ts">
	import { MapPin, Clock, ExternalLink, MoveUp, AlertCircle } from '@lucide/svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import {
		getAircraftTypeOgnDescription,
		getAircraftTypeColor,
		getFlagPath
	} from '$lib/formatters';
	import type { Flight } from '$lib/types';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';
	import AircraftLink from '$lib/components/AircraftLink.svelte';

	dayjs.extend(relativeTime);

	interface Props {
		flights: Flight[];
		showEnd?: boolean; // If true, show End column (for completed flights)
		showAircraft?: boolean; // If true, show aircraft column
	}

	let { flights, showEnd = false, showAircraft = true }: Props = $props();

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
		// Always use created_at as start time
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
		return !flight.takeoff_time;
	}

	// Check if flight timed out
	function isTimedOut(flight: Flight): boolean {
		return !!flight.timed_out_at && !flight.landing_time;
	}

	// Check if flight is still active
	function isActive(flight: Flight): boolean {
		return !flight.landing_time && !flight.timed_out_at;
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
				{#each flights as flight (flight.id)}
					<tr>
						{#if showAircraft}
							<td>
								<div class="flex flex-col gap-1">
									<div class="flex items-center gap-2">
										{#if flight.aircraft_id}
											<AircraftLink aircraft={flight} size="md" />
										{:else}
											<span class="font-medium"
												>{flight.registration ||
													formatAircraftAddress(
														flight.device_address,
														flight.device_address_type
													)}</span
											>
										{/if}
										{#if flight.towed_by_aircraft_id}
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
								{#if flight.aircraft_type_ogn}
									<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
										{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
									</span>
								{:else}
									<span class="text-surface-500">—</span>
								{/if}
							</td>
						{/if}
						<td>
							<div class="flex flex-col gap-1">
								<div class="text-sm">
									{formatRelativeTime(isAirborne(flight) ? flight.created_at : flight.takeoff_time)}
								</div>
								<div class="flex items-center gap-1 text-xs">
									<span class="text-surface-500-400-token">
										{formatLocalTime(isAirborne(flight) ? flight.created_at : flight.takeoff_time)}
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
								{#if flight.departure_airport}
									<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
										<MapPin class="h-3 w-3" />
										{#if flight.departure_airport_country}
											<img
												src={getFlagPath(flight.departure_airport_country)}
												alt=""
												class="inline-block h-3 rounded-sm"
											/>
										{/if}
										{flight.departure_airport}
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
											{formatRelativeTime(flight.latest_fix_timestamp)}
										</div>
										{#if flight.latest_fix_timestamp}
											<div class="text-surface-500-400-token text-xs">
												{formatLocalTime(flight.latest_fix_timestamp)}
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
											{formatRelativeTime(flight.landing_time)}
										</div>
										{#if flight.landing_time}
											<div class="text-surface-500-400-token text-xs">
												{formatLocalTime(flight.landing_time)}
											</div>
										{/if}
										{#if flight.arrival_airport}
											<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
												<MapPin class="h-3 w-3" />
												{#if flight.arrival_airport_country}
													<img
														src={getFlagPath(flight.arrival_airport_country)}
														alt=""
														class="inline-block h-3 rounded-sm"
													/>
												{/if}
												{flight.arrival_airport}
											</div>
										{/if}
									{/if}
								</div>
							</td>
						{/if}
						<td class="font-semibold">
							{calculateFlightDuration(
								flight.created_at,
								flight.latest_fix_timestamp,
								flight.landing_time
							)}
						</td>
						{#if showEnd}
							<td class="font-semibold">
								{formatDistance(flight.total_distance_meters)}
							</td>
						{/if}
						{#if !showEnd}
							<td>
								<div class="text-sm">
									{formatAltitude(flight.latest_altitude_msl_feet, flight.latest_altitude_agl_feet)}
								</div>
							</td>
							<td>
								<div class="flex items-center gap-1 text-sm">
									<Clock class="h-3 w-3" />
									{formatRelativeTime(flight.latest_fix_timestamp)}
								</div>
							</td>
						{/if}
						<td>
							{#if flight.towed_by_aircraft_id}
								<TowAircraftLink aircraftId={flight.towed_by_aircraft_id} size="sm" />
							{:else}
								<span class="text-surface-500">—</span>
							{/if}
						</td>
						<td>
							<a
								href={`/flights/${flight.id}`}
								target="_blank"
								rel="noopener noreferrer"
								class="btn flex items-center gap-1 preset-filled-primary-500 btn-sm"
							>
								<ExternalLink class="h-3 w-3" />
								Open
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
	{#each flights as flight (flight.id)}
		<div class="relative card p-4 transition-all duration-200 hover:shadow-lg">
			<!-- Aircraft info -->
			<div class="border-surface-200-700-token mb-3 flex items-start justify-between border-b pb-3">
				<div class="flex flex-wrap items-center gap-2">
					{#if showAircraft}
						{#if flight.aircraft_id}
							<AircraftLink aircraft={flight} size="md" />
						{:else}
							<span class="font-semibold"
								>{flight.registration ||
									formatAircraftAddress(flight.device_address, flight.device_address_type)}</span
							>
						{/if}
					{/if}
					{#if flight.aircraft_type_ogn}
						<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
							{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
						</span>
					{/if}
					{#if flight.towed_by_aircraft_id}
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
					class="relative z-10 flex-shrink-0"
					title="View flight details"
				>
					<ExternalLink class="h-4 w-4 text-surface-400 hover:text-primary-500" />
				</a>
			</div>

			<!-- Flight details -->
			<div class="text-surface-600-300-token space-y-2 text-sm">
				<div>
					<span class="text-surface-500-400-token text-xs">Start:</span>
					{#if flight.departure_airport}
						<span class="font-medium">
							{#if flight.departure_airport_country}
								<img
									src={getFlagPath(flight.departure_airport_country)}
									alt=""
									class="inline-block h-3 rounded-sm"
								/>
							{/if}
							{flight.departure_airport}{#if flight.takeoff_runway_ident}/{flight.takeoff_runway_ident}{/if}
						</span>
					{/if}
					{formatLocalTime(isAirborne(flight) ? flight.created_at : flight.takeoff_time)}
					<span class="text-surface-500-400-token text-xs">
						({formatRelativeTime(isAirborne(flight) ? flight.created_at : flight.takeoff_time)})
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
							{#if flight.latest_fix_timestamp}
								{formatLocalTime(flight.latest_fix_timestamp)}
								<span class="text-surface-500-400-token text-xs">
									({formatRelativeTime(flight.latest_fix_timestamp)})
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
							{#if flight.arrival_airport}
								<span class="font-medium">
									{#if flight.arrival_airport_country}
										<img
											src={getFlagPath(flight.arrival_airport_country)}
											alt=""
											class="inline-block h-3 rounded-sm"
										/>
									{/if}
									{flight.arrival_airport}{#if flight.landing_runway_ident}/{flight.landing_runway_ident}{/if}
								</span>
							{/if}
							{formatLocalTime(flight.landing_time)}
							<span class="text-surface-500-400-token text-xs">
								({formatRelativeTime(flight.landing_time)})
							</span>
						{/if}
					</div>
				{/if}
				<div class="flex gap-4">
					<div>
						<span class="text-surface-500-400-token text-xs">Duration:</span>
						<span class="font-semibold"
							>{calculateFlightDuration(
								flight.created_at,
								flight.latest_fix_timestamp,
								flight.landing_time
							)}</span
						>
					</div>
					{#if showEnd}
						<div>
							<span class="text-surface-500-400-token text-xs">Distance:</span>
							<span class="font-semibold">{formatDistance(flight.total_distance_meters)}</span>
						</div>
					{/if}
				</div>
				{#if !showEnd && (flight.latest_altitude_msl_feet !== null || flight.latest_altitude_agl_feet !== null)}
					<div>
						<span class="text-surface-500-400-token text-xs">Altitude:</span>
						{formatAltitude(flight.latest_altitude_msl_feet, flight.latest_altitude_agl_feet)}
					</div>
				{/if}
				{#if !showEnd && flight.latest_fix_timestamp}
					<div>
						<span class="text-surface-500-400-token text-xs">Latest fix:</span>
						{formatRelativeTime(flight.latest_fix_timestamp)}
					</div>
				{/if}
			</div>
		</div>
	{/each}
</div>
