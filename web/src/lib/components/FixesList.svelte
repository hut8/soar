<script lang="ts">
	import { Activity } from '@lucide/svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import type { Fix } from '$lib/types';

	dayjs.extend(relativeTime);

	interface Props {
		fixes: Fix[];
		loading?: boolean;
		showHideInactive?: boolean; // Show "hide inactive" checkbox
		showRaw?: boolean; // Show "show raw" checkbox
		useRelativeTimes?: boolean; // Use relative time formatting
		showClimb?: boolean; // Show climb column
		emptyMessage?: string;
		hideInactiveValue?: boolean; // Controlled value for hide inactive
		showRawValue?: boolean; // Controlled value for show raw
		onHideInactiveChange?: (value: boolean) => void; // Callback when hide inactive changes
	}

	let {
		fixes,
		loading = false,
		showHideInactive = true,
		showRaw = true,
		useRelativeTimes = false,
		showClimb = false,
		emptyMessage = 'No position fixes found',
		hideInactiveValue = false,
		showRawValue = false,
		onHideInactiveChange
	}: Props = $props();

	let showRawData = $state(showRawValue);
	let useRelativeTime = $state(useRelativeTimes);

	function handleHideInactiveChange(event: Event) {
		const target = event.target as HTMLInputElement;
		if (onHideInactiveChange) {
			onHideInactiveChange(target.checked);
		}
	}

	function formatRelativeTime(dateString: string): string {
		return dayjs(dateString).fromNow();
	}

	function formatDate(dateString: string): string {
		return dayjs(dateString).format('YYYY-MM-DD HH:mm:ss');
	}

	function formatLocalTime(dateString: string): string {
		return dayjs(dateString).format('HH:mm:ss');
	}

	function formatFixTime(dateString: string): string {
		if (useRelativeTime) {
			return formatRelativeTime(dateString);
		}
		return formatLocalTime(dateString);
	}

	function formatCoordinates(lat: number, lon: number): string {
		return `${lat.toFixed(4)}, ${lon.toFixed(4)}`;
	}

	function getGoogleMapsUrl(lat: number, lon: number): string {
		return `https://www.google.com/maps?q=${lat},${lon}`;
	}

	function formatAltitude(altitude: number | null | undefined): string {
		if (altitude === null || altitude === undefined) return 'N/A';
		return `${altitude.toLocaleString()} ft`;
	}

	function formatSpeed(speed: number | null | undefined): string {
		if (speed === null || speed === undefined) return 'N/A';
		return `${speed.toFixed(1)} kt`;
	}

	function formatTrack(track: number | null | undefined): string {
		if (track === null || track === undefined) return 'N/A';
		return `${track.toFixed(0)}°`;
	}

	function formatClimb(climb: number | null | undefined): string {
		if (climb === null || climb === undefined) return 'N/A';
		return `${climb.toFixed(0)} fpm`;
	}
</script>

<div class="space-y-4">
	<!-- Controls -->
	{#if fixes.length > 0 && (showHideInactive || showRaw || useRelativeTimes)}
		<div class="flex flex-wrap gap-4">
			{#if showHideInactive}
				<label class="flex items-center gap-2 text-sm">
					<input
						type="checkbox"
						class="checkbox"
						checked={hideInactiveValue}
						onchange={handleHideInactiveChange}
					/>
					<span>Hide inactive fixes</span>
				</label>
			{/if}
			{#if showRaw}
				<label class="flex cursor-pointer items-center gap-2 text-sm">
					<input type="checkbox" class="checkbox" bind:checked={showRawData} />
					<span>Show raw</span>
				</label>
			{/if}
			{#if useRelativeTimes}
				<label class="flex cursor-pointer items-center gap-2 text-sm">
					<input type="checkbox" class="checkbox" bind:checked={useRelativeTime} />
					<span>Relative times</span>
				</label>
			{/if}
		</div>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-8">
			<div
				class="border-primary-500 mx-auto h-8 w-8 animate-spin rounded-full border-4 border-t-transparent"
			></div>
			<span class="ml-2">Loading position fixes...</span>
		</div>
	{:else if fixes.length === 0}
		<!-- Empty State -->
		<div class="text-surface-600-300-token py-8 text-center">
			<Activity class="text-surface-400 mx-auto mb-4 h-12 w-12" />
			<p>{emptyMessage}</p>
		</div>
	{:else}
		<!-- Table -->
		<div class="overflow-x-auto">
			<table class="w-full table-auto">
				<thead class="bg-surface-100-800-token border-surface-300-600-token border-b">
					<tr>
						<th class="px-3 py-2 text-left text-sm font-medium">Time</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Coordinates</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Altitude MSL</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Altitude AGL</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Speed</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Track</th>
						{#if showClimb}
							<th class="px-3 py-2 text-left text-sm font-medium">Climb</th>
						{/if}
					</tr>
				</thead>
				<tbody>
					{#each fixes as fix, index (fix.id)}
						<tr
							class="border-surface-200-700-token hover:bg-surface-100-800-token border-b {index %
								2 ===
							0
								? 'bg-surface-50-900-token'
								: ''} {!fix.active ? 'opacity-50' : ''}"
						>
							<td class="px-3 py-2 text-sm" title={formatDate(fix.timestamp)}>
								{formatFixTime(fix.timestamp)}
							</td>
							<td class="px-3 py-2 font-mono text-sm">
								<a
									href={getGoogleMapsUrl(fix.latitude, fix.longitude)}
									target="_blank"
									rel="noopener noreferrer"
									class="text-primary-500 hover:text-primary-600 hover:underline"
								>
									{formatCoordinates(fix.latitude, fix.longitude)}
								</a>
							</td>
							<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitude_msl_feet)}</td>
							<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitude_agl_feet)}</td>
							<td class="px-3 py-2 text-sm">{formatSpeed(fix.ground_speed_knots)}</td>
							<td class="px-3 py-2 text-sm">{formatTrack(fix.track_degrees)}</td>
							{#if showClimb}
								<td class="px-3 py-2 text-sm">{formatClimb(fix.climb_fpm)}</td>
							{/if}
						</tr>
						{#if showRawData && fix.raw_packet}
							<tr
								class="border-surface-200-700-token border-b {index % 2 === 0
									? 'bg-surface-100-800-token'
									: ''}"
							>
								<td colspan={showClimb ? 7 : 6} class="px-3 py-2 font-mono text-sm">
									{fix.raw_packet}
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
