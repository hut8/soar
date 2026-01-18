<script lang="ts">
	import { Switch } from '@skeletonlabs/skeleton-svelte';
	import { X, Trash2 } from '@lucide/svelte';
	import { browser } from '$app/environment';
	import { onMount } from 'svelte';
	import { AircraftRegistry } from '$lib/services/AircraftRegistry';
	import { toaster } from '$lib/toaster';
	import { auth } from '$lib/stores/auth';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'SettingsModal']);

	// Props
	let { showModal = $bindable(), onSettingsChange } = $props();

	// Settings interface
	interface SettingsData {
		showCompassRose?: boolean;
		showAirportMarkers?: boolean;
		showReceiverMarkers?: boolean;
		showAirspaceMarkers?: boolean;
		showRunwayOverlays?: boolean;
	}

	// Settings state
	let showCompassRose = $state(true);
	let showAirportMarkers = $state(true);
	let showReceiverMarkers = $state(true);
	let showAirspaceMarkers = $state(true);
	let showRunwayOverlays = $state(false);

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
					showReceiverMarkers = backendSettings.showReceiverMarkers ?? true;
					showAirspaceMarkers = backendSettings.showAirspaceMarkers ?? true;
					showRunwayOverlays = backendSettings.showRunwayOverlays ?? false;
					return;
				}
			} catch (e) {
				logger.warn('Failed to load settings from backend, falling back to localStorage: {error}', {
					error: e
				});
			}
		}

		// Fallback to localStorage for non-authenticated users or if backend fails
		const saved = localStorage.getItem('operationsSettings');
		if (saved) {
			try {
				const settings = JSON.parse(saved);
				showCompassRose = settings.showCompassRose ?? true;
				showAirportMarkers = settings.showAirportMarkers ?? true;
				showReceiverMarkers = settings.showReceiverMarkers ?? true;
				showAirspaceMarkers = settings.showAirspaceMarkers ?? true;
				showRunwayOverlays = settings.showRunwayOverlays ?? false;
			} catch (e) {
				logger.warn('Failed to load settings from localStorage: {error}', { error: e });
			}
		}
	}

	async function saveSettings() {
		if (!browser) return;

		const settings = {
			showCompassRose,
			showAirportMarkers,
			showReceiverMarkers,
			showAirspaceMarkers,
			showRunwayOverlays
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
				logger.warn('Failed to save settings to backend: {error}', { error: e });
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
				showReceiverMarkers,
				showAirspaceMarkers,
				showRunwayOverlays
			});
		}
	}

	onMount(async () => {
		if (browser) {
			await loadSettings();
			// Notify parent of initial settings AFTER loading completes
			if (onSettingsChange) {
				onSettingsChange({
					showCompassRose,
					showAirportMarkers,
					showReceiverMarkers,
					showAirspaceMarkers,
					showRunwayOverlays
				});
			}
		}
	});

	function clearAircraftCache() {
		if (
			confirm(
				'Are you sure you want to clear all cached aircraft data? This will remove all stored aircraft from your browser.'
			)
		) {
			try {
				AircraftRegistry.getInstance().clear();
				toaster.success({
					title: 'Aircraft cache cleared successfully'
				});
			} catch (error) {
				logger.error('Failed to clear device cache: {error}', { error });
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
			class="max-h-[80vh] w-full max-w-lg overflow-y-auto card bg-surface-50 p-4 text-surface-900 shadow-xl dark:bg-surface-900 dark:text-surface-50"
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
							checked={showReceiverMarkers}
							onCheckedChange={(details) => {
								showReceiverMarkers = details.checked;
								saveSettings();
							}}
						>
							<Switch.Label class="text-sm font-medium">Show Receiver Markers</Switch.Label>
							<Switch.Control>
								<Switch.Thumb />
							</Switch.Control>
							<Switch.HiddenInput name="receivers-toggle" />
						</Switch>
						<Switch
							class="flex justify-between p-2"
							checked={showAirspaceMarkers}
							onCheckedChange={(details) => {
								showAirspaceMarkers = details.checked;
								saveSettings();
							}}
						>
							<Switch.Label class="text-sm font-medium">Show Airspace Boundaries</Switch.Label>
							<Switch.Control>
								<Switch.Thumb />
							</Switch.Control>
							<Switch.HiddenInput name="airspaces-toggle" />
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

				<!-- Cache Management -->
				<section>
					<h3 class="mb-3 text-lg font-semibold">Cache Management</h3>
					<div class="space-y-3">
						<p class="text-sm text-surface-600 dark:text-surface-400">
							Clear all cached device data from your browser's local storage.
						</p>
						<button
							class="btn w-full preset-filled-error-500"
							onclick={clearAircraftCache}
							type="button"
						>
							<Trash2 size={16} />
							<span>Clear Aircraft Cache</span>
						</button>
					</div>
				</section>
			</div>
		</div>
	</div>
{/if}
