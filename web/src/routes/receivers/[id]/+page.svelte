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
		ChevronRight
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';

	dayjs.extend(relativeTime);

	interface Receiver {
		id: number;
		callsign: string;
		description: string | null;
		contact: string | null;
		email: string | null;
		country: string | null;
		latitude: number | null;
		longitude: number | null;
		created_at: string;
		updated_at: string;
	}

	interface Fix {
		id: string;
		timestamp: string;
		latitude: number;
		longitude: number;
		altitude_feet: number | null;
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
		cpu_load: number | null;
		cpu_temperature: number | null;
		ram_free: number | null;
		ram_total: number | null;
		visible_senders: number | null;
		senders: number | null;
		voltage: number | null;
		amperage: number | null;
		lag: number | null;
	}

	interface StatusesResponse {
		statuses: ReceiverStatus[];
		page: number;
		total_pages: number;
	}

	let receiver: Receiver | null = null;
	let fixes: Fix[] = [];
	let statuses: ReceiverStatus[] = [];
	let loading = true;
	let loadingFixes = false;
	let loadingStatuses = false;
	let error = '';
	let fixesError = '';
	let statusesError = '';
	let receiverId = '';

	let fixesPage = 1;
	let fixesTotalPages = 1;
	let statusesPage = 1;
	let statusesTotalPages = 1;

	$: receiverId = $page.params.id || '';

	onMount(async () => {
		if (receiverId) {
			await loadReceiver();
			await loadFixes();
			await loadStatuses();
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

	function formatCoordinates(lat: number | null, lng: number | null): string {
		if (lat === null || lng === null) return 'Not available';
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

	function formatRamUsage(ramFree: number | null, ramTotal: number | null): string {
		if (ramFree === null || ramTotal === null) return '—';
		const usedMb = ramTotal - ramFree;
		const percentUsed = ((usedMb / ramTotal) * 100).toFixed(1);
		return `${usedMb.toFixed(0)} / ${ramTotal.toFixed(0)} MB (${percentUsed}%)`;
	}
</script>

<svelte:head>
	<title>{receiver?.callsign || 'Receiver Details'} - Receivers</title>
</svelte:head>

<div class="max-w-8xl container mx-auto space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="variant-soft btn btn-sm" onclick={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Search
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Loading receiver details...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert variant-filled-error">
			<div class="alert-message">
				<h3 class="h3">Error Loading Receiver</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="variant-filled btn" onclick={loadReceiver}> Try Again </button>
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
										class="variant-soft-primary btn btn-sm"
									>
										<ExternalLink class="mr-2 h-4 w-4" />
										Open in Google Maps
									</a>
									<a
										href={`https://www.google.com/maps/dir/?api=1&destination=${receiver.latitude},${receiver.longitude}`}
										target="_blank"
										rel="noopener noreferrer"
										class="variant-soft-secondary btn btn-sm"
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

			<!-- Fixes Section -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Signal class="h-6 w-6" />
					Received Fixes (Last 24 Hours)
				</h2>

				{#if loadingFixes}
					<div class="flex items-center justify-center space-x-4 p-8">
						<ProgressRing size="w-6 h-6" />
						<span>Loading fixes...</span>
					</div>
				{:else if fixesError}
					<div class="alert variant-filled-error">
						<p>{fixesError}</p>
					</div>
				{:else if fixes.length === 0}
					<p class="text-surface-500-400-token p-4 text-center">
						No fixes received in the last 24 hours
					</p>
				{:else}
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
											{fix.latitude.toFixed(4)}, {fix.longitude.toFixed(4)}
										</td>
										<td>{fix.altitude_feet !== null ? `${fix.altitude_feet} ft` : '—'}</td>
										<td
											>{fix.ground_speed_knots !== null
												? `${fix.ground_speed_knots.toFixed(0)} kt`
												: '—'}</td
										>
										<td>{fix.snr_db !== null ? `${fix.snr_db.toFixed(1)} dB` : '—'}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>

					<!-- Pagination Controls -->
					{#if fixesTotalPages > 1}
						<div class="mt-4 flex items-center justify-between">
							<button
								class="variant-soft btn btn-sm"
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
								class="variant-soft btn btn-sm"
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

			<!-- Statuses Section -->
			<div class="card p-6">
				<h2 class="mb-4 flex items-center gap-2 h2">
					<Info class="h-6 w-6" />
					Status Reports (Last 24 Hours)
				</h2>

				{#if loadingStatuses}
					<div class="flex items-center justify-center space-x-4 p-8">
						<ProgressRing size="w-6 h-6" />
						<span>Loading statuses...</span>
					</div>
				{:else if statusesError}
					<div class="alert variant-filled-error">
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
								{#each statuses as status (status.id)}
									<tr>
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
												{(status.cpu_load * 100).toFixed(0)}%
												{#if status.cpu_temperature !== null}
													<span class="text-surface-500-400-token text-xs">
														({status.cpu_temperature.toFixed(1)}°C)
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
												{status.voltage.toFixed(2)}V
												{#if status.amperage !== null}
													<span class="text-surface-500-400-token text-xs">
														({status.amperage.toFixed(2)}A)
													</span>
												{/if}
											{:else}
												—
											{/if}
										</td>
										<td>{status.lag !== null ? `${status.lag}ms` : '—'}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>

					<!-- Pagination Controls -->
					{#if statusesTotalPages > 1}
						<div class="mt-4 flex items-center justify-between">
							<button
								class="variant-soft btn btn-sm"
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
								class="variant-soft btn btn-sm"
								disabled={statusesPage === statusesTotalPages}
								onclick={nextStatusesPage}
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						</div>
					{/if}
				{/if}
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
							class="variant-ghost-primary btn btn-sm"
						>
							<ExternalLink class="mr-2 h-4 w-4" />
							View Larger Map
						</a>
						<a
							href={`https://www.google.com/maps/dir/?api=1&destination=${receiver.latitude},${receiver.longitude}`}
							target="_blank"
							rel="noopener noreferrer"
							class="variant-ghost-secondary btn btn-sm"
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
