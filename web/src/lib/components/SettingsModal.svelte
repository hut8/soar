<script lang="ts">
	import { Switch, Slider } from '@skeletonlabs/skeleton-svelte';
	import { X, Trash2 } from '@lucide/svelte';
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { DeviceRegistry } from '$lib/services/DeviceRegistry';
	import { toaster } from '$lib/toaster';
	import { auth } from '$lib/stores/auth';
	import { serverCall } from '$lib/api/server';

	// Props
	let { showModal = $bindable(), onSettingsChange } = $props();

	// Settings interface
	interface SettingsData {
		showCompassRose?: boolean;
		showAirportMarkers?: boolean;
		showRunwayOverlays?: boolean;
		trailLength?: number[];
	}

	// Settings state
	let showCompassRose = $state(true);
	let showAirportMarkers = $state(true);
	let showRunwayOverlays = $state(false);
	let trailLength = $state([0]); // Hours - logarithmic scale
	let trailLengthSlider = $state([0]); // Linear slider position (0-100)

	// Logarithmic trail length conversion
	// 0 = 0 hours, 50 = ~5 hours, 100 = 24 hours
	function sliderToHours(sliderValue: number): number {
		if (sliderValue === 0) return 0;
		// Logarithmic scale: slider 50 = 5 hours, slider 100 = 24 hours
		const exponent = (sliderValue / 100) * Math.log(24);
		return Math.exp(exponent) * (5 / Math.exp(Math.log(24) * 0.5));
	}

	function hoursToSlider(hours: number): number {
		if (hours === 0) return 0;
		// Reverse of the logarithmic conversion
		const normalized = hours / (5 / Math.exp(Math.log(24) * 0.5));
		return (Math.log(normalized) / Math.log(24)) * 100;
	}

	// Settings persistence functions
	async function loadSettings() {
		if (!browser) return;

		// If user is authenticated, load from backend
		if ($auth.isAuthenticated && $auth.token) {
			try {
				const backendSettings = await serverCall<SettingsData>('/user/settings', {
					headers: {
						Authorization: `Bearer ${$auth.token}`
					}
				});

				// Apply backend settings if they exist
				if (backendSettings && Object.keys(backendSettings).length > 0) {
					showCompassRose = backendSettings.showCompassRose ?? true;
					showAirportMarkers = backendSettings.showAirportMarkers ?? true;
					showRunwayOverlays = backendSettings.showRunwayOverlays ?? false;
					trailLength = backendSettings.trailLength ?? [0];
					trailLengthSlider = [hoursToSlider(trailLength[0])];
					return;
				}
			} catch (e) {
				console.warn('Failed to load settings from backend, falling back to localStorage:', e);
			}
		}

		// Fallback to localStorage for non-authenticated users or if backend fails
		const saved = localStorage.getItem('operationsSettings');
		if (saved) {
			try {
				const settings = JSON.parse(saved);
				showCompassRose = settings.showCompassRose ?? true;
				showAirportMarkers = settings.showAirportMarkers ?? true;
				showRunwayOverlays = settings.showRunwayOverlays ?? false;
				trailLength = settings.trailLength ?? [0];
				trailLengthSlider = [hoursToSlider(trailLength[0])];
			} catch (e) {
				console.warn('Failed to load settings from localStorage:', e);
				trailLengthSlider = [0];
			}
		} else {
			trailLengthSlider = [0];
		}
	}

	async function saveSettings() {
		if (!browser) return;

		const settings = {
			showCompassRose,
			showAirportMarkers,
			showRunwayOverlays,
			trailLength
		};

		// Always save to localStorage for offline support
		localStorage.setItem('operationsSettings', JSON.stringify(settings));

		// If user is authenticated, also save to backend
		if ($auth.isAuthenticated && $auth.token) {
			try {
				await serverCall('/user/settings', {
					method: 'PUT',
					headers: {
						Authorization: `Bearer ${$auth.token}`
					},
					body: JSON.stringify(settings)
				});
			} catch (e) {
				console.warn('Failed to save settings to backend:', e);
				toaster.warning({
					title: 'Settings saved locally only'
				});
			}
		}

		// Notify parent component of settings changes
		if (onSettingsChange) {
			onSettingsChange({
				showCompassRose,
				showAirportMarkers,
				showRunwayOverlays,
				trailLength: trailLength[0]
			});
		}
	}

	onMount(() => {
		if (browser) {
			loadSettings();
			// Notify parent of initial settings
			if (onSettingsChange) {
				onSettingsChange({
					showCompassRose,
					showAirportMarkers,
					showRunwayOverlays,
					trailLength: trailLength[0]
				});
			}
		}
	});

	function clearDevicesCache() {
		if (
			confirm(
				'Are you sure you want to clear all cached device data? This will remove all stored devices from your browser.'
			)
		) {
			try {
				DeviceRegistry.getInstance().clear();
				toaster.success({
					title: 'Device cache cleared successfully'
				});
			} catch (error) {
				console.error('Failed to clear device cache:', error);
				toaster.error({
					title: 'Failed to clear device cache'
				});
			}
		}
	}
