<script lang="ts">
	import { Activity, Loader2 } from '@lucide/svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { SvelteMap } from 'svelte/reactivity';
	import type { Fix, FixWithExtras, RawMessageResponse } from '$lib/types';
	import { serverCall } from '$lib/api/server';

	dayjs.extend(relativeTime);

	interface Props {
		fixes: FixWithExtras[];
		loading?: boolean;
		showHideInactive?: boolean; // Show "hide inactive" checkbox
		showRaw?: boolean; // Show "show raw" checkbox
		useRelativeTimes?: boolean; // Use relative time formatting
		showIntervals?: boolean; // Show "show intervals" checkbox
		showClimb?: boolean; // Show climb column
		emptyMessage?: string;
		hideInactiveValue?: boolean; // Controlled value for hide inactive
		showRawValue?: boolean; // Controlled value for show raw
		onHideInactiveChange?: (value: boolean) => void; // Callback when hide inactive changes
		fixesInChronologicalOrder?: boolean; // True if fixes are in chronological order (earliest first), false if reverse chronological (newest first)
	}

	let {
		fixes,
		loading = false,
		showHideInactive = true,
		showRaw = true,
		useRelativeTimes = false,
		showIntervals = true,
		showClimb = false,
		emptyMessage = 'No position fixes found',
		hideInactiveValue = false,
		showRawValue = false,
		onHideInactiveChange,
		fixesInChronologicalOrder = true
	}: Props = $props();

	let showRawData = $state(showRawValue);
	let useRelativeTime = $derived(useRelativeTimes);
	let showTimeIntervals = $state(false);

	// Cache for raw messages: rawMessageId -> { data, loading, error }
	let rawMessagesCache = new SvelteMap<
		string,
		{
			data?: RawMessageResponse;
			loading: boolean;
			error?: string;
		}
	>();

	// Fetch raw message for a fix
	async function fetchRawMessage(rawMessageId: string) {
		if (rawMessagesCache.get(rawMessageId)?.data || rawMessagesCache.get(rawMessageId)?.loading) {
			return; // Already fetched or loading
		}

		rawMessagesCache.set(rawMessageId, { loading: true });

		try {
			const response = await serverCall<{ data: RawMessageResponse }>(
				`/raw-messages/${rawMessageId}`
			);
			rawMessagesCache.set(rawMessageId, { data: response.data, loading: false });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Failed to fetch';
			rawMessagesCache.set(rawMessageId, { loading: false, error: errorMessage });
		}
	}

	// Fetch all raw messages when show raw is toggled on
	$effect(() => {
		if (showRawData && fixes.length > 0) {
			// Fetch raw messages for all visible fixes
			for (const fix of fixes) {
				fetchRawMessage(fix.rawMessageId);
			}
		}
	});

	// Get raw message display for a fix
	function getRawMessageDisplay(rawMessageId: string): {
		content?: string;
		loading: boolean;
		error?: string;
		source?: 'aprs' | 'adsb';
	} {
		const cached = rawMessagesCache.get(rawMessageId);
		if (!cached) {
			return { loading: false };
		}
		return {
			content: cached.data?.rawMessage,
			loading: cached.loading,
			error: cached.error,
			source: cached.data?.source
		};
	}

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

	function calculateInterval(currentFix: Fix, previousFix: Fix | undefined): string {
		if (!previousFix) return 'N/A';

		// If fixes are in chronological order (earliest first), previousFix is the earlier timestamp
		// If fixes are in reverse chronological order (newest first), previousFix is the later timestamp
		// We always want to calculate: later timestamp - earlier timestamp for positive intervals

		const current = dayjs(currentFix.timestamp);
		const previous = dayjs(previousFix.timestamp);

		let diffSeconds: number;
		if (fixesInChronologicalOrder) {
			// Chronological order: current is later than previous
			diffSeconds = current.diff(previous, 'second');
		} else {
			// Reverse chronological: current is earlier than previous
			diffSeconds = previous.diff(current, 'second');
		}

		// Handle negative values (shouldn't happen, but just in case)
		diffSeconds = Math.abs(diffSeconds);

		const hours = Math.floor(diffSeconds / 3600);
		const minutes = Math.floor((diffSeconds % 3600) / 60);
		const seconds = diffSeconds % 60;

		// Format as HH:MM:SS
		const hoursStr = hours.toString().padStart(2, '0');
		const minutesStr = minutes.toString().padStart(2, '0');
		const secondsStr = seconds.toString().padStart(2, '0');

		return `${hoursStr}:${minutesStr}:${secondsStr}`;
	}

	function isIntervalOverHour(currentFix: Fix, previousFix: Fix | undefined): boolean {
		if (!previousFix) return false;
		const current = dayjs(currentFix.timestamp);
		const previous = dayjs(previousFix.timestamp);

		let diffSeconds: number;
		if (fixesInChronologicalOrder) {
			diffSeconds = current.diff(previous, 'second');
		} else {
			diffSeconds = previous.diff(current, 'second');
		}

		diffSeconds = Math.abs(diffSeconds);
		return diffSeconds >= 3600;
	}
