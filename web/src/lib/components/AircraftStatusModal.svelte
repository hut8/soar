<script lang="ts">
	import { X, Plane, MapPin, RotateCcw, ExternalLink, Navigation, Star } from '@lucide/svelte';
	import type { Aircraft, Fix, AircraftRegistration, AircraftModel, Flight } from '$lib/types';
	import {
		formatTitleCase,
		formatAircraftAddress,
		getStatusCodeDescription,
		getAircraftTypeOgnDescription,
		getAircraftTypeColor,
		formatTransponderCode
	} from '$lib/formatters';
	import {
		calculateDistance,
		calculateBearing,
		formatDistance,
		formatBearing
	} from '$lib/geography';
	import dayjs from 'dayjs';
	import relativeTime from 'dayjs/plugin/relativeTime';
	import { onMount } from 'svelte';
	import { watchlist } from '$lib/stores/watchlist';

	// Extend dayjs with relative time plugin
	dayjs.extend(relativeTime);

	// Props
	let { showModal = $bindable(), selectedAircraft = $bindable() } = $props<{
		showModal: boolean;
		selectedAircraft: Aircraft | null;
	}>();

	// Reactive variables
	let aircraftRegistration: AircraftRegistration | null = $state(null);
	let aircraftModel: AircraftModel | null = $state(null);
	let currentFlight: Flight | null = $state(null);
	let loadingRegistration = $state(false);
	let loadingModel = $state(false);
	let loadingFlight = $state(false);
	let recentFixes: Fix[] = $state([]);

	// Direction arrow variables
	let userLocation: { lat: number; lng: number } | null = $state(null);
	let aircraftHeading: number = $state(0);
	let isCompassActive: boolean = $state(false);
	let directionToAircraft: number = $state(0);
	let previousDirectionToAircraft: number = $state(0);
	let locationPermissionDenied: boolean = $state(false);

	// Watchlist state
	let isInWatchlist: boolean = $state(false);
	let addingToWatchlist: boolean = $state(false);

	// Update data when device changes
	$effect(() => {
		if (selectedAircraft) {
			loadAircraftData();
			// Check if aircraft is in watchlist
			isInWatchlist = watchlist.has(selectedAircraft.id);
		} else {
			aircraftRegistration = null;
			aircraftModel = null;
			currentFlight = null;
			recentFixes = [];
			isInWatchlist = false;
		}
	});

	async function loadAircraftData() {
		if (!selectedAircraft) return;

		// Load aircraft registration and model data in parallel
		loadingRegistration = true;
		loadingModel = true;
		loadingFlight = true;

		try {
			// Import AircraftRegistry and serverCall
			const { AircraftRegistry } = await import('$lib/services/AircraftRegistry');
			const { serverCall } = await import('$lib/api/server');
			const registry = AircraftRegistry.getInstance();

			// Load recent fixes from API
			await registry.loadRecentFixesFromAPI(selectedAircraft.id, 100);

			// Update recent fixes from the aircraft (last 24 hours)
			const now = Date.now();
			const twentyFourHoursAgo = now - 24 * 60 * 60 * 1000;
			const allFixes = selectedAircraft.fixes || [];
			recentFixes = allFixes.filter((fix: Fix) => {
				const fixTime = new Date(fix.timestamp).getTime();
				return fixTime > twentyFourHoursAgo;
			});

			// Check if aircraft has registration/model data
			aircraftRegistration = selectedAircraft.aircraftRegistration || null;
			aircraftModel = selectedAircraft.aircraftModelDetails || null;

			// Load current flight - use currentFix.flight if available, otherwise fetch
			if (selectedAircraft.currentFix?.flight) {
				currentFlight = selectedAircraft.currentFix.flight;
			} else if (selectedAircraft.activeFlightId) {
				try {
					currentFlight = await serverCall(`/flights/${selectedAircraft.activeFlightId}`);
				} catch (error) {
					console.warn('Failed to load flight data:', error);
					currentFlight = null;
				}
			} else {
				currentFlight = null;
			}
		} catch (error) {
			console.warn('Failed to load aircraft data:', error);
			aircraftRegistration = null;
			aircraftModel = null;
			currentFlight = null;
			recentFixes = [];
		} finally {
			loadingRegistration = false;
			loadingModel = false;
			loadingFlight = false;
		}
	}

	function closeModal() {
		showModal = false;
		selectedAircraft = null;
	}

	async function toggleWatchlist() {
		if (!selectedAircraft) return;

		addingToWatchlist = true;
		try {
			if (isInWatchlist) {
				await watchlist.remove(selectedAircraft.id);
				isInWatchlist = false;
			} else {
				await watchlist.add(selectedAircraft.id, false);
				isInWatchlist = true;
			}
		} catch (error) {
			console.error('Failed to toggle watchlist:', error);
		} finally {
			addingToWatchlist = false;
		}
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

	// Update direction to aircraft
	function updateDirectionToAircraft() {
		if (!userLocation || !selectedAircraft) {
			return;
		}

		const fixes = selectedAircraft.fixes || [];
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
		// Add aircraft heading to rotate arrow opposite to phone rotation
		// When phone points north (aircraftHeading = 0), arrow shows absolute bearing
		// When phone rotates clockwise, arrow rotates counter-clockwise to keep pointing at aircraft
		let newDirection = (bearing + aircraftHeading) % 360;

		// Normalize to 0-360 range
		newDirection = ((newDirection % 360) + 360) % 360;

		// Calculate the shortest rotation path to avoid spinning around unnecessarily
		// If the difference is greater than 180°, we should wrap around
		let delta = newDirection - previousDirectionToAircraft;

		// Adjust for boundary crossing to take the shortest path
		if (delta > 180) {
			// Crossed from high to low (e.g., 350° to 10°)
			// Add a full rotation to previousDirectionToAircraft conceptually
			directionToAircraft = newDirection - 360;
		} else if (delta < -180) {
			// Crossed from low to high (e.g., 10° to 350°)
			// Subtract a full rotation from newDirection conceptually
			directionToAircraft = newDirection + 360;
		} else {
			// No boundary crossing, use the new direction directly
			directionToAircraft = newDirection;
		}

		// Save the normalized direction for the next comparison
		previousDirectionToAircraft = newDirection;
	}

	// Handle aircraft orientation changes
	function handleOrientationChange(event: DeviceOrientationEvent) {
		if (event.alpha !== null) {
			isCompassActive = true;
			aircraftHeading = event.alpha;
			updateDirectionToAircraft();
		}
	}

	// Get user location
	async function getUserLocation() {
		if (!navigator.geolocation) {
			locationPermissionDenied = true;
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
			locationPermissionDenied = false;

			updateDirectionToAircraft();
		} catch (error) {
			console.warn('Failed to get user location:', error);
			locationPermissionDenied = true;
		}
	}

	// Request location permission
	function requestLocationPermission() {
		void getUserLocation();
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

	// Update direction when aircraft changes
	$effect(() => {
		if (selectedAircraft) {
			updateDirectionToAircraft();
		}
	});
</script>

<!-- Aircraft Status Modal -->
{#if showModal && selectedAircraft}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-surface-950-50/50 pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		role="presentation"
	>
		<div
			class="relative max-h-[calc(90vh-5rem)] w-full max-w-4xl overflow-y-auto card bg-surface-50 text-surface-900 shadow-xl dark:bg-surface-900 dark:text-surface-50"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="aircraft-status-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between border-b border-surface-300 p-6 dark:border-surface-600"
			>
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
							<div class="mt-1 text-xs font-semibold text-surface-700 dark:text-surface-300">
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
						<h2 id="aircraft-status-title" class="text-xl font-bold">Aircraft Status</h2>
						<p class="text-sm text-surface-600 dark:text-surface-400">
							{selectedAircraft.registration ||
								formatAircraftAddress(selectedAircraft.addressType, selectedAircraft.address)}
							{#if selectedAircraft.aircraftModel}
								• {selectedAircraft.aircraftModel}
							{/if}
						</p>
					</div>
				</div>
				<div class="flex items-center gap-2">
					{#if selectedAircraft.activeFlightId}
						<a
							href="/flights/{selectedAircraft.activeFlightId}"
							target="_blank"
							rel="noopener noreferrer"
							class="btn preset-filled-success-500 btn-sm"
							title="View current flight"
						>
							<Plane size={16} />
							Current Flight
						</a>
					{/if}
					<button
						class="btn btn-sm {isInWatchlist
							? 'preset-filled-warning-500'
							: 'preset-tonal-surface-500'}"
						onclick={toggleWatchlist}
						disabled={addingToWatchlist}
						title={isInWatchlist ? 'Remove from watchlist' : 'Add to watchlist'}
					>
						<Star size={16} class={isInWatchlist ? 'fill-current' : ''} />
						{isInWatchlist ? 'Remove' : 'Watch'}
					</button>
					<a
						href="/aircraft/{selectedAircraft.id}"
						target="_blank"
						rel="noopener noreferrer"
						class="btn preset-filled-primary-500 btn-sm"
						title="View detailed aircraft page"
					>
						<ExternalLink size={16} />
						Details
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

						<div
							class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
						>
							<div class="space-y-3">
								<div class="grid grid-cols-2 gap-4">
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Registration
										</dt>
										<dd class="font-mono text-sm">
											{selectedAircraft.registration || 'Unknown'}
										</dd>
									</div>
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Address
										</dt>
										<dd class="font-mono text-sm">
											{formatAircraftAddress(
												selectedAircraft.addressType,
												selectedAircraft.address
											)}
										</dd>
									</div>
								</div>

								<div class="grid grid-cols-2 gap-4">
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Aircraft Model
										</dt>
										<dd class="text-sm">
											{selectedAircraft.aircraftModel || 'Unknown'}
										</dd>
									</div>
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Competition Number
										</dt>
										<dd class="text-sm">
											{selectedAircraft.competitionNumber || 'None'}
										</dd>
									</div>
								</div>

								<div class="grid grid-cols-3 gap-4">
									<div>
										<dd class="text-sm">
											<span
												class="badge preset-filled-{selectedAircraft.tracked
													? 'success-500'
													: 'warning-500'}"
											>
												{selectedAircraft.tracked ? 'Tracked' : 'Not tracked'}
											</span>
										</dd>
									</div>
									<div>
										<dd class="text-sm">
											<span
												class="badge preset-filled-{selectedAircraft.identified
													? 'success-500'
													: 'warning-500'}"
											>
												{selectedAircraft.identified ? 'Identified' : 'Not identified'}
											</span>
										</dd>
									</div>
									<div>
										<dd class="text-sm">
											<span
												class="badge {selectedAircraft.fromOgnDdb
													? 'preset-filled-success-500'
													: 'preset-filled-secondary-500'}"
											>
												{selectedAircraft.fromOgnDdb ? 'From OGN DB' : 'Not in OGN DB'}
											</span>
										</dd>
									</div>
								</div>

								{#if selectedAircraft.aircraftTypeOgn}
									<div class="grid grid-cols-1 gap-4">
										<div>
											<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
												Aircraft Type
											</dt>
											<dd class="text-sm">
												<span
													class="badge {getAircraftTypeColor(
														selectedAircraft.aircraftTypeOgn
													)} text-xs"
												>
													{getAircraftTypeOgnDescription(selectedAircraft.aircraftTypeOgn)}
												</span>
											</dd>
										</div>
									</div>
								{/if}
							</div>
						</div>

						<!-- Aircraft Registration Details -->
						{#if loadingRegistration}
							<div class="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400">
								<RotateCcw class="animate-spin" size={16} />
								Loading aircraft registration...
							</div>
						{:else if aircraftRegistration}
							<div class="space-y-3 border-t border-surface-300 pt-4 dark:border-surface-600">
								<h4 class="font-medium text-surface-900 dark:text-surface-100">
									FAA Registration Details
								</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Owner</dt>
										<dd>{aircraftRegistration.registrant_name}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Serial Number
										</dt>
										<dd class="font-mono">{aircraftRegistration.serial_number}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Transponder Code
										</dt>
										<dd class="font-mono">
											{formatTransponderCode(aircraftRegistration.transponder_code)}
										</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Year</dt>
										<dd>{aircraftRegistration.year_mfr}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Type</dt>
										<dd>{getAircraftTypeDescription(aircraftRegistration.type_aircraft)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Status</dt>
										<dd>
											{getStatusCodeDescription(aircraftRegistration.status_code)}
											<span class="ml-1 text-xs text-surface-500 dark:text-surface-500"
												>({aircraftRegistration.status_code})</span
											>
										</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Certificate Issue
										</dt>
										<dd>{dayjs(aircraftRegistration.cert_issue_date).format('YYYY-MM-DD')}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Expiration</dt>
										<dd>{dayjs(aircraftRegistration.expiration_date).format('YYYY-MM-DD')}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Airworthiness
										</dt>
										<dd>{dayjs(aircraftRegistration.air_worth_date).format('YYYY-MM-DD')}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Last Action</dt>
										<dd>{dayjs(aircraftRegistration.last_action_date).format('YYYY-MM-DD')}</dd>
									</div>
								</dl>
							</div>
						{/if}

						<!-- Aircraft Model Details -->
						{#if loadingModel}
							<div class="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400">
								<RotateCcw class="animate-spin" size={16} />
								Loading aircraft model details...
							</div>
						{:else if aircraftModel}
							<div class="space-y-3 border-t border-surface-300 pt-4 dark:border-surface-600">
								<h4 class="font-medium text-surface-900 dark:text-surface-100">
									Aircraft Model Details
								</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Manufacturer</dt>
										<dd>{aircraftModel.manufacturer_name || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Model</dt>
										<dd>{aircraftModel.model_name || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Aircraft Type
										</dt>
										<dd>{formatTitleCase(aircraftModel.aircraft_type)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Engine Type</dt>
										<dd>{formatTitleCase(aircraftModel.engine_type)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Category</dt>
										<dd>{formatTitleCase(aircraftModel.aircraft_category)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Builder Certification
										</dt>
										<dd>{formatTitleCase(aircraftModel.builder_certification)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Seats</dt>
										<dd>{aircraftModel.number_of_seats ?? 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Engines</dt>
										<dd>{aircraftModel.number_of_engines ?? 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Cruising Speed
										</dt>
										<dd>
											{aircraftModel.cruising_speed && aircraftModel.cruising_speed > 0
												? `${aircraftModel.cruising_speed} kts`
												: 'Unknown'}
										</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Weight Class</dt>
										<dd>{formatTitleCase(aircraftModel.weight_class)}</dd>
									</div>
								</dl>
							</div>
						{/if}

						<!-- Current Flight -->
						{#if loadingFlight}
							<div class="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400">
								<RotateCcw class="animate-spin" size={16} />
								Loading flight information...
							</div>
						{:else if currentFlight}
							<div class="space-y-3 border-t border-surface-300 pt-4 dark:border-surface-600">
								<h4 class="font-medium text-surface-900 dark:text-surface-100">Current Flight</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									{#if currentFlight.departure_airport}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Departure</dt>
											<dd>
												{currentFlight.departure_airport}
												{#if currentFlight.takeoff_time}
													<div class="text-xs text-surface-500">
														{dayjs(currentFlight.takeoff_time).format('HH:mm')}
													</div>
												{/if}
											</dd>
										</div>
									{/if}
									{#if currentFlight.arrival_airport}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Arrival</dt>
											<dd>
												{currentFlight.arrival_airport}
												{#if currentFlight.landing_time}
													<div class="text-xs text-surface-500">
														{dayjs(currentFlight.landing_time).format('HH:mm')}
													</div>
												{/if}
											</dd>
										</div>
									{/if}
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">State</dt>
										<dd>
											<span class="badge preset-filled-primary-500">
												{currentFlight.state}
											</span>
										</dd>
									</div>
									{#if currentFlight.duration_seconds}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Duration</dt>
											<dd>
												{Math.floor(currentFlight.duration_seconds / 3600)}h
												{Math.floor((currentFlight.duration_seconds % 3600) / 60)}m
											</dd>
										</div>
									{/if}
									{#if currentFlight.latest_altitude_msl_feet}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Current Altitude
											</dt>
											<dd>{currentFlight.latest_altitude_msl_feet.toLocaleString()} ft MSL</dd>
										</div>
									{/if}
									<div class="col-span-2">
										<a
											href="/flights/{currentFlight.id}"
											target="_blank"
											rel="noopener noreferrer"
											class="btn w-full preset-filled-primary-500 btn-sm"
										>
											<ExternalLink size={14} />
											View Full Flight Details
										</a>
									</div>
								</dl>
							</div>
						{/if}
					</div>

					<!-- Current Fix -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Navigation size={20} />
							Current Fix
						</h3>

						{#if recentFixes.length === 0}
							<div class="py-8 text-center text-surface-500 dark:text-surface-500">
								<MapPin size={48} class="mx-auto mb-2 opacity-50" />
								<p>No recent position data available</p>
							</div>
						{:else}
							<!-- Latest Fix Summary -->
							{@const latestFix = recentFixes[0]}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<h4 class="mb-3 font-medium text-surface-900 dark:text-surface-100">
									Latest Position
								</h4>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Altitude (ft)
										</dt>
										<dd>
											{#if latestFix.altitude_msl_feet !== undefined && latestFix.altitude_msl_feet !== null}
												{latestFix.altitude_msl_feet.toLocaleString()} MSL
												{#if latestFix.altitude_agl_feet !== undefined && latestFix.altitude_agl_feet !== null}
													/ {latestFix.altitude_agl_feet.toLocaleString()} AGL
												{/if}
											{:else}
												Unknown
											{/if}
										</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Ground Speed</dt>
										<dd>{formatSpeed(latestFix.ground_speed_knots)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Track</dt>
										<dd>{formatTrack(latestFix.track_degrees)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Climb Rate</dt>
										<dd>
											<span
												class="{latestFix.climb_fpm !== undefined &&
												latestFix.climb_fpm !== null &&
												latestFix.climb_fpm < 0
													? 'text-red-600 dark:text-red-400'
													: 'text-green-600 dark:text-green-400'} font-semibold"
											>
												{formatClimbRate(latestFix.climb_fpm)}
											</span>
										</dd>
									</div>

									<!-- Bearing and Distance to Aircraft -->
									{#if userLocation}
										{@const distanceNm = calculateDistance(
											userLocation.lat,
											userLocation.lng,
											latestFix.latitude,
											latestFix.longitude,
											'nm'
										)}
										{@const bearing = calculateBearing(
											userLocation.lat,
											userLocation.lng,
											latestFix.latitude,
											latestFix.longitude
										)}
										{@const distances = formatDistance(distanceNm)}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Distance from me
											</dt>
											<dd>{distances.nm} nm / {distances.mi} mi</dd>
										</div>
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Bearing</dt>
											<dd>{formatBearing(bearing)}</dd>
										</div>
									{:else}
										<div class="col-span-2">
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Distance & Bearing
											</dt>
											<dd>
												<button
													onclick={requestLocationPermission}
													class="btn preset-filled-primary-500 btn-sm"
													disabled={!locationPermissionDenied && userLocation === null}
												>
													<Navigation size={14} />
													Permit Location Access
												</button>
											</dd>
										</div>
									{/if}

									<div class="col-span-2">
										<dt class="font-medium text-surface-600 dark:text-surface-400">Coordinates</dt>
										<dd class="font-mono">
											{formatCoordinates(latestFix.latitude, latestFix.longitude)}
										</dd>
									</div>
									<div class="col-span-2">
										<dd>
											Last seen {formatTimestamp(latestFix.timestamp).relative}
											<div class="text-xs text-surface-500 dark:text-surface-500">
												{formatTimestamp(latestFix.timestamp).absolute}
											</div>
										</dd>
									</div>
								</dl>
							</div>
						{/if}
					</div>
				</div>
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
