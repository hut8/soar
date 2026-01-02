<script lang="ts">
	import { Plane, ChevronLeft, ChevronRight, Activity, CheckCircle } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';
	import FlightsList from '$lib/components/FlightsList.svelte';
	import type { Flight, PaginatedDataResponse } from '$lib/types';

	let flights: Flight[] = [];
	let totalCount = 0;
	let currentPage = 1;
	const ITEMS_PER_PAGE = 50;
	let loading = true;
	let error = '';
	let flightType = $state<'active' | 'completed'>('active');

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
			const offset = (currentPage - 1) * ITEMS_PER_PAGE;
			const completed = flightType === 'completed';

			const response = await serverCall<PaginatedDataResponse<Flight>>(
				`/flights?completed=${completed}&limit=${ITEMS_PER_PAGE}&offset=${offset}`
			);

			flights = response.data || [];
			totalCount = response.metadata.totalCount || 0;
		} catch (err) {
			const errorMessage = extractErrorMessage(err);
			error = `Failed to load flights: ${errorMessage}`;
			console.error('Error loading flights:', err);
			flights = [];
			totalCount = 0;
		} finally {
			loading = false;
		}
	}

	function handlePageChange(newPage: number) {
		currentPage = newPage;
		loadFlights();
	}

	// When flight type changes, reset to page 1 and reload
	$effect(() => {
		currentPage = 1;
		loadFlights();
	});

	const totalPages = $derived(Math.ceil(totalCount / ITEMS_PER_PAGE));

	onMount(() => {
		loadFlights();
	});
</script>

<svelte:head>
	<title>Flights - Aircraft Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-4 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<Plane class="h-8 w-8" />
			Flights
		</h1>

		<!-- Flight Type Tabs -->
		<div class="flex justify-center gap-2">
			<button
				class="btn {flightType === 'active'
					? 'preset-filled-primary-500'
					: 'preset-tonal-surface-500'}"
				onclick={() => (flightType = 'active')}
			>
				<Activity size={16} />
				Active Flights
			</button>
			<button
				class="btn {flightType === 'completed'
					? 'preset-filled-primary-500'
					: 'preset-tonal-surface-500'}"
				onclick={() => (flightType = 'completed')}
			>
				<CheckCircle size={16} />
				Completed Flights
			</button>
		</div>
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
	{:else if flights.length === 0}
		<!-- Empty State -->
		<div class="space-y-4 card p-12 text-center">
			<Plane class="mx-auto mb-4 h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">
					No {flightType === 'active' ? 'active' : 'completed'} flights found
				</h3>
				<p class="text-surface-500-400-token">
					{#if flightType === 'active'}
						No flights are currently in progress.
					{:else}
						No completed flights have been recorded yet.
					{/if}
				</p>
			</div>
		</div>
	{:else}
		<!-- Flights List -->
		<section class="card">
			<header class="card-header">
				<h2 class="h2">
					{flightType === 'active' ? 'Active Flights' : 'Completed Flights'}
				</h2>
				<p class="text-surface-500-400-token">
					{totalCount} flight{totalCount === 1 ? '' : 's'}
					{flightType === 'active' ? 'in progress' : 'found'}
				</p>
			</header>

			<FlightsList {flights} showEnd={flightType === 'completed'} showAircraft={true} />

			<!-- Pagination -->
			{#if totalPages > 1}
				<footer class="card-footer flex items-center justify-between">
					<div class="text-surface-500-400-token text-sm">
						Page {currentPage} of {totalPages}
					</div>
					<div class="flex gap-2">
						<button
							class="btn preset-tonal-surface btn-sm"
							disabled={currentPage === 1}
							onclick={() => handlePageChange(currentPage - 1)}
						>
							<ChevronLeft class="h-4 w-4" />
							Previous
						</button>
						<button
							class="btn preset-tonal-surface btn-sm"
							disabled={currentPage === totalPages}
							onclick={() => handlePageChange(currentPage + 1)}
						>
							Next
							<ChevronRight class="h-4 w-4" />
						</button>
					</div>
				</footer>
			{/if}
		</section>
	{/if}
</div>
