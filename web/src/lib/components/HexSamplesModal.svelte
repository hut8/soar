<script lang="ts">
	import { Hexagon, X, Loader, Radio } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import type {
		CoverageHexProperties,
		Fix,
		FixesInHexResponse,
		HexReceiversResponse,
		Receiver
	} from '$lib/types';
	import dayjs from 'dayjs';
	import { resolve } from '$app/paths';

	let {
		showModal = $bindable(),
		hexProperties = $bindable(),
		dateRange,
		receiverId,
		altitudeFilter
	} = $props<{
		showModal: boolean;
		hexProperties: CoverageHexProperties | null;
		dateRange: { start: string; end: string };
		receiverId?: string;
		altitudeFilter?: { min: number; max: number };
	}>();

	let fixes = $state<Fix[]>([]);
	let total = $state(0);
	let currentPage = $state(1);
	let isLoading = $state(false);
	let error = $state('');

	// Receivers state
	let receivers = $state<Receiver[]>([]);
	let isLoadingReceivers = $state(false);
	let receiversError = $state('');

	const fixesPerPage = 100;

	function closeModal() {
		showModal = false;
	}

	function formatTimestamp(timestamp: string): string {
		return dayjs(timestamp).format('MMM D, HH:mm:ss');
	}

	function formatDuration(hours: number): string {
		if (hours < 1) {
			const minutes = Math.round(hours * 60);
			return `${minutes} min`;
		}
		if (hours < 24) {
			return `${hours.toFixed(1)} hours`;
		}
		const days = Math.floor(hours / 24);
		const remainingHours = Math.round(hours % 24);
		return `${days}d ${remainingHours}h`;
	}

	async function loadFixes() {
		if (!hexProperties) return;

		isLoading = true;
		error = '';

		try {
			// eslint-disable-next-line svelte/prefer-svelte-reactivity -- URLSearchParams created fresh on each call, no persistent state
			const params = new URLSearchParams({
				limit: fixesPerPage.toString(),
				offset: ((currentPage - 1) * fixesPerPage).toString()
			});

			if (receiverId) {
				params.append('receiver_id', receiverId);
			}
			if (altitudeFilter) {
				params.append('min_altitude', altitudeFilter.min.toString());
				params.append('max_altitude', altitudeFilter.max.toString());
			}

			// Pass first_seen and last_seen for efficient partition pruning (required)
			if (hexProperties.firstSeenAt) {
				params.append('first_seen', hexProperties.firstSeenAt);
			}
			if (hexProperties.lastSeenAt) {
				params.append('last_seen', hexProperties.lastSeenAt);
			}

			const response = await serverCall<FixesInHexResponse>(
				`/coverage/hexes/${hexProperties.h3Index}/fixes?${params}`
			);

			fixes = response.data;
			total = response.total;
		} catch (err) {
			error = `Failed to load fixes: ${err instanceof Error ? err.message : 'Unknown error'}`;
			fixes = [];
			total = 0;
		} finally {
			isLoading = false;
		}
	}

	async function loadReceivers() {
		if (!hexProperties) return;

		isLoadingReceivers = true;
		receiversError = '';

		try {
			const params = new URLSearchParams({
				start_date: dateRange.start,
				end_date: dateRange.end
			});

			const response = await serverCall<HexReceiversResponse>(
				`/coverage/hexes/${hexProperties.h3Index}/receivers?${params}`
			);

			receivers = response.data;
		} catch (err) {
			receiversError = `Failed to load receivers: ${err instanceof Error ? err.message : 'Unknown error'}`;
			receivers = [];
		} finally {
			isLoadingReceivers = false;
		}
	}

	function changePage(newPage: number) {
		currentPage = newPage;
		loadFixes();
	}

	// Load fixes and receivers when modal opens or hex changes
	$effect(() => {
		if (showModal && hexProperties) {
			currentPage = 1;
			loadFixes();
			loadReceivers();
		}
	});

	const totalPages = $derived(Math.ceil(total / fixesPerPage));

	// Create a map of receiver ID to callsign for efficient lookup
	const receiverMap = $derived(new Map(receivers.map((r) => [r.id, r.callsign])));

	function getReceiverCallsign(receiverId: string | null): string {
		if (!receiverId) return '—';
		return receiverMap.get(receiverId) || receiverId.substring(0, 8);
	}
</script>

