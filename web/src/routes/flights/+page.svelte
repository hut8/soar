<script lang="ts">
	import { Plane, Calendar, MapPin, Clock } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';

	interface Flight {
		id: string;
		device_address: string;
		device_address_type: string;
		takeoff_time: string | null;
		landing_time: string | null;
		departure_airport: string | null;
		arrival_airport: string | null;
		tow_aircraft_id: string | null;
		tow_release_height_msl: number | null;
		takeoff_altitude_offset_ft: number | null;
		landing_altitude_offset_ft: number | null;
		device_id: string | null;
		created_at: string;
		updated_at: string;
	}

	let flights: Flight[] = [];
	let loading = true;
	let error = '';

	function formatDeviceAddress(address: string, addressType: string): string {
		const typePrefix = addressType === 'Flarm' ? 'F' : addressType === 'Ogn' ? 'O' : 'I';
		return `${typePrefix}-${address}`;
	}

	function formatDateTime(dateString: string | null): string {
		if (!dateString) return '—';
		const date = new Date(dateString);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function formatDate(dateString: string | null): string {
		if (!dateString) return '—';
		const date = new Date(dateString);
		return date.toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	function calculateFlightDuration(takeoff: string | null, landing: string | null): string {
		if (!takeoff || !landing) return '—';
		const takeoffTime = new Date(takeoff).getTime();
		const landingTime = new Date(landing).getTime();
		const durationMs = landingTime - takeoffTime;
		const hours = Math.floor(durationMs / (1000 * 60 * 60));
		const minutes = Math.floor((durationMs % (1000 * 60 * 60)) / (1000 * 60));
		return `${hours}h ${minutes}m`;
	}

	async function loadFlights() {
		loading = true;
		error = '';

		try {
			const response = await serverCall<Flight[]>('/flights/completed?limit=100');
			flights = response || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load flights: ${errorMessage}`;
			console.error('Error loading flights:', err);
			flights = [];
		} finally {
			loading = false;
		}
	}

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
			Recent Completed Flights
		</h1>
		<p class="text-surface-500-400-token">Showing the most recent 100 completed flights</p>
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
				<h3 class="h3">No completed flights found</h3>
				<p class="text-surface-500-400-token">No flights have been completed yet.</p>
			</div>
		</div>
	{:else}
		<!-- Flights Table -->
		<section class="card">
			<header class="card-header">
				<h2 class="h2">Completed Flights</h2>
				<p class="text-surface-500-400-token">
					{flights.length} flight{flights.length === 1 ? '' : 's'} found
				</p>
			</header>

			<div class="table-container">
				<table class="table-hover table">
					<thead>
						<tr>
							<th>Device</th>
							<th>Takeoff</th>
							<th>Landing</th>
							<th>Duration</th>
							<th>Route</th>
							<th>Tow</th>
							<th>Date</th>
						</tr>
					</thead>
					<tbody>
						{#each flights as flight (flight.id)}
							<tr>
								<td>
									<a
										href={`/flights/${flight.id}`}
										class="anchor font-mono text-primary-500 hover:text-primary-600"
									>
										{formatDeviceAddress(flight.device_address, flight.device_address_type)}
									</a>
								</td>
								<td>
									<div class="flex items-center gap-1 text-sm">
										<Clock class="h-3 w-3" />
										{formatDateTime(flight.takeoff_time).split(', ')[1] || '—'}
									</div>
									{#if flight.departure_airport}
										<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
											<MapPin class="h-3 w-3" />
											{flight.departure_airport}
										</div>
									{/if}
								</td>
								<td>
									<div class="flex items-center gap-1 text-sm">
										<Clock class="h-3 w-3" />
										{formatDateTime(flight.landing_time).split(', ')[1] || '—'}
									</div>
									{#if flight.arrival_airport}
										<div class="text-surface-500-400-token flex items-center gap-1 text-xs">
											<MapPin class="h-3 w-3" />
											{flight.arrival_airport}
										</div>
									{/if}
								</td>
								<td class="font-semibold">
									{calculateFlightDuration(flight.takeoff_time, flight.landing_time)}
								</td>
								<td>
									{#if flight.departure_airport && flight.arrival_airport}
										{#if flight.departure_airport === flight.arrival_airport}
											<span class="variant-soft-primary badge text-xs">Local</span>
										{:else}
											<span class="variant-soft-secondary badge text-xs">
												{flight.departure_airport} → {flight.arrival_airport}
											</span>
										{/if}
									{:else}
										<span class="text-surface-500">—</span>
									{/if}
								</td>
								<td>
									{#if flight.tow_aircraft_id}
										<div class="flex flex-col gap-1">
											<span class="text-xs">{flight.tow_aircraft_id}</span>
											{#if flight.tow_release_height_msl}
												<span class="text-surface-500-400-token text-xs">
													{flight.tow_release_height_msl}m MSL
												</span>
											{/if}
										</div>
									{:else}
										<span class="text-surface-500">—</span>
									{/if}
								</td>
								<td>
									<div class="text-surface-600-300-token flex items-center gap-1 text-sm">
										<Calendar class="h-3 w-3" />
										{formatDate(flight.landing_time)}
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</section>
	{/if}
</div>
