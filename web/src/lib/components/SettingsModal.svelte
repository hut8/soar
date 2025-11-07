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
		positionFixWindow?: number;
		// Legacy support for old settings
		trailLength?: number[];
	}

	// Settings state
	let showCompassRose = $state(true);
	let showAirportMarkers = $state(true);
	let showRunwayOverlays = $state(false);
	let positionFixWindow = $state(8); // Hours - default 8 hours

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
					// Use positionFixWindow, or fallback to trailLength for legacy support
					positionFixWindow =
						backendSettings.positionFixWindow ??
						(backendSettings.trailLength ? backendSettings.trailLength[0] : 8);
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
				// Use positionFixWindow, or fallback to trailLength for legacy support
				positionFixWindow =
					settings.positionFixWindow ?? (settings.trailLength ? settings.trailLength[0] : 8);
			} catch (e) {
				console.warn('Failed to load settings from localStorage:', e);
				positionFixWindow = 8;
			}
		} else {
			positionFixWindow = 8;
		}
	}

	async function saveSettings() {
		if (!browser) return;

		const settings = {
			showCompassRose,
			showAirportMarkers,
			showRunwayOverlays,
			positionFixWindow
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
				positionFixWindow
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
					positionFixWindow
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
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 dark:bg-black/70"
		onclick={() => (showModal = false)}
		onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
		role="presentation"
	>
		<div
			class="card bg-surface-50 text-surface-900 dark:bg-surface-900 dark:text-surface-50 max-h-[80vh] w-full max-w-lg overflow-y-auto p-4 shadow-xl"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && (showModal = false)}
			role="dialog"
			aria-modal="true"
			aria-labelledby="settings-modal-title"
			tabindex="-1"
		>
			<div class="mb-4 flex items-center justify-between">
				<h2 id="settings-modal-title" class="text-xl font-bold">Map Settings</h2>
				<button class="preset-tonal-surface-500 btn btn-sm" onclick={() => (showModal = false)}>
					<X size={20} />
				</button>
			</div>

			<div class="space-y-6">
				<!-- Display Options -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Display Options</h3>
					<div class="space-y-3">
						<Switch
							class="flex justify-between p-2"
							checked={showCompassRose}
							onCheckedChange={(details) => {
								showCompassRose = details.checked;
								saveSettings();
							}}
						>
							<Switch.Label class="text-sm font-medium">Show Compass Rose</Switch.Label>
							<Switch.Control>
								<Switch.Thumb />
							</Switch.Control>
							<Switch.HiddenInput name="compass-toggle" />
						</Switch>
						<Switch
							class="flex justify-between p-2"
							checked={showAirportMarkers}
							onCheckedChange={(details) => {
								showAirportMarkers = details.checked;
								saveSettings();
							}}
						>
							<Switch.Label class="text-sm font-medium">Show Airport Markers</Switch.Label>
							<Switch.Control>
								<Switch.Thumb />
							</Switch.Control>
							<Switch.HiddenInput name="airports-toggle" />
						</Switch>
						<Switch
							class="flex justify-between p-2"
							checked={showRunwayOverlays}
							onCheckedChange={(details) => {
								showRunwayOverlays = details.checked;
								saveSettings();
							}}
						>
							<Switch.Label class="text-sm font-medium">Show Runway Overlays</Switch.Label>
							<Switch.Control>
								<Switch.Thumb />
							</Switch.Control>
							<Switch.HiddenInput name="runways-toggle" />
						</Switch>
					</div>
				</section>

				<!-- Position Fix Window -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Position Fix Window</h3>
					<p class="text-surface-600 dark:text-surface-400 mb-3 text-sm">
						Only show devices that have been seen within this time window
					</p>
					<div class="space-y-4">
						<div class="text-sm font-medium">
							Duration: {positionFixWindow === 0
								? 'None'
								: positionFixWindow < 1
									? `${Math.round(positionFixWindow * 60)} minutes`
									: `${positionFixWindow} hours`}
						</div>
						<Slider
							value={[positionFixWindow]}
							onValueChange={(details) => {
								positionFixWindow = details.value[0];
								saveSettings();
							}}
							min={0}
							max={24}
							step={1}
						>
							<Slider.Control>
								<Slider.Track>
									<Slider.Range />
								</Slider.Track>
								<Slider.Thumb index={0}>
									<Slider.HiddenInput />
								</Slider.Thumb>
							</Slider.Control>
						</Slider>
						<div class="text-surface-500 dark:text-surface-400 flex justify-between text-xs">
							<span>None</span>
							<span>12h</span>
							<span>24h</span>
						</div>
					</div>
				</section>

				<!-- Cache Management -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Cache Management</h3>
					<div class="space-y-3">
						<p class="text-surface-600 dark:text-surface-400 text-sm">
							Clear all cached device data from your browser's local storage.
						</p>
						<button
							class="btn preset-filled-error-500 w-full"
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