{#if showModal && hexProperties}
	<!-- Backdrop overlay -->
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-surface-950-50/50 pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		role="presentation"
		tabindex="-1"
	>
		<!-- Modal card -->
		<div
			class="max-h-[calc(90vh-5rem)] w-full max-w-6xl overflow-y-auto card bg-surface-50 text-surface-900 shadow-xl dark:bg-surface-900 dark:text-surface-50"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="modal-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between border-b border-surface-300 p-6 dark:border-surface-600"
			>
				<div class="flex items-center gap-3">
					<Hexagon size={24} class="text-primary-500" />
					<div>
						<h2 id="modal-title" class="text-xl font-bold">Coverage Hex Details</h2>
						<p class="text-sm text-surface-600 dark:text-surface-400">
							H3 Index: {hexProperties.h3Index} (Resolution {hexProperties.resolution})
						</p>
					</div>
				</div>
				<button
					onclick={closeModal}
					class="preset-tonal-surface-500 btn btn-sm"
					aria-label="Close modal"
				>
					<X size={20} />
				</button>
			</div>

			<!-- Content -->
			<div class="space-y-6 p-6">
				<!-- Section 1: Aggregated Statistics -->
				<div>
					<h3 class="mb-4 text-lg font-semibold">Aggregated Statistics</h3>
					<div class="grid grid-cols-2 gap-4 md:grid-cols-3 lg:grid-cols-4">
						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
								Total Fixes
							</dt>
							<dd class="mt-1 text-2xl font-bold">
								{hexProperties.fixCount.toLocaleString()}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
								Coverage Duration
							</dt>
							<dd class="mt-1 text-lg font-semibold">
								{formatDuration(hexProperties.coverageHours)}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">First Seen</dt>
							<dd class="mt-1 text-sm font-medium">
								{dayjs(hexProperties.firstSeenAt).format('MMM D, HH:mm')}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">Last Seen</dt>
							<dd class="mt-1 text-sm font-medium">
								{dayjs(hexProperties.lastSeenAt).format('MMM D, HH:mm')}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
								Min Altitude (MSL)
							</dt>
							<dd class="mt-1 text-lg font-semibold">
								{hexProperties.minAltitudeMslFeet != null
									? `${hexProperties.minAltitudeMslFeet.toLocaleString()} ft`
									: '—'}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
								Max Altitude (MSL)
							</dt>
							<dd class="mt-1 text-lg font-semibold">
								{hexProperties.maxAltitudeMslFeet != null
									? `${hexProperties.maxAltitudeMslFeet.toLocaleString()} ft`
									: '—'}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
								Avg Altitude (MSL)
							</dt>
							<dd class="mt-1 text-lg font-semibold">
								{hexProperties.avgAltitudeMslFeet != null
									? `${hexProperties.avgAltitudeMslFeet.toLocaleString()} ft`
									: '—'}
							</dd>
						</div>

						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
								Altitude Range
							</dt>
							<dd class="mt-1 text-lg font-semibold">
								{hexProperties.minAltitudeMslFeet != null &&
								hexProperties.maxAltitudeMslFeet != null
									? `${(hexProperties.maxAltitudeMslFeet - hexProperties.minAltitudeMslFeet).toLocaleString()} ft`
									: '—'}
							</dd>
						</div>
					</div>
				</div>

				<!-- Section 2: Contributing Receivers -->
				<div>
					<div class="mb-4 flex items-center gap-2">
						<Radio size={20} class="text-primary-500" />
						<h3 class="text-lg font-semibold">Contributing Receivers</h3>
					</div>

					{#if isLoadingReceivers}
						<div class="flex items-center gap-2 p-4">
							<Loader class="h-5 w-5 animate-spin text-primary-500" />
							<span class="text-sm text-surface-600 dark:text-surface-400"
								>Loading receivers...</span
							>
						</div>
					{:else if receiversError}
						<div
							class="rounded-lg border border-error-500 bg-error-50 p-4 text-error-900 dark:bg-error-900/20 dark:text-error-100"
						>
							<p class="text-sm">{receiversError}</p>
						</div>
					{:else if receivers.length === 0}
						<div class="rounded-lg bg-surface-100 p-4 dark:bg-surface-800">
							<p class="text-sm text-surface-600 dark:text-surface-400">
								No receiver information available
							</p>
						</div>
					{:else}
						<div class="flex flex-wrap gap-3">
							{#each receivers as receiver (receiver.id)}
								<a
									href={resolve(`/receivers/${receiver.id}`)}
									class="flex items-center gap-2 rounded-lg bg-surface-100 px-4 py-3 transition-colors hover:bg-surface-200 dark:bg-surface-800 dark:hover:bg-surface-700"
								>
									<Radio size={16} class="text-primary-500" />
									<div>
										<div class="font-medium">{receiver.callsign}</div>
										{#if receiver.city || receiver.region}
											<div class="text-xs text-surface-600 dark:text-surface-400">
												{[receiver.city, receiver.region].filter(Boolean).join(', ')}
											</div>
										{/if}
									</div>
								</a>
							{/each}
						</div>
					{/if}
				</div>

				<!-- Section 3: Individual Position Fixes -->
				<div>
					<div class="mb-4 flex items-center justify-between">
						<h3 class="text-lg font-semibold">Individual Position Fixes</h3>
						{#if total > 0}
							<p class="text-sm text-surface-600 dark:text-surface-400">
								Showing {(currentPage - 1) * fixesPerPage + 1}–{Math.min(
									currentPage * fixesPerPage,
									total
								)} of {total.toLocaleString()}
							</p>
						{/if}
					</div>

					{#if isLoading}
						<div class="flex flex-col items-center justify-center p-12">
							<Loader class="mb-4 h-8 w-8 animate-spin text-primary-500" />
							<p class="text-sm text-surface-600 dark:text-surface-400">Loading fixes...</p>
						</div>
					{:else if error}
						<div
							class="rounded-lg border border-error-500 bg-error-50 p-4 text-error-900 dark:bg-error-900/20 dark:text-error-100"
						>
							<p class="font-medium">Error loading fixes</p>
							<p class="mt-1 text-sm">{error}</p>
						</div>
					{:else if fixes.length === 0}
						<div class="rounded-lg bg-surface-100 p-8 text-center dark:bg-surface-800">
							<p class="text-surface-600 dark:text-surface-400">
								No position fixes found for this hexagon
							</p>
						</div>
					{:else}
						<div class="table-container">
							<table class="table-compact table-hover table">
								<thead>
									<tr>
										<th>Time</th>
										<th>Aircraft</th>
										<th>Receiver</th>
										<th>Altitude MSL</th>
										<th>Altitude AGL</th>
										<th>Speed</th>
										<th>Track</th>
									</tr>
								</thead>
								<tbody>
									{#each fixes as fix (fix.id)}
										<tr>
											<td class="whitespace-nowrap">{formatTimestamp(fix.timestamp)}</td>
											<td>
												{#if fix.aircraftId}
													<a
														href={resolve(`/aircraft/${fix.aircraftId}`)}
														class="anchor font-mono text-sm"
													>
														{fix.source || 'Unknown'}
													</a>
												{:else}
													<span class="font-mono text-sm">{fix.source || 'Unknown'}</span>
												{/if}
											</td>
											<td>
												{#if fix.receiverId}
													<a href={resolve(`/receivers/${fix.receiverId}`)} class="anchor text-sm">
														{getReceiverCallsign(fix.receiverId)}
													</a>
												{:else}
													<span class="text-surface-500">—</span>
												{/if}
											</td>
											<td class="text-right">
												{fix.altitudeMslFeet != null
													? `${fix.altitudeMslFeet.toLocaleString()} ft`
													: '—'}
											</td>
											<td class="text-right">
												{fix.altitudeAglFeet != null
													? `${fix.altitudeAglFeet.toLocaleString()} ft`
													: '—'}
											</td>
											<td class="text-right">
												{fix.groundSpeedKnots != null ? `${fix.groundSpeedKnots} kt` : '—'}
											</td>
											<td class="text-right">
												{fix.trackDegrees != null ? `${fix.trackDegrees}°` : '—'}
											</td>
										</tr>
									{/each}
								</tbody>
							</table>
						</div>

						<!-- Pagination Controls -->
						{#if totalPages > 1}
							<div class="mt-4 flex items-center justify-between">
								<div class="text-sm text-surface-600 dark:text-surface-400">
									Page {currentPage} of {totalPages}
								</div>
								<div class="flex gap-2">
									<button
										onclick={() => changePage(currentPage - 1)}
										disabled={currentPage === 1 || isLoading}
										class="preset-tonal-surface-500 btn btn-sm"
									>
										Previous
									</button>
									<button
										onclick={() => changePage(currentPage + 1)}
										disabled={currentPage >= totalPages || isLoading}
										class="preset-tonal-surface-500 btn btn-sm"
									>
										Next
									</button>
								</div>
							</div>
						{/if}
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	/* Additional styles if needed */
</style>
