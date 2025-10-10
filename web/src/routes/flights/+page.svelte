<script lang="ts">
	import { Plane, MapPin, Clock, ExternalLink } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { getAircraftTypeOgnDescription } from '$lib/formatters';

	dayjs.extend(relativeTime);

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
		aircraft_model: string | null;
		registration: string | null;
		aircraft_type_ogn: string | null;
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

	function formatRelativeTime(dateString: string | null): string {
		if (!dateString) return '—';
		return dayjs(dateString).fromNow();
	}

	function formatLocalTime(dateString: string | null): string {
		if (!dateString) return '';
		return dayjs(dateString).format('HH:mm');
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
		<!-- Flights Table (Desktop) -->
		<section class="hidden card md:block">
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
							<th>Aircraft</th>
							<th>Takeoff</th>
							<th>Landing</th>
							<th>Duration</th>
							<th>Tow</th>
							<th>Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each flights as flight (flight.id)}
							<tr>
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
													</a>
												{:else}
													<span class="font-medium">{flight.aircraft_model}</span>
												{/if}
												<span class="text-surface-500-400-token text-sm"
													>({flight.registration})</span
												>
												{#if flight.aircraft_type_ogn}
													<span class="badge preset-filled-surface-500 text-xs">
														{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
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
												{#if flight.aircraft_type_ogn}
													<span class="badge preset-filled-surface-500 text-xs">
														{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
													</span>
												{/if}
											</div>
										{:else}
											<div class="flex items-center gap-2">
												<span class="text-surface-500-400-token font-mono text-sm">
													{formatDeviceAddress(flight.device_address, flight.device_address_type)}
												</span>
												{#if flight.aircraft_type_ogn}
													<span class="badge preset-filled-surface-500 text-xs">
														{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
													</span>
												{/if}
											</div>
										{/if}
									</div>
								</td>
								<td>
									<div class="flex flex-col gap-1">
										<div class="flex items-center gap-1 text-sm">
											<Clock class="h-3 w-3" />
											{formatRelativeTime(flight.takeoff_time)}
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
								<td>
									<div class="flex flex-col gap-1">
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
									</div>
								</td>
								<td class="font-semibold">
									{calculateFlightDuration(flight.takeoff_time, flight.landing_time)}
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
									<a
										href={`/flights/${flight.id}`}
										target="_blank"
										rel="noopener noreferrer"
										class="preset-tonal-surface-500 btn flex items-center gap-1 btn-sm"
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
		</section>

		<!-- Flights Cards (Mobile) -->
		<div class="block space-y-4 md:hidden">
			<div class="card-header">
				<h2 class="h2">Completed Flights</h2>
				<p class="text-surface-500-400-token">
					{flights.length} flight{flights.length === 1 ? '' : 's'} found
				</p>
			</div>

			{#each flights as flight (flight.id)}
				<div class="card p-4">
					<!-- Aircraft info -->
					<div class="border-surface-200-700-token mb-3 border-b pb-3">
						<div class="flex items-center justify-between">
							<div class="flex flex-col gap-1">
								{#if flight.aircraft_model && flight.registration}
									<div class="flex items-center gap-2">
										{#if flight.device_id}
											<a
												href={`/devices/${flight.device_id}`}
												class="anchor font-medium text-primary-500"
											>
												{flight.aircraft_model}
											</a>
										{:else}
											<span class="font-medium">{flight.aircraft_model}</span>
										{/if}
										<span class="text-surface-500-400-token text-sm">({flight.registration})</span>
									</div>
								{:else if flight.registration}
									<div class="flex items-center gap-2">
										{#if flight.device_id}
											<a
												href={`/devices/${flight.device_id}`}
												class="anchor font-medium text-primary-500"
											>
												{flight.registration}
											</a>
										{:else}
											<span class="font-medium">{flight.registration}</span>
										{/if}
									</div>
								{:else}
									<span class="text-surface-500-400-token font-mono text-sm">
										{formatDeviceAddress(flight.device_address, flight.device_address_type)}
									</span>
								{/if}
								{#if flight.aircraft_type_ogn}
									<span class="badge preset-filled-surface-500 text-xs">
										{getAircraftTypeOgnDescription(flight.aircraft_type_ogn)}
									</span>
								{/if}
							</div>
							<a
								href={`/flights/${flight.id}`}
								target="_blank"
								rel="noopener noreferrer"
								class="preset-tonal-surface-500 btn btn-sm"
							>
								<ExternalLink class="h-3 w-3" />
							</a>
						</div>
					</div>

					<!-- Flight details -->
					<div class="space-y-2 text-sm">
						<!-- Takeoff -->
						<div class="flex justify-between">
							<span class="text-surface-600-300-token">Takeoff:</span>
							<div class="flex flex-col items-end gap-0.5">
								<span>{formatRelativeTime(flight.takeoff_time)}</span>
								{#if flight.takeoff_time}
									<span class="text-surface-500-400-token text-xs">
										{formatLocalTime(flight.takeoff_time)}
									</span>
								{/if}
								{#if flight.departure_airport}
									<span class="text-surface-500-400-token flex items-center gap-1 text-xs">
										<MapPin class="h-3 w-3" />
										{flight.departure_airport}
									</span>
								{/if}
							</div>
						</div>

						<!-- Landing -->
						<div class="flex justify-between">
							<span class="text-surface-600-300-token">Landing:</span>
							<div class="flex flex-col items-end gap-0.5">
								<span>{formatRelativeTime(flight.landing_time)}</span>
								{#if flight.landing_time}
									<span class="text-surface-500-400-token text-xs">
										{formatLocalTime(flight.landing_time)}
									</span>
								{/if}
								{#if flight.arrival_airport}
									<span class="text-surface-500-400-token flex items-center gap-1 text-xs">
										<MapPin class="h-3 w-3" />
										{flight.arrival_airport}
									</span>
								{/if}
							</div>
						</div>

						<!-- Duration -->
						<div class="flex justify-between">
							<span class="text-surface-600-300-token">Duration:</span>
							<span class="font-semibold">
								{calculateFlightDuration(flight.takeoff_time, flight.landing_time)}
							</span>
						</div>

						<!-- Tow (if applicable) -->
						{#if flight.tow_aircraft_id}
							<div class="flex justify-between">
								<span class="text-surface-600-300-token">Tow:</span>
								<div class="flex flex-col items-end gap-0.5">
									<span class="text-xs">{flight.tow_aircraft_id}</span>
									{#if flight.tow_release_height_msl}
										<span class="text-surface-500-400-token text-xs">
											{flight.tow_release_height_msl}m MSL
										</span>
									{/if}
								</div>
							</div>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
