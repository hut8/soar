<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Radio,
		MapPin,
		Navigation,
		Info,
		ExternalLink,
		Mail,
		User,
		Globe,
		Calendar,
		Signal,
		ChevronLeft,
		ChevronRight,
		FileText
	} from '@lucide/svelte';
	import { Progress, Tabs } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import type { Device } from '$lib/types';
	import {
		formatDeviceAddress,
		getAircraftTypeOgnDescription,
		getAircraftTypeColor
	} from '$lib/formatters';

	dayjs.extend(relativeTime);

	interface Receiver {
		id: string;
		callsign: string;
		description: string | null;
		contact: string | null;
		email: string | null;
		country: string | null;
		latitude: number | null;
		longitude: number | null;
		created_at: string;
		updated_at: string;
		from_ogn_db: boolean;
	}

	interface Fix {
		id: string;
		timestamp: string;
		latitude: number;
		longitude: number;
		altitude_msl_feet: number | null;
		device_address: number;
		ground_speed_knots: number | null;
		track_degrees: number | null;
		climb_fpm: number | null;
		snr_db: number | null;
		registration: string | null;
	}

	interface FixesResponse {
		fixes: Fix[];
		page: number;
		total_pages: number;
	}

	interface ReceiverStatus {
		id: string;
		received_at: string;
		version: string | null;
		platform: string | null;
		cpu_load: number | string | null; // BigDecimal from backend
		cpu_temperature: number | string | null; // BigDecimal from backend
		ram_free: number | string | null; // BigDecimal from backend
		ram_total: number | string | null; // BigDecimal from backend
		visible_senders: number | null;
		senders: number | null;
		voltage: number | string | null; // BigDecimal from backend
		amperage: number | string | null; // BigDecimal from backend
		lag: number | null;
		raw_data: string;
	}

	interface StatusesResponse {
		statuses: ReceiverStatus[];
		page: number;
		total_pages: number;
	}

	interface RawMessage {
		id: string;
		raw_message: string;
		received_at: string;
		receiver_id: string;
		unparsed: string | null;
	}

	interface RawMessagesResponse {
		messages: RawMessage[];
		page: number;
		total_pages: number;
	}

	interface AprsTypeCount {
		aprs_type: string;
		count: number;
	}

	interface DeviceFixCount {
		device_id: string;
		count: number;
		device?: Device | null; // Device details fetched separately
	}

	interface ReceiverStatistics {
		average_update_interval_seconds: number | null;
		total_status_count: number;
		days_included: number | null;
	}

	interface AggregateStatsResponse {
		fix_counts_by_aprs_type: AprsTypeCount[];
		fix_counts_by_device: DeviceFixCount[];
	}

	let receiver = $state<Receiver | null>(null);
	let fixes = $state<Fix[] | null>(null);
	let statuses = $state<ReceiverStatus[]>([]);
	let rawMessages = $state<RawMessage[] | null>(null);
	let statistics = $state<ReceiverStatistics | null>(null);
	let aggregateStats = $state<AggregateStatsResponse | null>(null);
	let loading = $state(true);
	let loadingFixes = $state(false);
	let loadingStatuses = $state(false);
	let loadingRawMessages = $state(false);
	let loadingStatistics = $state(false);
	let loadingAggregateStats = $state(false);
	let error = $state('');
	let fixesError = $state('');
	let statusesError = $state('');
	let rawMessagesError = $state('');
	let statisticsError = $state('');
	let aggregateStatsError = $state('');

	let fixesPage = $state(1);
	let fixesTotalPages = $state(1);
	let statusesPage = $state(1);
	let statusesTotalPages = $state(1);
	let rawMessagesPage = $state(1);
	let rawMessagesTotalPages = $state(1);

	// Display options
	let showRawData = $state(false);
	let activeTab = $state('status-reports'); // 'status-reports', 'raw-messages', 'received-fixes', or 'aggregate-stats'

	let receiverId = $derived($page.params.id || '');

	onMount(async () => {
		if (receiverId) {
			await loadReceiver();
			await loadStatuses(); // Load status reports by default (first tab)
			await loadStatistics(); // Load statistics (cheap and important)
			// Aggregate stats, fixes, and raw messages are loaded lazily when their tabs are clicked
		}
	});

	// Load raw messages when switching to that tab
	$effect(() => {
		if (activeTab === 'raw-messages' && receiverId && rawMessages === null && !loadingRawMessages) {
			loadRawMessages();
		}
	});

	// Load fixes when switching to that tab (if not already loaded)
	$effect(() => {
		if (activeTab === 'received-fixes' && receiverId && fixes === null && !loadingFixes) {
			loadFixes();
		}
	});

	// Load aggregate stats when switching to aggregate stats tab
	$effect(() => {
		if (
			activeTab === 'aggregate-stats' &&
			receiverId &&
			aggregateStats === null &&
			!loadingAggregateStats
		) {
			loadAggregateStats();
		}
	});

	async function loadReceiver() {
		loading = true;
		error = '';

		try {
			receiver = await serverCall<Receiver>(`/receivers/${receiverId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load receiver: ${errorMessage}`;
			console.error('Error loading receiver:', err);
		} finally {
			loading = false;
		}
	}

	async function loadFixes() {
		loadingFixes = true;
		fixesError = '';

		try {
			const response = await serverCall<FixesResponse>(
				`/receivers/${receiverId}/fixes?page=${fixesPage}&per_page=100`
			);
			fixes = response.fixes || [];
			fixesTotalPages = response.total_pages || 1;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			fixesError = `Failed to load fixes: ${errorMessage}`;
			console.error('Error loading fixes:', err);
			fixes = []; // Set to empty array on error to prevent retry loop
		} finally {
			loadingFixes = false;
		}
	}

	async function loadStatuses() {
		loadingStatuses = true;
		statusesError = '';

		try {
			const response = await serverCall<StatusesResponse>(
				`/receivers/${receiverId}/statuses?page=${statusesPage}&per_page=100`
			);
			statuses = response.statuses || [];
			statusesTotalPages = response.total_pages || 1;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			statusesError = `Failed to load statuses: ${errorMessage}`;
			console.error('Error loading statuses:', err);
		} finally {
			loadingStatuses = false;
		}
	}

	async function loadStatistics() {
		loadingStatistics = true;
		statisticsError = '';

		try {
			const response = await serverCall<ReceiverStatistics>(`/receivers/${receiverId}/statistics`);
			statistics = response;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			statisticsError = `Failed to load statistics: ${errorMessage}`;
			console.error('Error loading statistics:', err);
		} finally {
			loadingStatistics = false;
		}
	}

	async function loadAggregateStats() {
		loadingAggregateStats = true;
		aggregateStatsError = '';

		try {
			const response = await serverCall<AggregateStatsResponse>(
				`/receivers/${receiverId}/aggregate-stats`
			);
			aggregateStats = response;

			// Fetch device details for each device
			if (aggregateStats && aggregateStats.fix_counts_by_device.length > 0) {
				await Promise.all(
					aggregateStats.fix_counts_by_device.map(async (deviceCount) => {
						try {
							const device = await serverCall<Device>(`/devices/${deviceCount.device_id}`);
							deviceCount.device = device;
						} catch (err) {
							console.warn(`Failed to load device details for ${deviceCount.device_id}:`, err);
							deviceCount.device = null;
						}
					})
				);
			}
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			aggregateStatsError = `Failed to load aggregate statistics: ${errorMessage}`;
			console.error('Error loading aggregate statistics:', err);
			aggregateStats = {
				fix_counts_by_aprs_type: [],
				fix_counts_by_device: []
			}; // Set to default on error to prevent retry loop
		} finally {
			loadingAggregateStats = false;
		}
	}

	async function loadRawMessages() {
		loadingRawMessages = true;
		rawMessagesError = '';

		try {
			const response = await serverCall<RawMessagesResponse>(
				`/receivers/${receiverId}/raw-messages?page=${rawMessagesPage}&per_page=100`
			);
			rawMessages = response.messages || [];
			rawMessagesTotalPages = response.total_pages || 1;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			rawMessagesError = `Failed to load raw messages: ${errorMessage}`;
			console.error('Error loading raw messages:', err);
			rawMessages = []; // Set to empty array on error to prevent retry loop
		} finally {
			loadingRawMessages = false;
		}
	}

	function formatCoordinates(lat: number | null, lng: number | null): string {
		if (lat === null || lng === null || typeof lat !== 'number' || typeof lng !== 'number') {
			return 'Not available';
		}
		return `${lat.toFixed(6)}, ${lng.toFixed(6)}`;
	}

	function generateGoogleMapsUrl(receiver: Receiver): string {
		if (receiver.latitude !== null && receiver.longitude !== null) {
			return `https://www.google.com/maps/search/?api=1&query=${receiver.latitude},${receiver.longitude}`;
		}
		return '';
	}

	function goBack() {
		goto(resolve('/receivers'));
	}

	function formatDateTime(dateStr: string): string {
		return dayjs(dateStr).format('YYYY-MM-DD HH:mm:ss UTC');
	}

	function formatRelativeTime(dateStr: string): string {
		return dayjs(dateStr).fromNow();
	}

	async function nextFixesPage() {
		if (fixesPage < fixesTotalPages) {
			fixesPage++;
			await loadFixes();
		}
	}

	async function prevFixesPage() {
		if (fixesPage > 1) {
			fixesPage--;
			await loadFixes();
		}
	}

	async function nextStatusesPage() {
		if (statusesPage < statusesTotalPages) {
			statusesPage++;
			await loadStatuses();
		}
	}

	async function prevStatusesPage() {
		if (statusesPage > 1) {
			statusesPage--;
			await loadStatuses();
		}
	}

	async function nextRawMessagesPage() {
		if (rawMessagesPage < rawMessagesTotalPages) {
			rawMessagesPage++;
			await loadRawMessages();
		}
	}

	async function prevRawMessagesPage() {
		if (rawMessagesPage > 1) {
			rawMessagesPage--;
			await loadRawMessages();
		}
	}

	function formatRamUsage(
		ramFree: number | string | null,
		ramTotal: number | string | null
	): string {
		if (ramFree === null || ramTotal === null) return '—';

		// Parse BigDecimal values that come as strings from the API
		const freeNum = typeof ramFree === 'string' ? parseFloat(ramFree) : ramFree;
		const totalNum = typeof ramTotal === 'string' ? parseFloat(ramTotal) : ramTotal;

		if (isNaN(freeNum) || isNaN(totalNum)) return '—';

		const usedMb = totalNum - freeNum;
		const percentUsed = ((usedMb / totalNum) * 100).toFixed(1);
		return `${usedMb.toFixed(0)} / ${totalNum.toFixed(0)} MB (${percentUsed}%)`;
	}

	function formatDuration(seconds: number | null): string {
		if (seconds === null || seconds === undefined) return '—';

		const hours = Math.floor(seconds / 3600);
		const minutes = Math.floor((seconds % 3600) / 60);
		const secs = Math.floor(seconds % 60);

		if (hours > 0) {
			return `${hours}h ${minutes}m`;
		} else if (minutes > 0) {
			return `${minutes}m ${secs}s`;
		} else {
			return `${secs}s`;
		}
	}

	/**
	 * Format device label for display according to the pattern:
	 * 1. Registration + Model (e.g., "N12345 - Cessna 182") if both available
	 * 2. Registration only if just that is available
	 * 3. Model only if just that is available
	 * 4. Fallback: Address type + 24-bit code (e.g., "ICAO-A59CDC")
	 */
	function formatDeviceLabel(device: Device | null | undefined, deviceId: string): string {
		if (!device) {
			// Fallback to device ID if no device details
			return deviceId;
		}

		const hasRegistration = device.registration && device.registration.trim() !== '';
		const hasModel = device.aircraft_model && device.aircraft_model.trim() !== '';

		if (hasRegistration && hasModel) {
			return `${device.registration} - ${device.aircraft_model}`;
		} else if (hasRegistration) {
			return device.registration;
		} else if (hasModel) {
			return device.aircraft_model;
		} else {
			// Fallback to address type + 24-bit code
			return formatDeviceAddress(device.address_type, device.address);
		}
	}
</script>

<svelte:head>
	<title>{receiver?.callsign || 'Receiver Details'} - Receivers</title>
</svelte:head>

<div class="max-w-8xl container mx-auto space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn preset-tonal btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Search
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<Progress class="h-8 w-8" />
				<span class="text-lg">Loading receiver details...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert preset-filled-error-500">
			<div class="alert-message">
				<h3 class="h3">Error Loading Receiver</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn preset-filled" onclick={loadReceiver}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Receiver Details -->
	{#if !loading && !error && receiver}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Radio class="h-8 w-10 text-primary-500" />
							<h1 class="h1">{receiver.callsign}</h1>
							{#if receiver.from_ogn_db}
								<span class="chip preset-filled-secondary-500 text-sm">OGN DB</span>
							{/if}
						</div>
						{#if receiver.description}
							<p class="text-surface-600-300-token text-lg">{receiver.description}</p>
						{/if}
					</div>
					<div class="text-surface-500-400-token text-sm">
						<div>Last heard: {formatRelativeTime(receiver.updated_at)}</div>
						<div class="text-xs">Added: {formatDateTime(receiver.created_at)}</div>
					</div>
				</div>
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				<!-- Location Information -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<MapPin class="h-6 w-6" />
						Location
					</h2>

					<div class="space-y-3">
						{#if receiver.country}
							<div class="flex items-start gap-3">
								<Globe class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Country</p>
									<p>{receiver.country}</p>
								</div>
							</div>
						{/if}

						{#if receiver.latitude !== null && receiver.longitude !== null}
							<div class="flex items-start gap-3">
								<Navigation class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Coordinates</p>
									<p class="font-mono text-sm">
										{formatCoordinates(receiver.latitude, receiver.longitude)}
									</p>
								</div>
							</div>

							<!-- External Links -->
							<div class="border-surface-200-700-token border-t pt-3">
								<div class="flex flex-wrap gap-2">
									<a
										href={generateGoogleMapsUrl(receiver)}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-primary-500 btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Open in Google Maps
									</a>
									<a
										href={`https://www.google.com/maps/dir/?api=1&destination=${receiver.latitude},${receiver.longitude}`}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-secondary-500 btn btn-sm"
									>
										<Navigation class="mr-2 h-4 w-4" />
										Get Directions
									</a>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Contact Information -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Info class="h-6 w-6" />
						Contact Information
					</h2>

					<div class="space-y-3">
						{#if receiver.contact}
							<div class="flex items-start gap-3">
								<User class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Contact Name</p>
									<p>{receiver.contact}</p>
								</div>
							</div>
						{/if}

						{#if receiver.email}
							<div class="flex items-start gap-3">
								<Mail class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Email</p>
									<a
										href={`mailto:${receiver.email}`}
										class="text-primary-500 hover:text-primary-600"
									>
										{receiver.email}
									</a>
								</div>
							</div>
						{/if}

						{#if !receiver.contact && !receiver.email}
							<p class="text-surface-500-400-token text-sm">No contact information available</p>
						{/if}

						<div class="border-surface-200-700-token border-t pt-3">
							<div class="flex items-start gap-3">
								<Calendar class="mt-1 h-4 w-4 text-surface-500" />
								<div class="flex-1">
									<p class="text-surface-600-300-token mb-1 text-sm">Updated</p>
									<p class="text-sm">{formatDateTime(receiver.updated_at)}</p>
									<p class="text-surface-500-400-token text-xs">
										{formatRelativeTime(receiver.updated_at)}
									</p>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>

			<!-- Statistics Section -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Signal class="h-6 w-6" />
					Statistics
				</h2>

				{#if loadingStatistics}
					<div class="flex items-center justify-center space-x-4 p-8">
						<Progress class="h-6 w-6" />
						<span>Loading statistics...</span>
					</div>
				{:else if statisticsError}
					<div class="alert preset-filled-error-500">
						<p>{statisticsError}</p>
					</div>
				{:else if statistics}
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
						<!-- Average Update Interval -->
						<div class="space-y-2 card p-4">
							<p class="text-surface-600-300-token text-sm">Average Time Between Updates</p>
							<p class="text-2xl font-semibold">
								{formatDuration(statistics.average_update_interval_seconds)}
							</p>
						</div>

						<!-- Total Status Count -->
						<div class="space-y-2 card p-4">
							<p class="text-surface-600-300-token text-sm">Total Status Updates</p>
							<p class="text-2xl font-semibold">
								{statistics.total_status_count.toLocaleString()}
							</p>
						</div>

						<!-- Time Period -->
						<div class="space-y-2 card p-4">
							<p class="text-surface-600-300-token text-sm">Time Period</p>
							<p class="text-2xl font-semibold">
								{#if statistics.days_included}
									Last {statistics.days_included} days
								{:else}
									All time
								{/if}
							</p>
						</div>
					</div>
				{/if}
			</div>

			<!-- Status Reports, Raw Messages, and Received Fixes Section with Tabs -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Info class="h-6 w-6" />
					Receiver Data (Last 24 Hours)
				</h2>

				<Tabs value={activeTab} onValueChange={(details) => (activeTab = details.value)}>
					<Tabs.List class="border-b border-surface-300 dark:border-surface-600">
						<Tabs.Trigger value="status-reports">
							<Signal class="mr-2 h-4 w-4" />
							Status Reports
						</Tabs.Trigger>
						<Tabs.Trigger value="raw-messages">
							<FileText class="mr-2 h-4 w-4" />
							Raw Messages
						</Tabs.Trigger>
						<Tabs.Trigger value="received-fixes">
							<Signal class="mr-2 h-4 w-4" />
							Received Fixes
						</Tabs.Trigger>
						<Tabs.Trigger value="aggregate-stats">
							<Signal class="mr-2 h-4 w-4" />
							Aggregate Statistics
						</Tabs.Trigger>
						<Tabs.Indicator />
					</Tabs.List>

					<!-- Status Reports Tab Content -->
					<Tabs.Content value="status-reports">
						<div class="mt-4">
							<div class="mb-4 flex items-center justify-end">
								<label class="flex cursor-pointer items-center gap-2">
									<input type="checkbox" class="checkbox" bind:checked={showRawData} />
									<span class="text-sm">Show raw</span>
								</label>
							</div>

							{#if loadingStatuses}
								<div class="flex items-center justify-center space-x-4 p-8">
									<Progress class="h-6 w-6" />
									<span>Loading statuses...</span>
								</div>
							{:else if statusesError}
								<div class="alert preset-filled-error-500">
									<p>{statusesError}</p>
								</div>
							{:else if statuses.length === 0}
								<p class="text-surface-500-400-token p-4 text-center">
									No status reports in the last 24 hours
								</p>
							{:else}
								<div class="table-container">
									<table class="table-hover table">
										<thead>
											<tr>
												<th>Timestamp</th>
												<th>Version</th>
												<th>Platform</th>
												<th>CPU</th>
												<th>RAM</th>
												<th>Senders</th>
												<th>Voltage</th>
												<th>Lag</th>
											</tr>
										</thead>
										<tbody>
											{#each statuses as status, index (status.id)}
												<tr
													class="border-b border-gray-200 hover:bg-gray-100 dark:border-gray-700 dark:hover:bg-gray-800 {index %
														2 ===
													0
														? 'bg-gray-50 dark:bg-gray-900'
														: ''}"
												>
													<td class="text-xs">
														<div>{formatDateTime(status.received_at)}</div>
														<div class="text-surface-500-400-token">
															{formatRelativeTime(status.received_at)}
														</div>
													</td>
													<td class="font-mono text-xs">{status.version || '—'}</td>
													<td class="text-xs">{status.platform || '—'}</td>
													<td class="text-sm">
														{#if status.cpu_load !== null}
															{(Number(status.cpu_load) * 100).toFixed(0)}%
															{#if status.cpu_temperature !== null}
																<span class="text-surface-500-400-token text-xs">
																	({Number(status.cpu_temperature).toFixed(1)}°C)
																</span>
															{/if}
														{:else}
															—
														{/if}
													</td>
													<td class="text-xs">
														{formatRamUsage(status.ram_free, status.ram_total)}
													</td>
													<td>
														{#if status.visible_senders !== null}
															{status.visible_senders}
															{#if status.senders !== null}
																<span class="text-surface-500-400-token">/ {status.senders}</span>
															{/if}
														{:else}
															—
														{/if}
													</td>
													<td>
														{#if status.voltage !== null}
															{Number(status.voltage).toFixed(2)}V
															{#if status.amperage !== null}
																<span class="text-surface-500-400-token text-xs">
																	({Number(status.amperage).toFixed(2)}A)
																</span>
															{/if}
														{:else}
															—
														{/if}
													</td>
													<td>{status.lag !== null ? `${status.lag}ms` : '—'}</td>
												</tr>
												{#if showRawData}
													<tr
														class="border-b border-gray-200 dark:border-gray-700 {index % 2 === 0
															? 'bg-gray-100 dark:bg-gray-800'
															: ''}"
													>
														<td colspan="8" class="px-3 py-2 font-mono text-sm">
															{status.raw_data}
														</td>
													</tr>
												{/if}
											{/each}
										</tbody>
									</table>
								</div>

								<!-- Pagination Controls -->
								{#if statusesTotalPages > 1}
									<div class="mt-4 flex items-center justify-between">
										<button
											class="btn preset-tonal btn-sm"
											disabled={statusesPage === 1}
											onclick={prevStatusesPage}
										>
											<ChevronLeft class="h-4 w-4" />
											Previous
										</button>
										<span class="text-sm">
											Page {statusesPage} of {statusesTotalPages}
										</span>
										<button
											class="btn preset-tonal btn-sm"
											disabled={statusesPage === statusesTotalPages}
											onclick={nextStatusesPage}
										>
											Next
											<ChevronRight class="h-4 w-4" />
										</button>
									</div>
								{/if}

								<!-- Fixes by Device -->
								{#if aggregateStats && aggregateStats.fix_counts_by_device.length > 0}
									<div class="mt-6 space-y-4">
										<h3 class="h3">Fixes Received by Device</h3>
										<div class="table-container">
											<table class="table-hover table">
												<thead>
													<tr>
														<th>Device</th>
														<th class="text-right">Count</th>
													</tr>
												</thead>
												<tbody>
													{#each aggregateStats.fix_counts_by_device as deviceCount (deviceCount.device_id)}
														<tr>
															<td>
																<a
																	href={resolve(`/devices/${deviceCount.device_id}`)}
																	class="text-primary-500 hover:text-primary-600"
																>
																	<div class="flex items-center gap-2">
																		<span class="font-semibold">
																			{formatDeviceLabel(deviceCount.device, deviceCount.device_id)}
																		</span>
																		{#if deviceCount.device?.aircraft_type_ogn}
																			<span
																				class="badge {getAircraftTypeColor(
																					deviceCount.device.aircraft_type_ogn
																				)} text-xs"
																			>
																				{getAircraftTypeOgnDescription(
																					deviceCount.device.aircraft_type_ogn
																				)}
																			</span>
																		{/if}
																	</div>
																</a>
															</td>
															<td class="text-right font-semibold">
																{deviceCount.count.toLocaleString()}
															</td>
														</tr>
													{/each}
												</tbody>
											</table>
										</div>
									</div>
								{/if}
							{/if}
						</div>
					</Tabs.Content>

					<!-- Raw Messages Tab Content -->
					<Tabs.Content value="raw-messages">
						<div class="mt-4">
							{#if loadingRawMessages}
								<div class="flex items-center justify-center space-x-4 p-8">
									<Progress class="h-6 w-6" />
									<span>Loading raw messages...</span>
								</div>
							{:else if rawMessagesError}
								<div class="alert preset-filled-error-500">
									<p>{rawMessagesError}</p>
								</div>
							{:else if rawMessages !== null && rawMessages.length === 0}
								<p class="text-surface-500-400-token p-4 text-center">
									No raw messages received in the last 24 hours
								</p>
							{:else if rawMessages !== null}
								<div class="table-container">
									<table class="table-hover table">
										<thead>
											<tr>
												<th>Timestamp</th>
												<th>Raw Message</th>
												<th>Unparsed Data</th>
											</tr>
										</thead>
										<tbody>
											{#each rawMessages as message (message.id)}
												<tr>
													<td class="text-xs" style="min-width: 150px;">
														<div>{formatDateTime(message.received_at)}</div>
														<div class="text-surface-500-400-token">
															{formatRelativeTime(message.received_at)}
														</div>
													</td>
													<td
														class="font-mono text-xs"
														style="max-width: 600px; word-break: break-all;"
													>
														{message.raw_message}
													</td>
													<td class="font-mono text-xs">
														{message.unparsed || '—'}
													</td>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>

								<!-- Pagination Controls -->
								{#if rawMessagesTotalPages > 1}
									<div class="mt-4 flex items-center justify-between">
										<button
											class="btn preset-tonal btn-sm"
											disabled={rawMessagesPage === 1}
											onclick={prevRawMessagesPage}
										>
											<ChevronLeft class="h-4 w-4" />
											Previous
										</button>
										<span class="text-sm">
											Page {rawMessagesPage} of {rawMessagesTotalPages}
										</span>
										<button
											class="btn preset-tonal btn-sm"
											disabled={rawMessagesPage === rawMessagesTotalPages}
											onclick={nextRawMessagesPage}
										>
											Next
											<ChevronRight class="h-4 w-4" />
										</button>
									</div>
								{/if}
							{/if}
						</div>
					</Tabs.Content>

					<!-- Received Fixes Tab Content -->
					<Tabs.Content value="received-fixes">
						<div class="mt-4">
							{#if loadingFixes}
								<div class="flex items-center justify-center space-x-4 p-8">
									<Progress class="h-6 w-6" />
									<span>Loading fixes...</span>
								</div>
							{:else if fixesError}
								<div class="alert preset-filled-error-500">
									<p>{fixesError}</p>
								</div>
							{:else if fixes !== null && fixes.length === 0}
								<p class="text-surface-500-400-token p-4 text-center">
									No fixes received in the last 24 hours
								</p>
							{:else if fixes !== null}
								<div class="table-container">
									<table class="table-hover table">
										<thead>
											<tr>
												<th>Timestamp</th>
												<th>Device</th>
												<th>Registration</th>
												<th>Position</th>
												<th>Altitude</th>
												<th>Speed</th>
												<th>SNR</th>
											</tr>
										</thead>
										<tbody>
											{#each fixes as fix (fix.id)}
												<tr>
													<td class="text-xs">
														<div>{formatDateTime(fix.timestamp)}</div>
														<div class="text-surface-500-400-token">
															{formatRelativeTime(fix.timestamp)}
														</div>
													</td>
													<td class="font-mono text-xs">
														{fix.device_address.toString(16).toUpperCase().padStart(6, '0')}
													</td>
													<td class="font-mono text-sm">{fix.registration || '—'}</td>
													<td class="font-mono text-xs">
														{fix.latitude?.toFixed(4) ?? '—'}, {fix.longitude?.toFixed(4) ?? '—'}
													</td>
													<td
														>{fix.altitude_msl_feet !== null && fix.altitude_msl_feet !== undefined
															? `${fix.altitude_msl_feet} ft`
															: '—'}</td
													>
													<td
														>{fix.ground_speed_knots !== null &&
														fix.ground_speed_knots !== undefined
															? `${fix.ground_speed_knots.toFixed(0)} kt`
															: '—'}</td
													>
													<td
														>{fix.snr_db !== null && fix.snr_db !== undefined
															? `${fix.snr_db.toFixed(1)} dB`
															: '—'}</td
													>
												</tr>
											{/each}
										</tbody>
									</table>
								</div>

								<!-- Pagination Controls -->
								{#if fixesTotalPages > 1}
									<div class="mt-4 flex items-center justify-between">
										<button
											class="btn preset-tonal btn-sm"
											disabled={fixesPage === 1}
											onclick={prevFixesPage}
										>
											<ChevronLeft class="h-4 w-4" />
											Previous
										</button>
										<span class="text-sm">
											Page {fixesPage} of {fixesTotalPages}
										</span>
										<button
											class="btn preset-tonal btn-sm"
											disabled={fixesPage === fixesTotalPages}
											onclick={nextFixesPage}
										>
											Next
											<ChevronRight class="h-4 w-4" />
										</button>
									</div>
								{/if}
							{/if}
						</div>
					</Tabs.Content>

					<!-- Aggregate Statistics Tab Content -->
					<Tabs.Content value="aggregate-stats">
						<div class="mt-4">
							{#if loadingAggregateStats}
								<div class="flex items-center justify-center space-x-4 p-8">
									<Progress class="h-6 w-6" />
									<span>Loading aggregate statistics...</span>
								</div>
							{:else if aggregateStatsError}
								<div class="alert preset-filled-error-500">
									<p>{aggregateStatsError}</p>
								</div>
							{:else if aggregateStats !== null}
								<div class="space-y-6">
									<!-- Fixes by APRS Type -->
									{#if aggregateStats.fix_counts_by_aprs_type.length === 0 && aggregateStats.fix_counts_by_device.length === 0}
										<p class="text-surface-500-400-token p-4 text-center">
											No fix data available for this receiver
										</p>
									{:else}
										<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
											<!-- Fixes by APRS Type -->
											{#if aggregateStats.fix_counts_by_aprs_type.length > 0}
												<div class="space-y-4">
													<h3 class="h3">Fixes Received by APRS Type</h3>
													<div class="table-container">
														<table class="table-hover table">
															<thead>
																<tr>
																	<th>APRS Type</th>
																	<th class="text-right">Count</th>
																</tr>
															</thead>
															<tbody>
																{#each aggregateStats.fix_counts_by_aprs_type as typeCount (typeCount.aprs_type)}
																	<tr>
																		<td class="font-mono">{typeCount.aprs_type}</td>
																		<td class="text-right font-semibold">
																			{typeCount.count.toLocaleString()}
																		</td>
																	</tr>
																{/each}
															</tbody>
														</table>
													</div>
												</div>
											{/if}

											<!-- Fixes by Device -->
											{#if aggregateStats && aggregateStats.fix_counts_by_device.length > 0}
												<div class="space-y-4">
													<h3 class="h3">Fixes Received by Device</h3>
													<div class="table-container">
														<table class="table-hover table">
															<thead>
																<tr>
																	<th>Device</th>
																	<th class="text-right">Count</th>
																</tr>
															</thead>
															<tbody>
																{#each aggregateStats.fix_counts_by_device as deviceCount (deviceCount.device_id)}
																	<tr>
																		<td>
																			<a
																				href={resolve(`/devices/${deviceCount.device_id}`)}
																				class="font-mono text-primary-500 hover:text-primary-600"
																			>
																				{deviceCount.device_id}
																			</a>
																		</td>
																		<td class="text-right font-semibold">
																			{deviceCount.count.toLocaleString()}
																		</td>
																	</tr>
																{/each}
															</tbody>
														</table>
													</div>
												</div>
											{/if}
										</div>
									{/if}
								</div>
							{/if}
						</div>
					</Tabs.Content>
				</Tabs>
			</div>

			<!-- Map Section -->
			{#if receiver.latitude !== null && receiver.longitude !== null}
				<div class="card p-6">
					<h2 class="mb-4 flex items-center gap-2 h2">
						<Navigation class="h-6 w-6" />
						Location Map
					</h2>
					<div class="border-surface-300-600-token overflow-hidden rounded-lg border">
						<!-- Embedded Google Map -->
						<iframe
							src={`https://maps.google.com/maps?q=${receiver.latitude},${receiver.longitude}&output=embed`}
							width="100%"
							height="500"
							style="border:0;"
							allowfullscreen
							loading="lazy"
							referrerpolicy="no-referrer-when-downgrade"
							title="Location map for {receiver.callsign}"
						></iframe>
					</div>
					<div class="mt-3 flex flex-wrap gap-2">
						<a
							href={generateGoogleMapsUrl(receiver)}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-tonal-primary-500 btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						<a
							href={`https://www.google.com/maps/dir/?api=1&destination=${receiver.latitude},${receiver.longitude}`}
							target="_blank"
							rel="noopener noreferrer"
							class="preset-tonal-secondary-500 btn btn-sm"
						>
							<Navigation class="mr-2 h-4 w-4" />
							Get Directions
						</a>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
