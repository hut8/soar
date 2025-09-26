<script lang="ts">
	/// <reference types="@types/google.maps" />
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	// Placeholder for Google Maps API key - to be added later
	const GOOGLE_MAPS_API_KEY = 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

	let mapContainer: HTMLElement;
	let map: google.maps.Map;
	let userLocationButton: HTMLButtonElement;
	let isLocating = false;
	let userMarker: google.maps.marker.AdvancedMarkerElement | null = null;

	// Compass rose variables
	let deviceHeading: number = 0;
	let compassHeading: number = 0;
	let isCompassActive: boolean = false;
	let displayHeading: number = 0;

	// Screen orientation lock
	let screenOrientation:
		| 'portrait-primary'
		| 'landscape-primary'
		| 'portrait-secondary'
		| 'landscape-secondary'
		| null = null;

	// Center of continental US
	const CONUS_CENTER = {
		lat: 39.8283,
		lng: -98.5795
	};

	onMount(async () => {
		if (browser) {
			await loadGoogleMapsScript();
			initializeMap();
			initializeCompass();
		}
	});

	async function loadGoogleMapsScript(): Promise<void> {
		return new Promise((resolve, reject) => {
			// Check if Google Maps is already loaded
			if (window.google && window.google.maps) {
				resolve();
				return;
			}

			// Create script element
			const script = document.createElement('script');
			script.src = `https://maps.googleapis.com/maps/api/js?key=${GOOGLE_MAPS_API_KEY}&libraries=geometry,marker`;
			script.async = true;
			script.defer = true;

			script.onload = () => resolve();
			script.onerror = () => reject(new Error('Failed to load Google Maps API'));

			document.head.appendChild(script);
		});
	}

	function initializeMap(): void {
		if (!mapContainer || !window.google) return;

		// Initialize map centered on continental US
		map = new google.maps.Map(mapContainer, {
			mapId: 'SOAR_MAP', // Required for AdvancedMarkerElement
			center: CONUS_CENTER,
			zoom: 4, // Shows continental US
			mapTypeId: window.google.maps.MapTypeId.TERRAIN,
			mapTypeControl: true,
			mapTypeControlOptions: {
				style: window.google.maps.MapTypeControlStyle.HORIZONTAL_BAR,
				position: window.google.maps.ControlPosition.LEFT_BOTTOM
			},
			zoomControl: false,
			scaleControl: true,
			streetViewControl: true,
			streetViewControlOptions: {
				position: window.google.maps.ControlPosition.RIGHT_TOP
			},
			fullscreenControl: true,
			fullscreenControlOptions: {
				position: window.google.maps.ControlPosition.RIGHT_TOP
			}
		});

		console.log('Google Maps initialized for operations view');
	}

	async function locateUser(): Promise<void> {
		if (!navigator.geolocation) {
			alert('Geolocation is not supported by this browser.');
			return;
		}

		isLocating = true;

		try {
			const position = await getCurrentPosition();
			const userLocation = {
				lat: position.coords.latitude,
				lng: position.coords.longitude
			};

			console.log(`User location found: ${userLocation.lat}, ${userLocation.lng}`);

			// Step 1: First place the marker at user location
			// Create a custom content element for the marker
			const markerContent = document.createElement('div');
			markerContent.innerHTML = `
				<div style="
					width: 24px;
					height: 24px;
					background: #4285F4;
					border: 2px solid #FFFFFF;
					border-radius: 50%;
					position: relative;
					box-shadow: 0 2px 6px rgba(0,0,0,0.3);
				">
					<div style="
						width: 6px;
						height: 6px;
						background: #FFFFFF;
						border-radius: 50%;
						position: absolute;
						top: 50%;
						left: 50%;
						transform: translate(-50%, -50%);
					"></div>
				</div>
			`;

			// Remove existing marker if present
			if (userMarker) {
				userMarker.map = null;
			}

			userMarker = new google.maps.marker.AdvancedMarkerElement({
				position: userLocation,
				map: map,
				title: 'Your Location',
				content: markerContent
			});

			// Step 2: Animate pan to user location
			if (map) {
				map.panTo(userLocation);

				// Step 3: Wait for pan animation to complete, then zoom in smoothly
				setTimeout(() => {
					// Smooth zoom animation to show approximately 10-mile radius
					const targetZoom = 13;
					const currentZoom = map.getZoom() || 4;

					// Animate zoom gradually for smoother transition
					animateZoom(currentZoom, targetZoom);
				}, 1000); // Wait for pan animation to complete
			}

			console.log(`User located and animated to: ${userLocation.lat}, ${userLocation.lng}`);
		} catch (error) {
			console.error('Error getting user location:', error);
			alert('Unable to get your location. Please make sure location services are enabled.');
		} finally {
			isLocating = false;
		}
	}

	function animateZoom(currentZoom: number, targetZoom: number): void {
		if (!map || currentZoom >= targetZoom) return;

		const zoomStep = Math.min(1, targetZoom - currentZoom);
		const nextZoom = currentZoom + zoomStep;

		map.setZoom(nextZoom);

		if (nextZoom < targetZoom) {
			setTimeout(() => animateZoom(nextZoom, targetZoom), 200);
		}
	}

	function getCurrentPosition(): Promise<GeolocationPosition> {
		return new Promise((resolve, reject) => {
			navigator.geolocation.getCurrentPosition(resolve, reject, {
				enableHighAccuracy: true,
				timeout: 10000,
				maximumAge: 300000 // 5 minutes
			});
		});
	}

	async function initializeCompass(): Promise<void> {
		// Check if we need to request permission (for iOS 13+)
		if (
			'requestPermission' in DeviceOrientationEvent &&
			typeof DeviceOrientationEvent.requestPermission === 'function'
		) {
			try {
				const permission = await DeviceOrientationEvent.requestPermission();
				if (permission !== 'granted') {
					console.log('Device orientation permission denied');
					return;
				}
			} catch (error) {
				console.log('Error requesting device orientation permission:', error);
				return;
			}
		}

		// Try to lock screen orientation
		if ('screen' in window && 'orientation' in window.screen) {
			try {
				// Use type assertion to handle potential undefined
				const orientation = window.screen.orientation as any;
				if (orientation && typeof orientation.lock === 'function') {
					await orientation.lock('portrait-primary');
					screenOrientation = 'portrait-primary';
					console.log('Screen orientation locked to portrait');
				}
			} catch (error) {
				console.log('Could not lock screen orientation:', error);
			}
		}

		window.addEventListener('deviceorientation', handleOrientationChange);
	}

	function handleOrientationChange(event: DeviceOrientationEvent): void {
		if (event.alpha !== null) {
			isCompassActive = true;
			// Normalize the heading to ensure it's always between 0 and 360
			deviceHeading = event.alpha;
			displayHeading = Math.round(deviceHeading);

			// Adjust compass rotation to keep red arrow pointing north
			// Use absolute value to ensure consistent rotation
			compassHeading = -deviceHeading;

			// Ensure compassHeading is between 0 and 360
			compassHeading = ((compassHeading % 360) + 360) % 360;
		}
	}
