<script lang="ts">
	import {
		X,
		Plane,
		MapPin,
		RotateCcw,
		ExternalLink,
		Navigation,
		Star,
		FileText,
		Cog,
		PlaneTakeoff
	} from '@lucide/svelte';
	import type {
		Aircraft,
		Fix,
		AircraftRegistration,
		AircraftModel,
		Flight,
		DataResponse,
		DeviceOrientationEventWithCompass
	} from '$lib/types';
	import {
		formatTitleCase,
		formatPrimaryAddress,
		getAllAddresses,
		getStatusCodeDescription,
		getAircraftCategoryDescription,
		getAircraftCategoryColor,
		formatTransponderCode,
		getFlagPath
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
	import { auth } from '$lib/stores/auth';
	import { toaster } from '$lib/toaster';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'AircraftStatusModal']);

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

	// Direction arrow variables
	let userLocation: { lat: number; lng: number } | null = $state(null);
	let deviceHeading: number = $state(0); // Magnetic heading of the device/phone
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
			// Import serverCall
			const { serverCall } = await import('$lib/api/server');

			// Load aircraft registration and model data from API
			const [registrationResponse, modelResponse, flightResponse] = await Promise.all([
				serverCall<DataResponse<AircraftRegistration>>(
					`/aircraft/${selectedAircraft.id}/registration`
				).catch(() => null),
				serverCall<DataResponse<AircraftModel>>(`/aircraft/${selectedAircraft.id}/model`).catch(
					() => null
				),
				selectedAircraft.currentFix?.flightId
					? serverCall<DataResponse<Flight>>(
							`/flights/${selectedAircraft.currentFix.flightId}`
						).catch(() => null)
					: Promise.resolve(null)
			]);

			aircraftRegistration = registrationResponse?.data || null;
			aircraftModel = modelResponse?.data || null;
			currentFlight = flightResponse?.data || null;
		} catch (error) {
			logger.warn('Failed to load aircraft data: {error}', { error });
			aircraftRegistration = null;
			aircraftModel = null;
			currentFlight = null;
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

		if (!$auth.isAuthenticated) {
			toaster.warning({ title: 'Please log in to use watchlist' });
			return;
		}

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
			logger.error('Failed to toggle watchlist: {error}', { error });
		} finally {
			addingToWatchlist = false;
		}
	}

	function formatSpeed(speed_knots: number | undefined): string {
		if (speed_knots === undefined || speed_knots === null) return 'Unknown';
		return `${Math.round(speed_knots)} kts`;
	}

	function formatClimbRate(climbFpm: number | undefined): string {
		if (climbFpm === undefined || climbFpm === null) return 'Unknown';
		const sign = climbFpm >= 0 ? '+' : '';
		return `${sign}${Math.round(climbFpm)} fpm`;
	}

	function formatTrack(trackDegrees: number | undefined): string {
		if (trackDegrees === undefined || trackDegrees === null) return 'Unknown';
		return `${Math.round(trackDegrees)}°`;
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

	// Update direction to aircraft
	function updateDirectionToAircraft() {
		if (!userLocation || !selectedAircraft) {
			return;
		}

		const currentFix = selectedAircraft.currentFix as Fix | null;
		if (!currentFix) {
			return;
		}

		// Calculate bearing from user to aircraft (absolute bearing from north)
		const bearing = calculateBearing(
			userLocation.lat,
			userLocation.lng,
			currentFix.latitude,
			currentFix.longitude
		);

		// Calculate the arrow rotation to point at the aircraft
		// The arrow should always point toward the aircraft's actual location
		// bearing = absolute direction to aircraft (from north)
		// deviceHeading = direction phone is pointing (magnetic heading)
		//
		// Example: If aircraft is east (90°) and device points west (270°):
		//   - On screen: top=west, right=south, bottom=east, left=north
		//   - To point east (at aircraft), arrow must point down (180°)
		//   - Formula: 90° - 270° = -180° = 180° ✓
		//
		// The formula is: arrow rotation = bearing - deviceHeading
		let newDirection = bearing - deviceHeading;

		// Normalize to 0-360 range
		newDirection = ((newDirection % 360) + 360) % 360;

		// Calculate the shortest rotation path to avoid spinning around unnecessarily
		let delta = newDirection - previousDirectionToAircraft;

		// Adjust for boundary crossing to take the shortest path
		if (delta > 180) {
			directionToAircraft = newDirection - 360;
		} else if (delta < -180) {
			directionToAircraft = newDirection + 360;
		} else {
			directionToAircraft = newDirection;
		}

		// Save for the next comparison (use the normalized value)
		previousDirectionToAircraft = newDirection;
	}

	// Handle aircraft orientation changes
	function handleOrientationChange(event: DeviceOrientationEventWithCompass) {
		if (event.alpha !== null) {
			isCompassActive = true;

			// Get the magnetic heading from the device
			// iOS provides webkitCompassHeading which is the true magnetic heading
			const webkitHeading = event.webkitCompassHeading;

			if (webkitHeading !== undefined && webkitHeading !== null) {
				// iOS: Use webkitCompassHeading directly (already magnetic heading)
				deviceHeading = webkitHeading;
			} else if (event.absolute && event.alpha !== null) {
				// Android with absolute orientation: Convert alpha to magnetic heading
				// alpha is counter-clockwise from north, compass is clockwise from north
				deviceHeading = (360 - event.alpha) % 360;
			} else {
				// Fallback: Use alpha as-is (may not be accurate, default to 0 if somehow null)
				logger.warn(
					'Using raw alpha for heading (absolute={absolute}), compass may be inaccurate',
					{ absolute: event.absolute }
				);
				deviceHeading = event.alpha ?? 0;
			}

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
			logger.warn('Failed to get user location: {error}', { error });
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
					logger.warn('Device orientation permission denied: {error}', { error });
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
			<!-- Sticky Header -->
			<div
				class="sticky top-0 z-10 flex items-center justify-between border-b border-surface-300 bg-surface-50 p-6 dark:border-surface-600 dark:bg-surface-900"
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
							{selectedAircraft.registration || formatPrimaryAddress(selectedAircraft)}
							{#if selectedAircraft.aircraftModel}
								• {selectedAircraft.aircraftModel}
							{/if}
						</p>
					</div>
				</div>
				<div class="flex items-center gap-2">
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
					<!-- 1. Aircraft Information -->
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
									{#each getAllAddresses(selectedAircraft) as addr (addr.label)}
										<div>
											<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
												{addr.label} Address
											</dt>
											<dd class="font-mono text-sm">
												{addr.hex}
											</dd>
										</div>
									{/each}
								</div>

								<div class="grid grid-cols-2 gap-4">
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Aircraft Model
										</dt>
										<dd class="text-sm">
											{selectedAircraft.aircraftModel || 'Unknown'}
											{#if selectedAircraft.icaoModelCode}
												<span class="ml-1 text-xs text-surface-500"
													>({selectedAircraft.icaoModelCode})</span
												>
											{/if}
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

								{#if selectedAircraft.ownerOperator}
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Owner / Operator
										</dt>
										<dd class="text-sm">
											{selectedAircraft.ownerOperator}
										</dd>
									</div>
								{/if}

								{#if selectedAircraft.homeBaseAirportIdent}
									<div>
										<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
											Home Base Airport
										</dt>
										<dd class="text-sm">
											{selectedAircraft.homeBaseAirportIdent}
										</dd>
									</div>
								{/if}

								<div class="grid grid-cols-2 gap-4">
									<div>
										<dd class="text-sm">
											<span
												class="badge {selectedAircraft.fromOgnDdb
													? 'preset-filled-success-500'
													: 'preset-filled-secondary-500'}"
											>
												{selectedAircraft.fromOgnDdb ? 'In Unified DDB' : 'Not in Unified DDB'}
											</span>
										</dd>
									</div>
									<div>
										<dd class="text-sm">
											<span
												class="badge {selectedAircraft.fromAdsbxDdb
													? 'preset-filled-success-500'
													: 'preset-filled-secondary-500'}"
											>
												{selectedAircraft.fromAdsbxDdb ? 'In ADSBX DB' : 'Not in ADSBX DB'}
											</span>
										</dd>
									</div>
								</div>

								{#if selectedAircraft.aircraftCategory}
									<div class="grid grid-cols-1 gap-4">
										<div>
											<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
												Aircraft Category
											</dt>
											<dd class="text-sm">
												<span
													class="badge {getAircraftCategoryColor(
														selectedAircraft.aircraftCategory
													)} text-xs"
												>
													{getAircraftCategoryDescription(selectedAircraft.aircraftCategory)}
												</span>
											</dd>
										</div>
									</div>
								{/if}
							</div>
						</div>
					</div>

					<!-- 2. Current Position -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Navigation size={20} />
							Current Position
						</h3>

						{#if !selectedAircraft.currentFix}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="py-8 text-center text-surface-500 dark:text-surface-500">
									<MapPin size={48} class="mx-auto mb-2 opacity-50" />
									<p>No recent position data available</p>
								</div>
							</div>
						{:else}
							{@const positionFix = selectedAircraft.currentFix as Fix}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Altitude (ft)
										</dt>
										<dd>
											{#if positionFix.altitudeMslFeet !== undefined && positionFix.altitudeMslFeet !== null}
												{positionFix.altitudeMslFeet.toLocaleString()} MSL
												{#if positionFix.altitudeAglFeet !== undefined && positionFix.altitudeAglFeet !== null}
													/ {positionFix.altitudeAglFeet.toLocaleString()} AGL
												{/if}
											{:else}
												Unknown
											{/if}
										</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Ground Speed</dt>
										<dd>{formatSpeed(positionFix.groundSpeedKnots ?? undefined)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Track</dt>
										<dd>{formatTrack(positionFix.trackDegrees ?? undefined)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Climb Rate</dt>
										<dd>
											<span
												class="{positionFix.climbFpm !== undefined &&
												positionFix.climbFpm !== null &&
												positionFix.climbFpm < 0
													? 'text-red-600 dark:text-red-400'
													: 'text-green-600 dark:text-green-400'} font-semibold"
											>
												{formatClimbRate(positionFix.climbFpm ?? undefined)}
											</span>
										</dd>
									</div>

									<!-- Bearing and Distance to Aircraft -->
									{#if userLocation}
										{@const distanceNm = calculateDistance(
											userLocation.lat,
											userLocation.lng,
											positionFix.latitude,
											positionFix.longitude,
											'nm'
										)}
										{@const bearing = calculateBearing(
											userLocation.lat,
											userLocation.lng,
											positionFix.latitude,
											positionFix.longitude
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
											{formatCoordinates(positionFix.latitude, positionFix.longitude)}
										</dd>
									</div>
									<div class="col-span-2">
										<dd>
											Last seen {formatTimestamp(positionFix.receivedAt).relative}
											<div class="text-xs text-surface-500 dark:text-surface-500">
												{formatTimestamp(positionFix.receivedAt).absolute}
											</div>
										</dd>
									</div>
								</dl>
							</div>
						{/if}
					</div>

					<!-- 3. Current Flight -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<PlaneTakeoff size={20} />
							Current Flight
						</h3>

						{#if loadingFlight}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400">
									<RotateCcw class="animate-spin" size={16} />
									Loading flight information...
								</div>
							</div>
						{:else if currentFlight}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="mb-4 flex items-center justify-between">
									<span
										class="badge preset-filled-{currentFlight.state === 'active'
											? 'success-500'
											: currentFlight.state === 'complete'
												? 'primary-500'
												: 'warning-500'} text-sm"
									>
										{currentFlight.state === 'active'
											? 'In Flight'
											: currentFlight.state === 'complete'
												? 'Completed'
												: 'Timed Out'}
									</span>
									<a
										href="/flights/{currentFlight.id}"
										target="_blank"
										rel="noopener noreferrer"
										class="btn preset-filled-success-500 btn-sm"
										title="View full flight details"
									>
										<Plane size={14} />
										View Flight
										<ExternalLink size={12} />
									</a>
								</div>

								<dl class="grid grid-cols-2 gap-4 text-sm">
									<!-- Callsign -->
									{#if currentFlight.callsign}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Callsign</dt>
											<dd class="font-mono font-semibold">{currentFlight.callsign}</dd>
										</div>
									{/if}

									<!-- Detection Method -->
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Detection</dt>
										<dd>
											{#if currentFlight.takeoffTime}
												<span class="badge preset-filled-success-500 text-xs">From Takeoff</span>
											{:else}
												<span class="badge preset-filled-warning-500 text-xs"
													>Detected Airborne</span
												>
											{/if}
										</dd>
									</div>

									<!-- Start Location with Flag -->
									{#if currentFlight.startLocationCity || currentFlight.startLocationCountry || currentFlight.departureAirport}
										<div class="col-span-2">
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Start Location
											</dt>
											<dd class="flex items-center gap-2">
												{#if currentFlight.startLocationCountry}
													{@const flagPath = getFlagPath(currentFlight.startLocationCountry)}
													{#if flagPath}
														<img
															src={flagPath}
															alt={currentFlight.startLocationCountry}
															class="h-4 w-6 rounded-sm object-cover"
														/>
													{/if}
												{:else if currentFlight.departureAirportCountry}
													{@const flagPath = getFlagPath(currentFlight.departureAirportCountry)}
													{#if flagPath}
														<img
															src={flagPath}
															alt={currentFlight.departureAirportCountry}
															class="h-4 w-6 rounded-sm object-cover"
														/>
													{/if}
												{/if}
												<span>
													{#if currentFlight.departureAirport}
														<span class="font-semibold">{currentFlight.departureAirport}</span>
														{#if currentFlight.startLocationCity}
															<span class="text-surface-500">
																- {currentFlight.startLocationCity}{#if currentFlight.startLocationState},
																	{currentFlight.startLocationState}{/if}
															</span>
														{/if}
													{:else if currentFlight.startLocationCity}
														{currentFlight.startLocationCity}{#if currentFlight.startLocationState}, {currentFlight.startLocationState}{/if}{#if currentFlight.startLocationCountry},
															{currentFlight.startLocationCountry}{/if}
													{:else if currentFlight.startLocationCountry}
														{currentFlight.startLocationCountry}
													{/if}
												</span>
											</dd>
										</div>
									{/if}

									<!-- Arrival Airport -->
									{#if currentFlight.arrivalAirport}
										<div class="col-span-2">
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Destination
											</dt>
											<dd class="flex items-center gap-2">
												{#if currentFlight.arrivalAirportCountry}
													{@const flagPath = getFlagPath(currentFlight.arrivalAirportCountry)}
													{#if flagPath}
														<img
															src={flagPath}
															alt={currentFlight.arrivalAirportCountry}
															class="h-4 w-6 rounded-sm object-cover"
														/>
													{/if}
												{/if}
												<span class="font-semibold">{currentFlight.arrivalAirport}</span>
												{#if currentFlight.endLocationCity}
													<span class="text-surface-500">
														- {currentFlight.endLocationCity}{#if currentFlight.endLocationState}, {currentFlight.endLocationState}{/if}
													</span>
												{/if}
											</dd>
										</div>
									{/if}

									<!-- Takeoff/Detection Time -->
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											{currentFlight.takeoffTime ? 'Takeoff' : 'First Seen'}
										</dt>
										<dd>
											{formatTimestamp(currentFlight.takeoffTime || currentFlight.createdAt)
												.relative}
											<div class="text-xs text-surface-500">
												{formatTimestamp(currentFlight.takeoffTime || currentFlight.createdAt)
													.absolute}
											</div>
										</dd>
									</div>

									<!-- Landing Time -->
									{#if currentFlight.landingTime}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Landing</dt>
											<dd>
												{formatTimestamp(currentFlight.landingTime).relative}
												<div class="text-xs text-surface-500">
													{formatTimestamp(currentFlight.landingTime).absolute}
												</div>
											</dd>
										</div>
									{/if}

									<!-- Takeoff Runway -->
									{#if currentFlight.takeoffRunwayIdent}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Takeoff Runway
											</dt>
											<dd class="font-semibold">
												RWY {currentFlight.takeoffRunwayIdent}
												{#if currentFlight.runwaysInferred}
													<span class="text-xs text-surface-500">(inferred)</span>
												{/if}
											</dd>
										</div>
									{/if}

									<!-- Landing Runway -->
									{#if currentFlight.landingRunwayIdent}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Landing Runway
											</dt>
											<dd class="font-semibold">
												RWY {currentFlight.landingRunwayIdent}
												{#if currentFlight.runwaysInferred}
													<span class="text-xs text-surface-500">(inferred)</span>
												{/if}
											</dd>
										</div>
									{/if}

									<!-- Flight Duration -->
									{#if currentFlight.durationSeconds}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Duration</dt>
											<dd class="font-semibold">
												{Math.floor(currentFlight.durationSeconds / 3600)}h
												{Math.floor((currentFlight.durationSeconds % 3600) / 60)}m
											</dd>
										</div>
									{/if}

									<!-- Current Altitude -->
									{#if currentFlight.latestAltitudeMslFeet}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Current Altitude
											</dt>
											<dd class="font-semibold">
												{currentFlight.latestAltitudeMslFeet.toLocaleString()} ft MSL
												{#if currentFlight.latestAltitudeAglFeet}
													<span class="text-surface-500">
														/ {currentFlight.latestAltitudeAglFeet.toLocaleString()} AGL
													</span>
												{/if}
											</dd>
										</div>
									{/if}

									<!-- Distance Traveled -->
									{#if currentFlight.totalDistanceMeters}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Distance</dt>
											<dd class="font-semibold">
												{(currentFlight.totalDistanceMeters / 1852).toFixed(1)} nm
											</dd>
										</div>
									{/if}

									<!-- Maximum Displacement -->
									{#if currentFlight.maximumDisplacementMeters}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Max Displacement
											</dt>
											<dd class="font-semibold">
												{(currentFlight.maximumDisplacementMeters / 1852).toFixed(1)} nm
											</dd>
										</div>
									{/if}
								</dl>
							</div>
						{:else}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="py-8 text-center text-surface-500 dark:text-surface-500">
									<PlaneTakeoff size={48} class="mx-auto mb-2 opacity-50" />
									<p>No current flight data available</p>
								</div>
							</div>
						{/if}
					</div>

					<!-- 4. FAA Registration Details -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<FileText size={20} />
							FAA Registration Details
						</h3>

						{#if loadingRegistration}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400">
									<RotateCcw class="animate-spin" size={16} />
									Loading aircraft registration...
								</div>
							</div>
						{:else if aircraftRegistration}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Owner</dt>
										<dd>{aircraftRegistration.registrantName}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Serial Number
										</dt>
										<dd class="font-mono">{aircraftRegistration.serialNumber}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Transponder Code
										</dt>
										<dd class="font-mono">
											{formatTransponderCode(aircraftRegistration.transponderCode)}
										</dd>
									</div>
									{#if aircraftRegistration.yearManufactured}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Year</dt>
											<dd>{aircraftRegistration.yearManufactured}</dd>
										</div>
									{/if}
									{#if aircraftRegistration.aircraftType}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Type</dt>
											<dd>{aircraftRegistration.aircraftType}</dd>
										</div>
									{/if}
									{#if aircraftRegistration.statusCode}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Status</dt>
											<dd>
												{getStatusCodeDescription(aircraftRegistration.statusCode)}
												<span class="ml-1 text-xs text-surface-500 dark:text-surface-500"
													>({aircraftRegistration.statusCode})</span
												>
											</dd>
										</div>
									{/if}
									{#if aircraftRegistration.certificateIssueDate}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Certificate Issue
											</dt>
											<dd>
												{dayjs(aircraftRegistration.certificateIssueDate).format('YYYY-MM-DD')}
											</dd>
										</div>
									{/if}
									{#if aircraftRegistration.expirationDate}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">Expiration</dt>
											<dd>{dayjs(aircraftRegistration.expirationDate).format('YYYY-MM-DD')}</dd>
										</div>
									{/if}
									{#if aircraftRegistration.airworthinessDate}
										<div>
											<dt class="font-medium text-surface-600 dark:text-surface-400">
												Airworthiness
											</dt>
											<dd>{dayjs(aircraftRegistration.airworthinessDate).format('YYYY-MM-DD')}</dd>
										</div>
									{/if}
								</dl>
							</div>
						{:else}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="py-8 text-center text-surface-500 dark:text-surface-500">
									<FileText size={48} class="mx-auto mb-2 opacity-50" />
									<p>No FAA registration data available</p>
								</div>
							</div>
						{/if}
					</div>

					<!-- 5. Aircraft Model Details -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Cog size={20} />
							Aircraft Model Details
						</h3>

						{#if loadingModel}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400">
									<RotateCcw class="animate-spin" size={16} />
									Loading aircraft model details...
								</div>
							</div>
						{:else if aircraftModel}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<dl class="grid grid-cols-2 gap-4 text-sm">
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Manufacturer</dt>
										<dd>{aircraftModel.manufacturerName || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Model</dt>
										<dd>{aircraftModel.modelName || 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Aircraft Type
										</dt>
										<dd>{formatTitleCase(aircraftModel.aircraftType)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Engine Type</dt>
										<dd>{formatTitleCase(aircraftModel.engineType)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Category</dt>
										<dd>{formatTitleCase(aircraftModel.aircraftCategory)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Builder Certification
										</dt>
										<dd>{formatTitleCase(aircraftModel.builderCertification)}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Seats</dt>
										<dd>{aircraftModel.numberOfSeats ?? 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Engines</dt>
										<dd>{aircraftModel.numberOfEngines ?? 'Unknown'}</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">
											Cruising Speed
										</dt>
										<dd>
											{aircraftModel.cruisingSpeed && aircraftModel.cruisingSpeed > 0
												? `${aircraftModel.cruisingSpeed} kts`
												: 'Unknown'}
										</dd>
									</div>
									<div>
										<dt class="font-medium text-surface-600 dark:text-surface-400">Weight Class</dt>
										<dd>{formatTitleCase(aircraftModel.weightClass)}</dd>
									</div>
								</dl>
							</div>
						{:else}
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="py-8 text-center text-surface-500 dark:text-surface-500">
									<Cog size={48} class="mx-auto mb-2 opacity-50" />
									<p>No aircraft model data available</p>
								</div>
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
