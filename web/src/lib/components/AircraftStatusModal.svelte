<script lang="ts">
	import { X, Plane, MapPin, Clock, RotateCcw } from '@lucide/svelte';
	import { Device, type Fix, type AircraftRegistration, type AircraftModel } from '$lib/types';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';

	// Extend dayjs with relative time plugin
	dayjs.extend(relativeTime);

	// Props
	let { showModal = $bindable(), selectedDevice = $bindable() } = $props<{
		showModal: boolean;
		selectedDevice: Device | null;
	}>();

	// Reactive variables
	let aircraftRegistration: AircraftRegistration | null = $state(null);
	let aircraftModel: AircraftModel | null = $state(null);
	let loadingRegistration = $state(false);
	let loadingModel = $state(false);
	let recentFixes: Fix[] = $state([]);

	// Update data when device changes
	$effect(() => {
		if (selectedDevice) {
			loadAircraftData();
		} else {
			aircraftRegistration = null;
			aircraftModel = null;
			recentFixes = [];
		}
	});

	async function loadAircraftData() {
		if (!selectedDevice) return;

		// Get recent fixes (last 24 hours)
		recentFixes = selectedDevice.getRecentFixes(24);

		// Load aircraft registration and model data in parallel
		loadingRegistration = true;
		loadingModel = true;

		try {
			const [registration, model] = await Promise.all([
				selectedDevice.getAircraftRegistration(),
				selectedDevice.getAircraftModel()
			]);

			aircraftRegistration = registration;
			aircraftModel = model;
		} catch (error) {
			console.warn('Failed to load aircraft data:', error);
			aircraftRegistration = null;
			aircraftModel = null;
		} finally {
			loadingRegistration = false;
			loadingModel = false;
		}
	}

	function closeModal() {
		showModal = false;
		selectedDevice = null;
	}

	function formatAltitude(altitude_feet: number | undefined): string {
		if (altitude_feet === undefined || altitude_feet === null) return 'Unknown';
		return `${altitude_feet.toLocaleString()} ft`;
	}

	function formatSpeed(speed_knots: number | undefined): string {
		if (speed_knots === undefined || speed_knots === null) return 'Unknown';
		return `${Math.round(speed_knots)} kts`;
	}

	function formatClimbRate(climb_fpm: number | undefined): string {
		if (climb_fpm === undefined || climb_fpm === null) return 'Unknown';
		const sign = climb_fpm >= 0 ? '+' : '';
		return `${sign}${Math.round(climb_fpm)} fpm`;
	}

	function formatTrack(track_degrees: number | undefined): string {
		if (track_degrees === undefined || track_degrees === null) return 'Unknown';
		return `${Math.round(track_degrees)}°`;
	}

	function formatCoordinates(lat: number, lng: number): string {
		const latDir = lat >= 0 ? 'N' : 'S';
		const lngDir = lng >= 0 ? 'E' : 'W';
		return `${Math.abs(lat).toFixed(4)}°${latDir}, ${Math.abs(lng).toFixed(4)}°${lngDir}`;
	}

	function formatTimestamp(timestamp: string): { relative: string; absolute: string } {
		const time = dayjs(timestamp);
		return {
			relative: time.fromNow(),
			absolute: time.format('YYYY-MM-DD HH:mm:ss UTC')
		};
	}

	// Get aircraft type description
	function getAircraftTypeDescription(typeAircraft: number | undefined): string {
		if (typeAircraft === undefined) return 'Unknown';

		// FAA aircraft type codes
		const typeMap: Record<number, string> = {
			1: 'Glider',
			2: 'Balloon',
			3: 'Blimp/Dirigible',
			4: 'Fixed wing single engine',
			5: 'Fixed wing multi engine',
			6: 'Rotorcraft',
			7: 'Weight-shift-control',
			8: 'Powered parachute',
			9: 'Gyroplane'
		};

		return typeMap[typeAircraft] || `Type ${typeAircraft}`;
	}