</script>

<svelte:head>
	<title>Operations - Glider Flights</title>
</svelte:head>

<div class="fixed inset-0 top-16 w-full">
	<!-- Google Maps Container -->
	<div bind:this={mapContainer} class="h-full w-full"></div>

	<!-- Location Button -->
	<div class="absolute top-4 left-4 z-10">
		<button
			bind:this={userLocationButton}
			class="location-btn"
			class:opacity-50={isLocating}
			disabled={isLocating}
			on:click={locateUser}
			title={isLocating ? 'Locating...' : 'Find My Location'}
		>
			{#if isLocating}
				<div
					class="h-5 w-5 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"
				></div>
			{:else}
				<span class="text-xl">📍</span>
			{/if}
		</button>
	</div>

	<!-- Compass Rose -->
	{#if isCompassActive}
		<div class="compass-container absolute top-8 left-1/2 z-10 -translate-x-1/2 transform">
			<div class="compass-rose" style="transform: rotate({compassHeading}deg)">
				<svg width="80" height="80" viewBox="0 0 80 80">
					<!-- Outer circle -->
					<circle
						cx="40"
						cy="40"
						r="38"
						fill="rgba(255, 255, 255, 0.9)"
						stroke="rgba(0, 0, 0, 0.3)"
						stroke-width="2"
					/>

					<!-- Cardinal direction markers -->
					<!-- North -->
					<line
						x1="40"
						y1="4"
						x2="40"
						y2="16"
						stroke="#dc2626"
						stroke-width="3"
						stroke-linecap="round"
					/>

					<!-- South -->
					<line
						x1="40"
						y1="64"
						x2="40"
						y2="76"
						stroke="#374151"
						stroke-width="2"
						stroke-linecap="round"
					/>

					<!-- East -->
					<line
						x1="64"
						y1="40"
						x2="76"
						y2="40"
						stroke="#374151"
						stroke-width="2"
						stroke-linecap="round"
					/>

					<!-- West -->
					<line
						x1="4"
						y1="40"
						x2="16"
						y2="40"
						stroke="#374151"
						stroke-width="2"
						stroke-linecap="round"
					/>

					<!-- Intercardinal directions -->
					<!-- Northeast -->
					<line
						x1="56.57"
						y1="23.43"
						x2="64.85"
						y2="15.15"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- Southeast -->
					<line
						x1="56.57"
						y1="56.57"
						x2="64.85"
						y2="64.85"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- Southwest -->
					<line
						x1="23.43"
						y1="56.57"
						x2="15.15"
						y2="64.85"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- Northwest -->
					<line
						x1="23.43"
						y1="23.43"
						x2="15.15"
						y2="15.15"
						stroke="#6b7280"
						stroke-width="1.5"
						stroke-linecap="round"
					/>

					<!-- North arrow (pointing up) -->
					<polygon
						points="40,8 44,18 40,16 36,18"
						fill="#dc2626"
						stroke="#dc2626"
						stroke-width="1"
					/>
				</svg>
			</div>
			<div
				class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 text-[24px] font-bold text-gray-700"
			>
				{displayHeading}°
			</div>
		</div>
	{/if}
</div>

<style>
	/* Location button styling */
	.location-btn {
		background: white;
		border: none;
		border-radius: 0.375rem;
		padding: 0.75rem;
		cursor: pointer;
		transition: all 200ms;
		box-shadow: 0 2px 8px rgba(0, 0, 0, 0.15);
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.location-btn:hover {
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
		transform: translateY(-1px);
	}

	.location-btn:focus {
		outline: none;
		box-shadow:
			0 0 0 2px rgba(59, 130, 246, 0.5),
			0 2px 8px rgba(0, 0, 0, 0.15);
	}

	.location-btn:disabled {
		cursor: not-allowed;
		opacity: 0.5;
		transform: none;
	}

	/* Loading spinner animation */
	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.animate-spin {
		animation: spin 1s linear infinite;
	}

	/* Compass rose styling */
	.compass-container {
		filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.3));
	}

	.compass-rose {
		transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
	}
</style>
