<script lang="ts">
	import { onMount } from 'svelte';
	import { browser } from '$app/environment';
	import '$lib/types/google-maps.d.ts';

	// Placeholder for Google Maps API key - to be added later
	const GOOGLE_MAPS_API_KEY = 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

	let mapContainer: HTMLElement;
	let map: GoogleMap;
	let userLocationButton: HTMLButtonElement;
	let isLocating = false;

	// Continental US bounds for initial display
	const CONUS_BOUNDS = {
		north: 49.3457868, // Northern border
		south: 24.7433195, // Southern border
		west: -124.7844079, // Western border
		east: -66.9513812 // Eastern border
	};

	// Center of continental US
	const CONUS_CENTER = {
		lat: 39.8283,
		lng: -98.5795
	};

	onMount(async () => {
		if (browser) {
			await loadGoogleMapsScript();
			initializeMap();
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
			script.src = `https://maps.googleapis.com/maps/api/js?key=${GOOGLE_MAPS_API_KEY}&libraries=geometry`;
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
		map = new window.google.maps.Map(mapContainer, {
			center: CONUS_CENTER,
			zoom: 4, // Shows continental US
			mapTypeId: window.google.maps.MapTypeId.TERRAIN,
			restriction: {
				latLngBounds: CONUS_BOUNDS,
				strictBounds: false
			},
			mapTypeControl: true,
			mapTypeControlOptions: {
				style: window.google.maps.MapTypeControlStyle.HORIZONTAL_BAR,
				position: window.google.maps.ControlPosition.TOP_CENTER
			},
			zoomControl: true,
			zoomControlOptions: {
				position: window.google.maps.ControlPosition.RIGHT_CENTER
			},
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
			console.log('Map object:', map);

			// Center map on user location - use the same object format as the marker
			map?.setCenter(userLocation);

			// Zoom to approximately 10 miles in the smaller dimension
			// Zoom level 13-14 typically shows about 10-20 miles depending on screen size
			map?.setZoom(13);

			console.log('Map centered and zoomed to user location');

			// Add a marker for user location
			new window.google.maps.Marker({
				position: userLocation,
				map: map,
				title: 'Your Location',
				icon: {
					url: 'data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQiIGhlaWdodD0iMjQiIHZpZXdCb3g9IjAgMCAyNCAyNCIgZmlsbD0ibm9uZSIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj4KPGNpcmNsZSBjeD0iMTIiIGN5PSIxMiIgcj0iMTAiIGZpbGw9IiM0Mjg1RjQiIHN0cm9rZT0iI0ZGRkZGRiIgc3Ryb2tlLXdpZHRoPSIyIi8+CjxjaXJjbGUgY3g9IjEyIiBjeT0iMTIiIHI9IjMiIGZpbGw9IiNGRkZGRkYiLz4KPC9zdmc+',
					size: new window.google.maps.Size(24, 24),
					anchor: new window.google.maps.Point(12, 12)
				}
			});

			console.log(`User located at: ${userLocation.lat}, ${userLocation.lng}`);
		} catch (error) {
			console.error('Error getting user location:', error);
			alert('Unable to get your location. Please make sure location services are enabled.');
		} finally {
			isLocating = false;
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
</script>

<svelte:head>
	<title>Operations - Glider Flights</title>
</svelte:head>

<div class="relative h-screen w-full">
	<!-- Google Maps Container -->
	<div bind:this={mapContainer} class="h-full w-full"></div>

	<!-- Control Panel -->
	<div class="absolute top-4 left-4 z-10 rounded-lg bg-white p-4 shadow-lg">
		<h2 class="mb-3 text-lg font-semibold">Operations Center</h2>

		<div class="flex flex-col space-y-2">
			<button
				bind:this={userLocationButton}
				class="variant-filled-primary btn"
				class:opacity-50={isLocating}
				disabled={isLocating}
				on:click={locateUser}
			>
				{#if isLocating}
					<div class="flex items-center space-x-2">
						<div
							class="h-4 w-4 animate-spin rounded-full border-2 border-white border-t-transparent"
						></div>
						<span>Locating...</span>
					</div>
				{:else}
					<div class="flex items-center space-x-2">
						<span>📍</span>
						<span>Find My Location</span>
					</div>
				{/if}
			</button>
		</div>
	</div>

	<!-- Info Panel -->
	<div class="absolute bottom-4 left-4 z-10 max-w-sm rounded-lg bg-white p-4 shadow-lg">
		<h3 class="mb-2 font-semibold">Map Information</h3>
		<div class="space-y-1 text-sm text-gray-600">
			<p>• Use map controls to navigate and zoom</p>
			<p>• Click "Find My Location" to center on your position</p>
			<p>• Initial view shows the continental United States</p>
			<p>• Location zoom shows approximately 10-mile radius</p>
		</div>
	</div>
</div>

<style>
	/* Custom button styling for location button */
	.btn {
		padding: 1rem 1.5rem;
		border-radius: 0.375rem;
		font-weight: 500;
		transition: all 200ms;
		border: none;
		cursor: pointer;
	}

	.btn.variant-filled-primary {
		background-color: #2563eb;
		color: white;
	}

	.btn.variant-filled-primary:hover {
		background-color: #1d4ed8;
	}

	.btn.variant-filled-primary:focus {
		box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.5);
		outline: none;
	}

	.btn:disabled {
		cursor: not-allowed;
		opacity: 0.5;
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
</style>
