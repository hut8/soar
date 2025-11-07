<script lang="ts">
	import { MapPin, Clock, ExternalLink, MoveUp, Radio, AlertCircle } from '@lucide/svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { getAircraftTypeOgnDescription, getAircraftTypeColor } from '$lib/formatters';
	import type { Flight } from '$lib/types';
	import TowAircraftLink from '$lib/components/TowAircraftLink.svelte';

	dayjs.extend(relativeTime);

	interface Props {
		flights: Flight[];
		showEnd?: boolean; // If true, show End column (for completed flights)
		showAircraft?: boolean; // If true, show aircraft column
	}

	let { flights, showEnd = false, showAircraft = true }: Props = $props();

	function formatDeviceAddress(address: string, addressType: string): string {
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
		takeoff: string | null | undefined,
		landing: string | null | undefined,
		timedOut: string | null | undefined
	): string {
		// If no takeoff time, use created_at if available
		const startTime = takeoff;
		if (!startTime) return '—';

		const takeoffTime = new Date(startTime).getTime();
		let endTime: number;

		// Determine end time: landing, timeout, or now
		if (landing) {
			endTime = new Date(landing).getTime();
		} else if (timedOut) {
			endTime = new Date(timedOut).getTime();
		} else {
			// Still flying - use now
			endTime = new Date().getTime();
		}

		const durationMs = endTime - takeoffTime;
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
					{#if !showEnd}
						<th>Recognized</th>
					{/if}
					<th>Start</th>
					{#if showEnd}
						<th>End</th>
					{/if}
					<th>Duration</th>
					<th>Distance</th>
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
									{#if flight.aircraft_model && flight.registration}
										<div class="flex items-center gap-2">
											{#if flight.device_id}
												<a
													href={`/devices/${flight.device_id}`}
													class="anchor font-medium text-primary-500 hover:text-primary-600"
												>
													{flight.aircraft_model}
													<span class="text-surface-500-400-token text-sm font-normal"
														>({flight.registration})</span
													>
												</a>
											{:else}
												<span class="font-medium"
													>{flight.aircraft_model}
													<span class="text-surface-500-400-token text-sm font-normal"
														>({flight.registration})</span
													></span
												>
											{/if}
											{#if flight.towed_by_device_id}
												<span
													class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
													title="This aircraft was towed"
												>
													<MoveUp class="h-3 w-3" />
													Towed
												</span>
											{/if}
										</div>
									{:else if flight.registration}
										<div class="flex items-center gap-2">
											{#if flight.device_id}
												<a
													href={`/devices/${flight.device_id}`}
													class="anchor font-medium text-primary-500 hover:text-primary-600"
												>
													{flight.registration}
												</a>
											{:else}
												<span class="font-medium">{flight.registration}</span>
											{/if}
											{#if flight.towed_by_device_id}
												<span
													class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
													title="This aircraft was towed"
												>
													<MoveUp class="h-3 w-3" />
													Towed
												</span>
											{/if}
										</div>
									{:else}
										<div class="flex items-center gap-2">
											{#if flight.device_id}
												<a
													href={`/devices/${flight.device_id}`}
													class="text-surface-500-400-token anchor font-mono text-sm hover:text-primary-500"
												>
													{formatDeviceAddress(flight.device_address, flight.device_address_type)}
												</a>
											{:else}
												<span class="text-surface-500-400-token font-mono text-sm">
													{formatDeviceAddress(flight.device_address, flight.device_address_type)}
												</span>
											{/if}
											{#if flight.towed_by_device_id}
												<span
													class="badge flex items-center gap-1 preset-filled-primary-500 text-xs"
													title="This aircraft was towed"
												>
													<MoveUp class="h-3 w-3" />
													Towed
												</span>
											{/if}
										</div>
									{/if}
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
						{#if !showEnd}
							<td>
								<div class="flex flex-col gap-1">
									<div class="flex items-center gap-1 text-sm">
										<Clock class="h-3 w-3" />
										{formatRelativeTime(flight.created_at)}
									</div>
									{#if flight.created_at}
										<div class="text-surface-500-400-token text-xs">
											{formatLocalTime(flight.created_at)}
										</div>
									{/if}
								</div>
							</td>
						{/if}
						<td>
							<div class="flex flex-col gap-1">
								<div class="flex items-center gap-1 text-sm">
									{#if isAirborne(flight)}
										<Radio class="h-3 w-3" />
										<span
											class="badge preset-filled-tertiary-500 text-xs"
											title="First detected while airborne"
										>
											Airborne
										</span>
									{:else}
										<Clock class="h-3 w-3" />
										{formatRelativeTime(flight.takeoff_time)}
									{/if}
								</div>
								{#if flight.takeoff_time}
									<div class="text-surface-500-400-token text-xs">
										{formatLocalTime(flight.takeoff_time)}
									</div>
								{/if}
								{#if flight.departure_airport}
									<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
										<MapPin class="h-3 w-3" />
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
												{flight.arrival_airport}
											</div>
										{/if}
									{/if}
								</div>
							</td>
						{/if}
						<td class="font-semibold">
							{calculateFlightDuration(
								flight.takeoff_time,
								flight.landing_time,
								flight.timed_out_at
							)}
						</td>
						<td class="font-semibold">
							{formatDistance(flight.total_distance_meters)}
						</td>
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
							{#if flight.towed_by_device_id}
								<TowAircraftLink deviceId={flight.towed_by_device_id} size="sm" />
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
						{#if flight.aircraft_model && flight.registration}
							{#if flight.device_id}
								<a
									href={`/devices/${flight.device_id}`}
									class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
								>
									{flight.aircraft_model} ({flight.registration})
								</a>
							{:else}
								<span class="font-semibold">{flight.aircraft_model} ({flight.registration})</span>
							{/if}
						{:else if flight.registration}
							{#if flight.device_id}
								<a
									href={`/devices/${flight.device_id}`}
									class="relative z-10 anchor font-semibold text-primary-500 hover:text-primary-600"
								>
									{flight.registration}
								</a>
							{:else}
								<span class="font-semibold">{flight.registration}</span>
							{/if}
						{:else if flight.device_id}
							<a
								href={`/devices/${flight.device_id}`}
								class="text-surface-500-400-token relative z-10 anchor font-mono text-sm hover:text-primary-500"
							>
								{formatDeviceAddress(flight.device_address, flight.device_address_type)}
							</a>
						{:else}
							<span class="text-surface-500-400-token font-mono text-sm">
								{formatDeviceAddress(flight.device_address, flight.device_address_type)}
							</span>
						{/if}
					{/if}
					{#if flight.aircraft_type_ogn}
						<span class="badge {getAircraftTypeColor(flight.aircraft_type_ogn)} text-xs">
							{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
						</span>
					{/if}
					{#if flight.towed_by_device_id}
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
				{#if !showEnd}
					<div>
						<span class="text-surface-500-400-token text-xs">Recognized:</span>
						{formatLocalTime(flight.created_at)}
						<span class="text-surface-500-400-token text-xs">
							({formatRelativeTime(flight.created_at)})
						</span>
					</div>
				{/if}
				<div>
					<span class="text-surface-500-400-token text-xs">Start:</span>
					{#if isAirborne(flight)}
						<span
							class="badge preset-filled-tertiary-500 text-xs"
							title="First detected while airborne"
						>
							<Radio class="mr-1 inline h-3 w-3" />
							Airborne
						</span>
						{#if flight.created_at}
							{formatLocalTime(flight.created_at)}
						{/if}
					{:else}
						{#if flight.departure_airport}
							<span class="font-medium">
								{flight.departure_airport}{#if flight.takeoff_runway_ident}/{flight.takeoff_runway_ident}{/if}
							</span>
						{/if}
						{formatLocalTime(flight.takeoff_time)}
						<span class="text-surface-500-400-token text-xs">
							({formatRelativeTime(flight.takeoff_time)})
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
								flight.takeoff_time,
								flight.landing_time,
								flight.timed_out_at
							)}</span
						>
					</div>
					<div>
						<span class="text-surface-500-400-token text-xs">Distance:</span>
						<span class="font-semibold">{formatDistance(flight.total_distance_meters)}</span>
					</div>
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
