<script lang="ts">
	import { Plane, ChevronLeft, ChevronRight } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';
	import FlightsList from '$lib/components/FlightsList.svelte';
	import type { Flight } from '$lib/types';

	interface FlightsListResponse {
		flights: Flight[];
		total_count: number;
	}

	let completedFlights: Flight[] = [];
	let activeFlights: Flight[] = [];
	let activeTotalCount = 0;
	let completedTotalCount = 0;
	let activeCurrentPage = 1;
	let completedCurrentPage = 1;
	const ITEMS_PER_PAGE = 50;
	let loading = true;
	let error = '';

	function extractErrorMessage(err: unknown): string {
		if (err instanceof Error) {
			// Try to parse the error message as JSON
			try {
				const parsed = JSON.parse(err.message);
				if (parsed && typeof parsed === 'object' && 'errors' in parsed) {
					return String(parsed.errors);
				}
			} catch {
				// Not JSON, return the original message
			}
			return err.message;
		}
		return 'Unknown error';
	}

	async function loadFlights() {
		loading = true;
		error = '';

		try {
			const activeOffset = (activeCurrentPage - 1) * ITEMS_PER_PAGE;
			const completedOffset = (completedCurrentPage - 1) * ITEMS_PER_PAGE;

			// Load both active and completed flights in parallel
			const [activeResponse, completedResponse] = await Promise.all([
				serverCall<FlightsListResponse>(
					`/flights?completed=false&limit=${ITEMS_PER_PAGE}&offset=${activeOffset}`
				),
				serverCall<FlightsListResponse>(
					`/flights?completed=true&limit=${ITEMS_PER_PAGE}&offset=${completedOffset}`
				)
			]);

			activeFlights = activeResponse.flights || [];
			activeTotalCount = activeResponse.total_count || 0;
			completedFlights = completedResponse.flights || [];
			completedTotalCount = completedResponse.total_count || 0;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load flights: ${errorMessage}`;
			console.error('Error loading flights:', err);
			activeFlights = [];
			completedFlights = [];
			activeTotalCount = 0;
			completedTotalCount = 0;
		} finally {
			loading = false;
		}
	}

	function handleActivePageChange(newPage: number) {
		activeCurrentPage = newPage;
		loadFlights();
	}

	function handleCompletedPageChange(newPage: number) {
		completedCurrentPage = newPage;
		loadFlights();
	}

	$: activeTotalPages = Math.ceil(activeTotalCount / ITEMS_PER_PAGE);
	$: completedTotalPages = Math.ceil(completedTotalCount / ITEMS_PER_PAGE);

	onMount(() => {
		loadFlights();
	});
</script>

<svelte:head>
	<title>Flights - Aircraft Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Plane class="h-8 w-8" />
			Flights
		</h1>
		<p class="text-surface-500-400-token">Active flights and recent completed flights</p>
	</header>

	<!-- Loading State -->
	{#if loading}
		<div class="space-y-4 card p-12 text-center">
			<div
				class="mx-auto h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
			<p class="text-surface-500-400-token">Loading flights...</p>
		</div>
	{:else if error}
		<!-- Error State -->
		<div class="space-y-4 card p-12 text-center">
			<div class="rounded border border-red-200 bg-red-50 p-4 text-red-600">
				{error}
			</div>
			<button class="btn preset-filled-primary-500" onclick={loadFlights}> Try Again </button>
		</div>
	{:else if activeFlights.length === 0 && completedFlights.length === 0}
		<!-- Empty State -->
		<div class="space-y-4 card p-12 text-center">
			<Plane class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No flights found</h3>
				<p class="text-surface-500-400-token">No flights have been recorded yet.</p>
			</div>
		</div>
	{:else}
		<!-- Active Flights -->
		{#if activeFlights.length > 0}
			<section class="card">
				<header class="card-header">
					<h2 class="h2">Active Flights</h2>
					<p class="text-surface-500-400-token">
						{activeTotalCount} flight{activeTotalCount === 1 ? '' : 's'} in progress
					</p>
				</header>

				<FlightsList flights={activeFlights} showEnd={false} showAircraft={true} />

				<!-- Active Flights Pagination -->
				{#if activeTotalPages > 1}
					<footer class="card-footer flex items-center justify-between">
						<div class="text-surface-500-400-token text-sm">
							Page {activeCurrentPage} of {activeTotalPages}
						</div>
						<div class="flex gap-2">
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={activeCurrentPage === 1}
								onclick={() => handleActivePageChange(activeCurrentPage - 1)}
							>
								<ChevronLeft class="h-4 w-4" />
								Previous
							</button>
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={activeCurrentPage === activeTotalPages}
								onclick={() => handleActivePageChange(activeCurrentPage + 1)}
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						</div>
					</footer>
				{/if}
			</section>
		{/if}

		<!-- Completed Flights -->
		{#if completedFlights.length > 0}
			<section class="card">
				<header class="card-header">
					<h2 class="h2">Completed Flights</h2>
					<p class="text-surface-500-400-token">
						{completedTotalCount} flight{completedTotalCount === 1 ? '' : 's'} found
					</p>
				</header>

				<FlightsList flights={completedFlights} showEnd={true} showAircraft={true} />

				<!-- Completed Flights Pagination -->
				{#if completedTotalPages > 1}
					<footer class="card-footer flex items-center justify-between">
						<div class="text-surface-500-400-token text-sm">
							Page {completedCurrentPage} of {completedTotalPages}
						</div>
						<div class="flex gap-2">
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={completedCurrentPage === 1}
								onclick={() => handleCompletedPageChange(completedCurrentPage - 1)}
							>
								<ChevronLeft class="h-4 w-4" />
								Previous
							</button>
							<button
								class="btn preset-tonal-surface btn-sm"
								disabled={completedCurrentPage === completedTotalPages}
								onclick={() => handleCompletedPageChange(completedCurrentPage + 1)}
							>
								Next
								<ChevronRight class="h-4 w-4" />
							</button>
						</div>
					</footer>
				{/if}
			</section>
		{/if}
	{/if}
</div>