</script>

<div class="space-y-4">
	<!-- Controls -->
	{#if fixes.length > 0 && (showHideInactive || showRaw || useRelativeTimes || showIntervals)}
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
			{#if showIntervals}
				<label class="flex cursor-pointer items-center gap-2 text-sm">
					<input type="checkbox" class="checkbox" bind:checked={showTimeIntervals} />
					<span>Show intervals</span>
				</label>
			{/if}
		</div>
	{/if}

	<!-- Loading State -->
	{#if loading}
		<div class="flex items-center justify-center py-8">
			<div
				class="mx-auto h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
			<span class="ml-2">Loading position fixes...</span>
		</div>
	{:else if fixes.length === 0}
		<!-- Empty State -->
		<div class="text-surface-600-300-token py-8 text-center">
			<Activity class="mx-auto mb-4 h-12 w-12 text-surface-400" />
			<p>{emptyMessage}</p>
		</div>
	{:else}
		<!-- Desktop: Table -->
		<div class="hidden overflow-x-auto md:block">
			<table class="w-full table-auto">
				<thead class="bg-surface-100-800-token border-surface-300-600-token border-b">
					<tr>
						<th class="px-3 py-2 text-left text-sm font-medium">Time</th>
						{#if showTimeIntervals}
							<th class="px-3 py-2 text-left text-sm font-medium">Interval</th>
						{/if}
						<th class="px-3 py-2 text-left text-sm font-medium">Coordinates</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Altitude MSL</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Altitude AGL</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Speed</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Track</th>
						{#if showClimb}
							<th class="px-3 py-2 text-left text-sm font-medium">Climb</th>
						{/if}
						<th class="px-3 py-2 text-left text-sm font-medium">Squawk</th>
						<th class="px-3 py-2 text-left text-sm font-medium">Flight #</th>
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
							{#if showTimeIntervals}
								<td
									class="px-3 py-2 text-sm {isIntervalOverHour(fix, fixes[index - 1])
										? 'font-semibold text-error-500'
										: ''}"
								>
									{calculateInterval(fix, fixes[index - 1])}
								</td>
							{/if}
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
							<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitudeMslFeet)}</td>
							<td class="px-3 py-2 text-sm">{formatAltitude(fix.altitudeAglFeet)}</td>
							<td class="px-3 py-2 text-sm">{formatSpeed(fix.groundSpeedKnots)}</td>
							<td class="px-3 py-2 text-sm">{formatTrack(fix.trackDegrees)}</td>
							{#if showClimb}
								<td class="px-3 py-2 text-sm">{formatClimb(fix.climbFpm)}</td>
							{/if}
							<td class="px-3 py-2 font-mono text-sm">{fix.squawk || 'N/A'}</td>
							<td class="px-3 py-2 font-mono text-sm">{fix.flightNumber || 'N/A'}</td>
						</tr>
						{#if showRawData}
							{@const rawDisplay = getRawMessageDisplay(fix.rawMessageId)}
							<tr
								class="border-surface-200-700-token border-b {index % 2 === 0
									? 'bg-surface-100-800-token'
									: ''}"
							>
								<td
									colspan={showClimb ? (showTimeIntervals ? 10 : 9) : showTimeIntervals ? 9 : 8}
									class="px-3 py-2 font-mono text-sm"
								>
									{#if rawDisplay.loading}
										<span class="inline-flex items-center gap-1 text-surface-500">
											<Loader2 class="h-3 w-3 animate-spin" />
											Loading...
										</span>
									{:else if rawDisplay.error}
										<span class="text-error-500">Error: {rawDisplay.error}</span>
									{:else if rawDisplay.content}
										<span class="text-surface-400" title="Source: {rawDisplay.source}">
											[{rawDisplay.source?.toUpperCase()}]
										</span>
										{rawDisplay.content}
									{:else}
										<span class="text-surface-400">No raw message available</span>
									{/if}
								</td>
							</tr>
						{/if}
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Mobile: Cards -->
		<div class="space-y-4 md:hidden">
			{#each fixes as fix, index (fix.id)}
				<div class="card p-4 {!fix.active ? 'opacity-50' : ''}">
					<div class="mb-3 text-sm" title={formatDate(fix.timestamp)}>
						<div class="font-semibold">{formatFixTime(fix.timestamp)}</div>
						<div class="text-surface-500-400-token text-xs">{formatDate(fix.timestamp)}</div>
						{#if showTimeIntervals}
							<div
								class="text-xs {isIntervalOverHour(fix, fixes[index - 1])
									? 'font-semibold text-error-500'
									: 'text-surface-500-400-token'}"
							>
								Interval: {calculateInterval(fix, fixes[index - 1])}
							</div>
						{/if}
					</div>

					<dl class="mb-3 space-y-2 text-sm">
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Coordinates</dt>
							<dd class="font-mono text-xs">
								<a
									href={getGoogleMapsUrl(fix.latitude, fix.longitude)}
									target="_blank"
									rel="noopener noreferrer"
									class="text-primary-500 hover:text-primary-600 hover:underline"
								>
									{formatCoordinates(fix.latitude, fix.longitude)}
								</a>
							</dd>
						</div>
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Altitude MSL</dt>
							<dd class="font-medium">{formatAltitude(fix.altitudeMslFeet)}</dd>
						</div>
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Altitude AGL</dt>
							<dd class="font-medium">{formatAltitude(fix.altitudeAglFeet)}</dd>
						</div>
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Speed</dt>
							<dd class="font-medium">{formatSpeed(fix.groundSpeedKnots)}</dd>
						</div>
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Track</dt>
							<dd class="font-medium">{formatTrack(fix.trackDegrees)}</dd>
						</div>
						{#if showClimb}
							<div class="flex justify-between gap-4">
								<dt class="text-surface-600-300-token">Climb</dt>
								<dd class="font-medium">{formatClimb(fix.climbFpm)}</dd>
							</div>
						{/if}
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Squawk</dt>
							<dd class="font-mono font-medium">{fix.squawk || 'N/A'}</dd>
						</div>
						<div class="flex justify-between gap-4">
							<dt class="text-surface-600-300-token">Flight Number</dt>
							<dd class="font-mono font-medium">{fix.flightNumber || 'N/A'}</dd>
						</div>
					</dl>

					{#if showRawData}
						{@const rawDisplay = getRawMessageDisplay(fix.rawMessageId)}
						<div class="border-t border-surface-300 pt-3 dark:border-surface-600">
							<div class="text-surface-600-300-token mb-1 text-xs">
								Raw Packet
								{#if rawDisplay.source}
									<span class="text-surface-400">({rawDisplay.source.toUpperCase()})</span>
								{/if}
							</div>
							{#if rawDisplay.loading}
								<div class="inline-flex items-center gap-1 text-surface-500">
									<Loader2 class="h-3 w-3 animate-spin" />
									<span>Loading...</span>
								</div>
							{:else if rawDisplay.error}
								<div class="text-xs text-error-500">Error: {rawDisplay.error}</div>
							{:else if rawDisplay.content}
								<div class="overflow-x-auto font-mono text-xs">{rawDisplay.content}</div>
							{:else}
								<div class="text-xs text-surface-400">No raw message available</div>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
