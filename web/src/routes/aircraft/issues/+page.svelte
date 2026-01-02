<script lang="ts">
	import { AlertTriangle, Radio, ChevronLeft, ChevronRight, Search } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';
	import type { Aircraft, PaginatedDataResponse } from '$lib/types';
	import AircraftLink from '$lib/components/AircraftLink.svelte';
	import { getFlagPath } from '$lib/formatters';

	let duplicateDevices = $state<Aircraft[]>([]);
	let loading = $state(false);
	let error = $state('');
	let currentPage = $state(1);
	let totalPages = $state(0);
	let totalCount = $state(0);
	let perPage = $state(50);
	let searchQuery = $state('');

	async function loadIssues(page: number = 1, search: string = '') {
		loading = true;
		error = '';

		try {
			const params: { page: number; perPage: number; hexSearch?: string } = {
				page,
				perPage
			};

			// Add search parameter if provided
			if (search.trim()) {
				params.hexSearch = search.trim();
			}

			const response = await serverCall<PaginatedDataResponse<Aircraft>>('/aircraft/issues', {
				method: 'GET',
				params
			});
			duplicateDevices = response.data || [];
			currentPage = response.metadata.page;
			totalPages = response.metadata.totalPages;
			totalCount = response.metadata.totalCount;
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load aircraft issues: ${errorMessage}`;
			console.error('Error loading aircraft issues:', err);
			duplicateDevices = [];
		} finally {
			loading = false;
		}
	}

	// Debounce timer for search
	let searchTimer: ReturnType<typeof setTimeout> | null = null;

	// Handle search input changes
	function handleSearchInput() {
		// Clear existing timer
		if (searchTimer !== null) {
			clearTimeout(searchTimer);
		}

		// Set new timer to trigger search after 300ms of inactivity
		searchTimer = setTimeout(() => {
			// Reset to page 1 when search changes
			loadIssues(1, searchQuery);
		}, 300);
	}

	function goToPage(page: number) {
		if (page >= 1 && page <= totalPages) {
			loadIssues(page, searchQuery);
		}
	}

	onMount(() => {
		loadIssues();
	});

	function formatDate(dateStr: string | null | undefined): string {
		if (!dateStr) return '-';
		const date = new Date(dateStr);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: '2-digit',
			day: '2-digit',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head>
	<title>Aircraft Issues - SOAR Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<AlertTriangle class="h-8 w-8" />
			Aircraft Issues
		</h1>
		<p class="text-surface-500-400-token">
			Review aircraft data quality issues and duplicate addresses
		</p>
	</header>

	<!-- Duplicate Addresses Section -->
	<section class="space-y-4 card p-6">
		<h2 class="flex items-center gap-2 text-xl font-semibold text-error-500">
			<AlertTriangle class="h-6 w-6" />
			Duplicate Aircraft Addresses
		</h2>

		<!-- Search Input -->
		<div class="flex gap-2">
			<div class="input-group flex-1 grid-cols-[auto_1fr_auto]">
				<div class="input-group-shim"><Search class="h-4 w-4" /></div>
				<input
					type="text"
					placeholder="Search by hex ID (e.g., f00)"
					bind:value={searchQuery}
					oninput={handleSearchInput}
					class="input"
				/>
			</div>
		</div>

		{#if loading}
			<div class="flex items-center justify-center p-12">
				<div
					class="h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
				></div>
			</div>
		{:else if error}
			<div class="rounded border border-error-200 bg-error-50 p-4 text-error-700">
				{error}
			</div>
		{:else if duplicateDevices.length === 0}
			<div class="space-y-4 card p-12 text-center">
				<Radio class="mx-auto mb-4 h-16 w-16 text-success-500" />
				<div class="space-y-2">
					<h3 class="h3 text-success-500">No Issues Found</h3>
					<p class="text-surface-500-400-token">
						All aircraft addresses are unique. No duplicate addresses detected.
					</p>
				</div>
			</div>
		{:else}
			<div class="space-y-3">
				<p class="text-surface-500-400-token">
					The following addresses appear multiple times with different address types. This may
					indicate data quality issues that need attention.
				</p>

				<div class="overflow-x-auto">
					<table class="w-full table-auto">
						<thead>
							<tr class="bg-surface-200-700-token">
								<th class="p-3 text-left font-semibold">Address (Hex)</th>
								<th class="p-3 text-left font-semibold">Address Country</th>
								<th class="p-3 text-left font-semibold">Address Type</th>
								<th class="p-3 text-left font-semibold">Registration</th>
								<th class="p-3 text-left font-semibold">Aircraft Model</th>
								<th class="p-3 text-left font-semibold">From DDB</th>
								<th class="p-3 text-left font-semibold">Tracked</th>
								<th class="p-3 text-left font-semibold">Last Fix</th>
							</tr>
						</thead>
						<tbody>
							{#each duplicateDevices as device (device.id || device.address)}
								<tr class="border-surface-200-700-token hover:bg-surface-100-800-token border-b">
									<td class="p-3 font-mono font-semibold">{device.address}</td>
									<td class="p-3">
										{#if device.addressCountry}
											<img
												src={getFlagPath(device.addressCountry)}
												alt={device.addressCountry}
												class="inline-block h-4 rounded-sm"
												title={device.addressCountry}
											/>
										{:else}
											<span class="text-surface-500">—</span>
										{/if}
									</td>
									<td class="p-3">{device.addressType}</td>
									<td class="p-3">
										{#if device.id}
											<AircraftLink aircraft={device} size="sm" />
										{:else}
											{device.registration}
										{/if}
									</td>
									<td class="p-3">{device.aircraftModel || '-'}</td>
									<td class="p-3">{device.fromOgnDdb ? 'Yes' : 'No'}</td>
									<td class="p-3">{device.tracked ? 'Yes' : 'No'}</td>
									<td class="p-3">{formatDate(device.lastFixAt)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="bg-surface-100-800-token mt-4 rounded p-4">
					<p class="text-surface-600-300-token text-sm">
						<strong>Total Devices with Duplicate Addresses:</strong>
						{totalCount}
						{#if searchQuery.trim()}
							matching "{searchQuery}"
						{/if}
					</p>
					<p class="text-surface-600-300-token text-sm">
						Showing {(currentPage - 1) * perPage + 1} - {Math.min(
							currentPage * perPage,
							totalCount
						)} of {totalCount}
					</p>
				</div>

				<!-- Pagination Controls -->
				{#if totalPages > 1}
					<div class="mt-6 flex items-center justify-center gap-2">
						<button
							onclick={() => goToPage(currentPage - 1)}
							disabled={currentPage === 1 || loading}
							class="variant-filled-surface btn disabled:opacity-50"
							aria-label="Previous page"
						>
							<ChevronLeft class="h-5 w-5" />
						</button>

						<div class="flex items-center gap-1">
							{#if currentPage > 3}
								<button
									onclick={() => goToPage(1)}
									class="variant-filled-surface btn min-w-[2.5rem]"
									disabled={loading}
								>
									1
								</button>
								{#if currentPage > 4}
									<span class="px-2">...</span>
								{/if}
							{/if}

							{#each Array.from({ length: totalPages }, (_, i) => i + 1).filter((page) => page >= currentPage - 2 && page <= currentPage + 2) as page (page)}
								<button
									onclick={() => goToPage(page)}
									class="btn min-w-[2.5rem] {page === currentPage
										? 'variant-filled-primary'
										: 'variant-filled-surface'}"
									disabled={loading}
								>
									{page}
								</button>
							{/each}

							{#if currentPage < totalPages - 2}
								{#if currentPage < totalPages - 3}
									<span class="px-2">...</span>
								{/if}
								<button
									onclick={() => goToPage(totalPages)}
									class="variant-filled-surface btn min-w-[2.5rem]"
									disabled={loading}
								>
									{totalPages}
								</button>
							{/if}
						</div>

						<button
							onclick={() => goToPage(currentPage + 1)}
							disabled={currentPage === totalPages || loading}
							class="variant-filled-surface btn disabled:opacity-50"
							aria-label="Next page"
						>
							<ChevronRight class="h-5 w-5" />
						</button>
					</div>
				{/if}
			</div>
		{/if}
	</section>
</div>