</script>

<!-- Aircraft Status Modal -->
{#if showModal && selectedDevice}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-surface-950-50/50"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		tabindex="-1"
		role="dialog"
	>
		<div
			class="max-h-[90vh] w-full max-w-4xl overflow-y-auto card bg-white text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			tabindex="0"
		>
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-gray-200 p-6">
				<div class="flex items-center gap-3">
					<div
						class="flex h-10 w-10 items-center justify-center rounded-full bg-red-500 text-white"
					>
						<Plane size={24} />
					</div>
					<div>
						<h2 class="text-xl font-bold">Aircraft Status</h2>
						<p class="text-sm text-gray-600">
							{selectedDevice.registration || selectedDevice.address}
							{#if selectedDevice.aircraft_model}
								• {selectedDevice.aircraft_model}
							{/if}
						</p>
					</div>
				</div>
				<button class="variant-ghost-surface btn btn-sm" onclick={closeModal}>
					<X size={20} />
				</button>
			</div>

			<div class="p-6">
				<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
					<!-- Aircraft Information -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Plane size={20} />
							Aircraft Information
						</h3>

						<div class="space-y-3">
							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-gray-600">Registration</dt>
									<dd class="font-mono text-sm">
										{selectedDevice.registration || 'Unknown'}
									</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">Address</dt>
									<dd class="font-mono text-sm">
										{selectedDevice.address}
										{#if selectedDevice.address_type}
											({selectedDevice.address_type})
										{/if}
									</dd>
								</div>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-gray-600">Aircraft Model</dt>
									<dd class="text-sm">
										{selectedDevice.aircraft_model || 'Unknown'}
									</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">Competition Number</dt>
									<dd class="text-sm">
										{selectedDevice.cn || 'None'}
									</dd>
								</div>
							</div>

							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-gray-600">Tracked</dt>
									<dd class="text-sm">
										<span
											class="badge variant-filled-{selectedDevice.tracked ? 'success' : 'warning'}"
										>
											{selectedDevice.tracked ? 'Yes' : 'No'}
										</span>
									</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-gray-600">Identified</dt>
									<dd class="text-sm">
										<span
											class="badge variant-filled-{selectedDevice.identified
												? 'success'
												: 'warning'}"
										>
											{selectedDevice.identified ? 'Yes' : 'No'}
										</span>
									</dd>
								</div>
							</div>
						</div>

						<!-- Aircraft Registration Details -->
						{#if loadingRegistration}
							<div class="flex items-center gap-2 text-sm text-gray-600">
								<RotateCcw class="animate-spin" size={16} />
								Loading aircraft registration...
							</div>
						{:else if aircraftRegistration}
							<div class="space-y-3 border-t border-gray-200 pt-4">
								<h4 class="font-medium text-gray-900">FAA Registration Details</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-gray-600">Owner</dt>
										<dd>{aircraftRegistration.registrant_name}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Serial Number</dt>
										<dd class="font-mono">{aircraftRegistration.serial_number}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Manufacturer</dt>
										<dd>{aircraftRegistration.mfr_mdl_code}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Year</dt>
										<dd>{aircraftRegistration.year_mfr}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Type</dt>
										<dd>{getAircraftTypeDescription(aircraftRegistration.type_aircraft)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Status</dt>
										<dd class="font-mono">{aircraftRegistration.status_code}</dd>
									</div>
								</dl>
							</div>
						{/if}

						<!-- Aircraft Model Details -->
						{#if loadingModel}
							<div class="flex items-center gap-2 text-sm text-gray-600">
								<RotateCcw class="animate-spin" size={16} />
								Loading aircraft model details...
							</div>
						{:else if aircraftModel}
							<div class="space-y-3 border-t border-gray-200 pt-4">
								<h4 class="font-medium text-gray-900">Aircraft Model Details</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-gray-600">Manufacturer</dt>
										<dd>{aircraftModel.manufacturer}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Model</dt>
										<dd>{aircraftModel.model}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Type Certificate</dt>
										<dd class="font-mono">{aircraftModel.type_certificate || 'N/A'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Seats</dt>
										<dd>{aircraftModel.seats || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Engines</dt>
										<dd>{aircraftModel.engines || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Weight</dt>
										<dd>{aircraftModel.weight ? `${aircraftModel.weight} lbs` : 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Max Speed</dt>
										<dd>{aircraftModel.speed ? `${aircraftModel.speed} kts` : 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Weight Class</dt>
										<dd>{aircraftModel.ac_weight || 'Unknown'}</dd>
									</div>
								</dl>
							</div>
						{/if}
					</div>

					<!-- Recent Activity -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Clock size={20} />
							Recent Activity
							<span class="text-sm font-normal text-gray-600">
								({recentFixes.length} fixes in last 24h)
							</span>
						</h3>

						{#if recentFixes.length === 0}
							<div class="py-8 text-center text-gray-500">
								<MapPin size={48} class="mx-auto mb-2 opacity-50" />
								<p>No recent position data available</p>
							</div>
						{:else}
							<!-- Latest Fix Summary -->
							{@const latestFix = recentFixes[0]}
							<div class="rounded-lg border border-gray-200 bg-gray-50 p-4">
								<h4 class="mb-3 font-medium text-gray-900">Latest Position</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-gray-600">Altitude</dt>
										<dd>{formatAltitude(latestFix.altitude_feet)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Ground Speed</dt>
										<dd>{formatSpeed(latestFix.ground_speed_knots)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Track</dt>
										<dd>{formatTrack(latestFix.track_degrees)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Climb Rate</dt>
										<dd>{formatClimbRate(latestFix.climb_fpm)}</dd>
									</div>
									<div class="col-span-2">
										<dt class="font-medium text-gray-600">Coordinates</dt>
										<dd class="font-mono">
											{formatCoordinates(latestFix.latitude, latestFix.longitude)}
										</dd>
									</div>
									<div class="col-span-2">
										<dt class="font-medium text-gray-600">Last Seen</dt>
										<dd>
											{formatTimestamp(latestFix.timestamp).relative}
											<div class="text-xs text-gray-500">
												{formatTimestamp(latestFix.timestamp).absolute}
											</div>
										</dd>
									</div>
								</dl>
							</div>

							<!-- Recent Fixes List -->
							<div class="max-h-64 overflow-y-auto rounded-lg border border-gray-200">
								<table class="w-full text-sm">
									<thead class="border-b border-gray-200 bg-gray-50">
										<tr>
											<th class="px-3 py-2 text-left font-medium text-gray-600">Time</th>
											<th class="px-3 py-2 text-left font-medium text-gray-600">Altitude</th>
											<th class="px-3 py-2 text-left font-medium text-gray-600">Speed</th>
											<th class="px-3 py-2 text-left font-medium text-gray-600">Track</th>
										</tr>
									</thead>
									<tbody>
										{#each recentFixes.slice(0, 20) as fix (fix.id)}
											<tr class="border-b border-gray-100 hover:bg-gray-50">
												<td class="px-3 py-2">
													{formatTimestamp(fix.timestamp).relative}
												</td>
												<td class="px-3 py-2">
													{formatAltitude(fix.altitude_feet)}
												</td>
												<td class="px-3 py-2">
													{formatSpeed(fix.ground_speed_knots)}
												</td>
												<td class="px-3 py-2">
													{formatTrack(fix.track_degrees)}
												</td>
											</tr>
										{/each}
									</tbody>
								</table>
							</div>

							{#if recentFixes.length > 20}
								<p class="text-center text-xs text-gray-500">
									Showing latest 20 of {recentFixes.length} fixes
								</p>
							{/if}
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