</script>

<!-- Settings Modal -->
{#if showModal}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-surface-950-50/50"
		onclick={() => (showModal = false)}
		onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
		tabindex="-1"
		role="dialog"
	>
		<div
			class="max-h-[80vh] w-full max-w-lg overflow-y-auto card bg-white p-4 text-gray-900 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
			role="dialog"
			tabindex="0"
		>
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-xl font-bold">Map Settings</h2>
				<button class="preset-tonal-surface-500 btn btn-sm" onclick={() => (showModal = false)}>
					<X size={20} />
				</button>
			</div>

			<div class="space-y-6">
				<!-- Display Options -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Display Options</h3>
					<div class="space-y-3">
						<div class="flex items-center justify-between">
							<label for="compass-toggle" class="text-sm font-medium">Show Compass Rose</label>
							<Switch
								name="compass-toggle"
								checked={showCompassRose}
								onCheckedChange={(e) => {
									showCompassRose = e.checked;
									saveSettings();
								}}
							/>
						</div>
						<div class="flex items-center justify-between">
							<label for="airports-toggle" class="text-sm font-medium">Show Airport Markers</label>
							<Switch
								name="airports-toggle"
								checked={showAirportMarkers}
								onCheckedChange={(e) => {
									showAirportMarkers = e.checked;
									saveSettings();
								}}
							/>
						</div>
						<div class="flex items-center justify-between">
							<label for="runways-toggle" class="text-sm font-medium">Show Runway Overlays</label>
							<Switch
								name="runways-toggle"
								checked={showRunwayOverlays}
								onCheckedChange={(e) => {
									showRunwayOverlays = e.checked;
									saveSettings();
								}}
							/>
						</div>
					</div>
				</section>

				<!-- Trail Length -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Trail Length</h3>
					<div class="space-y-4">
						<div class="text-sm font-medium">
							Duration: {trailLength[0] === 0
								? 'None'
								: trailLength[0] < 1
									? `${Math.round(trailLength[0] * 60)} minutes`
									: `${Math.round(trailLength[0] * 10) / 10} hours`}
						</div>
						<Slider
							value={trailLengthSlider}
							onValueChange={(e) => {
								trailLengthSlider = e.value;
								trailLength = [sliderToHours(e.value[0])];
								saveSettings();
							}}
							min={0}
							max={100}
							step={1}
						/>
						<div class="flex justify-between text-xs text-gray-500">
							<span>None</span>
							<span>~5h</span>
							<span>24h</span>
						</div>
					</div>
				</section>

				<!-- Cache Management -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Cache Management</h3>
					<div class="space-y-3">
						<p class="text-sm text-gray-600">
							Clear all cached device data from your browser's local storage.
						</p>
						<button
							class="btn w-full preset-filled-error-500"
							onclick={clearDevicesCache}
							type="button"
						>
							<Trash2 size={16} />
							<span>Clear Devices Cache</span>
						</button>
					</div>
				</section>
			</div>
		</div>
	</div>
{/if}
