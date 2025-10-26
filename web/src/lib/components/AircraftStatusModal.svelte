<script lang="ts">
	import { X, Plane, MapPin, Clock, RotateCcw, ExternalLink } from '@lucide/svelte';
	import type { Device, Aircraft, Fix, AircraftRegistration, AircraftModel } from '$lib/types';
	import {
		formatTitleCase,
		formatDeviceAddress,
		getStatusCodeDescription,
		getAircraftTypeOgnDescription,
		getAircraftTypeColor
	} from '$lib/formatters';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { onMount } from 'svelte';

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

	// Direction arrow variables
	let userLocation: { lat: number; lng: number } | null = $state(null);
	let deviceHeading: number = $state(0);
	let isCompassActive: boolean = $state(false);
	let directionToAircraft: number = $state(0);

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

		// Load aircraft registration and model data in parallel
		loadingRegistration = true;
		loadingModel = true;

		try {
			// Import DeviceRegistry to fetch recent fixes
			const { DeviceRegistry } = await import('$lib/services/DeviceRegistry');
			const registry = DeviceRegistry.getInstance();

			// Load recent fixes from API
			await registry.loadRecentFixesFromAPI(selectedDevice.id, 100);

			// Update recent fixes from the device (last 24 hours)
			const now = Date.now();
			const twentyFourHoursAgo = now - 24 * 60 * 60 * 1000;
			const allFixes = selectedDevice.fixes || [];
			recentFixes = allFixes.filter((fix: Fix) => {
				const fixTime = new Date(fix.timestamp).getTime();
				return fixTime > twentyFourHoursAgo;
			});

			// Check if device is actually an Aircraft with registration/model data
			const aircraft = selectedDevice as Aircraft;
			aircraftRegistration = aircraft.aircraft_registration || null;
			aircraftModel = aircraft.aircraft_model_details || null;
		} catch (error) {
			console.warn('Failed to load aircraft data:', error);
			aircraftRegistration = null;
			aircraftModel = null;
			recentFixes = [];
		} finally {
			loadingRegistration = false;
			loadingModel = false;
		}
	}

	function closeModal() {
		showModal = false;
		selectedDevice = null;
	}

	function formatAltitude(altitude_msl_feet: number | undefined): string {
		if (altitude_msl_feet === undefined || altitude_msl_feet === null) return 'Unknown';
		return `${altitude_msl_feet.toLocaleString()} ft`;
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

	// Calculate bearing from user to aircraft
	function calculateBearing(lat1: number, lng1: number, lat2: number, lng2: number): number {
		const toRadians = (deg: number) => (deg * Math.PI) / 180;
		const toDegrees = (rad: number) => (rad * 180) / Math.PI;

		const dLng = toRadians(lng2 - lng1);
		const lat1Rad = toRadians(lat1);
		const lat2Rad = toRadians(lat2);

		const y = Math.sin(dLng) * Math.cos(lat2Rad);
		const x =
			Math.cos(lat1Rad) * Math.sin(lat2Rad) -
			Math.sin(lat1Rad) * Math.cos(lat2Rad) * Math.cos(dLng);

		let bearing = toDegrees(Math.atan2(y, x));
		bearing = (bearing + 360) % 360; // Normalize to 0-360

		return bearing;
	}

	// Update direction to aircraft
	function updateDirectionToAircraft() {
		if (!userLocation || !selectedDevice) {
			return;
		}

		const fixes = selectedDevice.fixes || [];
		const latestFix = fixes.length > 0 ? fixes[0] : null;
		if (!latestFix) {
			return;
		}

		// Calculate bearing from user to aircraft (absolute bearing from north)
		const bearing = calculateBearing(
			userLocation.lat,
			userLocation.lng,
			latestFix.latitude,
			latestFix.longitude
		);

		// The arrow should point toward the aircraft and stay pointing there as phone rotates
		// Add device heading to rotate arrow opposite to phone rotation
		// When phone points north (deviceHeading = 0), arrow shows absolute bearing
		// When phone rotates clockwise, arrow rotates counter-clockwise to keep pointing at aircraft
		directionToAircraft = (bearing + deviceHeading) % 360;
	}

	// Handle device orientation changes
	function handleOrientationChange(event: DeviceOrientationEvent) {
		if (event.alpha !== null) {
			isCompassActive = true;
			deviceHeading = event.alpha;
			updateDirectionToAircraft();
		}
	}

	// Get user location
	async function getUserLocation() {
		if (!navigator.geolocation) {
			return;
		}

		try {
			const position = await new Promise<GeolocationPosition>((resolve, reject) => {
				navigator.geolocation.getCurrentPosition(resolve, reject, {
					enableHighAccuracy: true,
					timeout: 10000,
					maximumAge: 300000 // 5 minutes
				});
			});

			userLocation = {
				lat: position.coords.latitude,
				lng: position.coords.longitude
			};

			updateDirectionToAircraft();
		} catch (error) {
			console.warn('Failed to get user location:', error);
		}
	}

	// Initialize compass on mount
	onMount(() => {
		// Request device orientation permission for iOS 13+
		if (
			'requestPermission' in DeviceOrientationEvent &&
			typeof DeviceOrientationEvent.requestPermission === 'function'
		) {
			DeviceOrientationEvent.requestPermission()
				.then((permission: PermissionState) => {
					if (permission === 'granted') {
						window.addEventListener('deviceorientation', handleOrientationChange);
					}
				})
				.catch((error: unknown) => {
					console.warn('Device orientation permission denied:', error);
				});
		} else {
			// Add listener for other browsers
			window.addEventListener('deviceorientation', handleOrientationChange);
		}

		// Get user location
		getUserLocation();

		// Cleanup
		return () => {
			window.removeEventListener('deviceorientation', handleOrientationChange);
		};
	});

	// Update direction when device changes
	$effect(() => {
		if (selectedDevice) {
			updateDirectionToAircraft();
		}
	});
</script>

<!-- Aircraft Status Modal -->
{#if showModal && selectedDevice}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-surface-950-50/50 pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		tabindex="-1"
		role="dialog"
	>
		<div
			class="relative max-h-[calc(90vh-5rem)] w-full max-w-4xl overflow-y-auto card bg-white text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			tabindex="0"
		>
			<!-- Header -->
			<div class="flex items-center justify-between border-b border-gray-200 p-6">
				<div class="flex items-center gap-3">
					{#if isCompassActive && userLocation}
						<!-- Direction arrow pointing to aircraft -->
						<div class="flex flex-col items-center">
							<div
								class="direction-arrow"
								style="transform: rotate({directionToAircraft}deg)"
								title="Arrow points toward the aircraft"
							>
								<svg width="40" height="40" viewBox="0 0 40 40">
									<!-- Red arrow pointing up -->
									<path
										d="M 20 8 L 26 20 L 22 20 L 22 32 L 18 32 L 18 20 L 14 20 Z"
										fill="#dc2626"
										stroke="#991b1b"
										stroke-width="1.5"
									/>
								</svg>
							</div>
							<div class="mt-1 text-xs font-semibold text-gray-700">
								{Math.round(directionToAircraft)}°
							</div>
						</div>
					{:else}
						<!-- Static plane icon when compass not available -->
						<div
							class="flex h-10 w-10 items-center justify-center rounded-full bg-red-500 text-white"
						>
							<Plane size={24} />
						</div>
					{/if}
					<div>
						<h2 class="text-xl font-bold">Aircraft Status</h2>
						<p class="text-sm text-gray-600">
							{selectedDevice.registration ||
								formatDeviceAddress(selectedDevice.address_type, selectedDevice.address)}
							{#if selectedDevice.aircraft_model}
								• {selectedDevice.aircraft_model}
							{/if}
						</p>
					</div>
				</div>
				<div class="flex items-center gap-2">
					<a
						href="/devices/{selectedDevice.id}"
						target="_blank"
						rel="noopener noreferrer"
						class="btn preset-filled-primary-500 btn-sm"
						title="View detailed device page"
					>
						<ExternalLink size={16} />
						View Details
					</a>
					<button class="preset-tonal-surface-500 btn btn-sm" onclick={closeModal}>
						<X size={20} />
					</button>
				</div>
			</div>

			<div class="p-6">
				<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
					<!-- Aircraft Information -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Plane size={20} />
							Aircraft Information
						</h3>

						<div class="rounded-lg border border-gray-200 bg-gray-50 p-4">
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
											{formatDeviceAddress(selectedDevice.address_type, selectedDevice.address)}
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
											{selectedDevice.competition_number || 'None'}
										</dd>
									</div>
								</div>

								<div class="grid grid-cols-2 gap-4">
									<div>
										<dt class="text-sm font-medium text-gray-600">Tracked</dt>
										<dd class="text-sm">
											<span
												class="badge preset-filled-{selectedDevice.tracked
													? 'success-500'
													: 'warning-500'}"
											>
												{selectedDevice.tracked ? 'Yes' : 'No'}
											</span>
										</dd>
									</div>
									<div>
										<dt class="text-sm font-medium text-gray-600">Identified</dt>
										<dd class="text-sm">
											<span
												class="badge preset-filled-{selectedDevice.identified
													? 'success-500'
													: 'warning-500'}"
											>
												{selectedDevice.identified ? 'Yes' : 'No'}
											</span>
										</dd>
									</div>
								</div>

								<div class="grid grid-cols-1 gap-4">
									<div>
										<dt class="text-sm font-medium text-gray-600">OGN Database</dt>
										<dd class="text-sm">
											<span
												class="badge {selectedDevice.from_ddb
													? 'preset-filled-success-500'
													: 'preset-filled-secondary-500'}"
											>
												{selectedDevice.from_ddb ? 'From OGN DB' : 'Not in OGN DB'}
											</span>
										</dd>
									</div>
								</div>

								{#if selectedDevice.aircraft_type_ogn}
									<div class="grid grid-cols-1 gap-4">
										<div>
											<dt class="text-sm font-medium text-gray-600">Aircraft Type</dt>
											<dd class="text-sm">
												<span
													class="badge {getAircraftTypeColor(
														selectedDevice.aircraft_type_ogn
													)} text-xs"
												>
													{getAircraftTypeOgnDescription(selectedDevice.aircraft_type_ogn)}
												</span>
											</dd>
										</div>
									</div>
								{/if}
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
										<dt class="font-medium text-gray-600">Transponder Code</dt>
										<dd class="font-mono">{aircraftRegistration.mode_s_code_hex || 'N/A'}</dd>
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
										<dd>
											{getStatusCodeDescription(aircraftRegistration.status_code)}
											<span class="ml-1 text-xs text-gray-500"
												>({aircraftRegistration.status_code})</span
											>
										</dd>
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
										<dd>{aircraftModel.manufacturer_name || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Model</dt>
										<dd>{aircraftModel.model_name || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Aircraft Type</dt>
										<dd>{formatTitleCase(aircraftModel.aircraft_type)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Engine Type</dt>
										<dd>{formatTitleCase(aircraftModel.engine_type)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Category</dt>
										<dd>{formatTitleCase(aircraftModel.aircraft_category)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Builder Certification</dt>
										<dd>{formatTitleCase(aircraftModel.builder_certification)}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Seats</dt>
										<dd>{aircraftModel.number_of_seats ?? 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Engines</dt>
										<dd>{aircraftModel.number_of_engines ?? 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Cruising Speed</dt>
										<dd>
											{aircraftModel.cruising_speed && aircraftModel.cruising_speed > 0
												? `${aircraftModel.cruising_speed} kts`
												: 'Unknown'}
										</dd>
									</div>
									<div>
										<dt class="font-medium text-gray-600">Weight Class</dt>
										<dd>{formatTitleCase(aircraftModel.weight_class)}</dd>
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
										<dd>{formatAltitude(latestFix.altitude_msl_feet)}</dd>
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
						{/if}
					</div>
				</div>

				<!-- Recent Fixes List - Full Width -->
				{#if recentFixes.length > 0}
					<div class="mt-6 space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Clock size={20} />
							Recent Fixes
							<span class="text-sm font-normal text-gray-600"> (Last 24 hours) </span>
						</h3>

						<div class="max-h-64 overflow-y-auto rounded-lg border border-gray-200">
							<table class="w-full text-sm">
								<thead class="border-b border-gray-200 bg-gray-50">
									<tr>
										<th class="px-3 py-2 text-left font-medium text-gray-600">Time</th>
										<th class="px-3 py-2 text-left font-medium text-gray-600">Altitude MSL</th>
										<th class="px-3 py-2 text-left font-medium text-gray-600">Altitude AGL</th>
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
												{formatAltitude(fix.altitude_msl_feet)}
											</td>
											<td class="px-3 py-2">
												{formatAltitude(fix.altitude_agl_feet)}
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
					</div>
				{/if}
			</div>
		</div>
	</div>
{/if}

<style>
	.direction-arrow {
		transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
		filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.3));
	}
</style>
