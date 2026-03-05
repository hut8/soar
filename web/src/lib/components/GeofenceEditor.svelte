<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Plus, Trash2, MapPin } from '@lucide/svelte';
	import type { Viewer } from 'cesium';
	import type { Geofence, GeofenceLayer, CreateGeofenceRequest, Airport } from '$lib/types';
	import { createGeofenceEntities, flyToGeofence } from '$lib/cesium/geofenceEntities';
	import { CESIUM_ION_TOKEN } from '$lib/config';
	import AirportSelector from '$lib/components/AirportSelector.svelte';
	import 'cesium/Build/Cesium/Widgets/widgets.css';

	// Props
	interface Props {
		geofence?: Geofence;
		onSave: (request: CreateGeofenceRequest) => Promise<void>;
		onCancel: () => void;
		isNew?: boolean;
		initialAirportId?: number;
	}

	let { geofence, onSave, onCancel, isNew = false, initialAirportId }: Props = $props();

	// Form state - initialized via $effect.pre to avoid state_referenced_locally warnings
	let name = $state('');
	let description = $state('');
	let centerLatitude = $state(39.8283);
	let centerLongitude = $state(-98.5795);
	let layers: GeofenceLayer[] = $state([{ floorFt: 0, ceilingFt: 5000, radiusNm: 5 }]);
	let formInitialized = false;

	// Sync form state with geofence prop
	$effect.pre(() => {
		if (geofence && !formInitialized) {
			name = geofence.name || '';
			description = geofence.description || '';
			centerLatitude = geofence.centerLatitude || 39.8283;
			centerLongitude = geofence.centerLongitude || -98.5795;
			layers = geofence.layers?.length
				? [...geofence.layers]
				: [{ floorFt: 0, ceilingFt: 5000, radiusNm: 5 }];
			formInitialized = true;
		}
	});

	// Ground elevation state
	let groundElevationFt: number | null = $state(null);
	let elevationLookupTimeout: ReturnType<typeof setTimeout>;

	async function lookupGroundElevation(lat: number, lng: number) {
		// Use the Cesium terrain provider to sample elevation once the viewer is ready
		if (!viewer) return;
		try {
			const { Cartographic, sampleTerrainMostDetailed } = window.Cesium;
			const positions = [Cartographic.fromDegrees(lng, lat)];
			const terrainProvider = viewer.terrainProvider;
			const updatedPositions = await sampleTerrainMostDetailed(terrainProvider, positions);
			if (updatedPositions && updatedPositions.length > 0) {
				const heightMeters = updatedPositions[0].height;
				if (heightMeters !== undefined && !isNaN(heightMeters)) {
					groundElevationFt = heightMeters / 0.3048;
					// Update first layer floor to ground elevation (rounded to nearest 100)
					if (layers.length > 0) {
						const roundedElevation = Math.round(groundElevationFt / 100) * 100;
						layers = layers.map((layer, i) =>
							i === 0 ? { ...layer, floorFt: roundedElevation } : layer
						);
					}
				}
			}
		} catch (err) {
			console.warn('Failed to sample terrain elevation:', err);
		}
	}

	// Airport selector state
	let airportSelectorValue: string[] = $state([]);
	let selectedAirportName = $state('');

	function handleAirportSelect(airport: Airport | null) {
		if (airport && airport.latitudeDeg != null && airport.longitudeDeg != null) {
			centerLatitude = airport.latitudeDeg;
			centerLongitude = airport.longitudeDeg;
			selectedAirportName = airport.icaoCode || airport.ident;
			// If name is empty and this is new, suggest the airport name
			if (isNew && !name.trim()) {
				name = `${selectedAirportName} Geofence`;
			}
			// Fly to the new center
			setTimeout(() => flyToCenter(), 100);
		}
	}

	let submitting = $state(false);
	let error = $state('');

	// Cesium viewer
	let cesiumContainer: HTMLDivElement;
	let viewer: Viewer | null = null;

	// Build a preview geofence object from current form state
	function buildPreviewGeofence(): Geofence {
		return {
			id: geofence?.id || 'preview',
			name: name || 'New Geofence',
			description: description || null,
			centerLatitude,
			centerLongitude,
			maxRadiusMeters: Math.max(...layers.map((l) => l.radiusNm * 1852)),
			layers,
			ownerUserId: geofence?.ownerUserId || '',
			clubId: geofence?.clubId ?? null,
			createdAt: geofence?.createdAt || new Date().toISOString(),
			updatedAt: geofence?.updatedAt || new Date().toISOString()
		};
	}

	// Update Cesium preview
	function updatePreview() {
		if (!viewer) return;

		// Remove existing entities
		viewer.entities.removeAll();

		// Add new entities
		const previewGeofence = buildPreviewGeofence();
		const entities = createGeofenceEntities(previewGeofence);
		entities.forEach((entity) => viewer!.entities.add(entity));
	}

	// Fly to current center
	function flyToCenter() {
		if (!viewer) return;
		const previewGeofence = buildPreviewGeofence();
		flyToGeofence(viewer, previewGeofence);
	}

	// Dynamically load Cesium script (must be loaded before using window.Cesium)
	// Cached per instance so concurrent callers share a single in-flight load
	let cesiumPromise: Promise<void> | null = null;
	function loadCesiumScript(): Promise<void> {
		if (cesiumPromise) return cesiumPromise;
		cesiumPromise = new Promise((resolve, reject) => {
			if (window.Cesium) {
				resolve();
				return;
			}

			const script = document.createElement('script');
			script.src = '/cesium/Cesium.js';
			script.async = true;
			script.onload = () => resolve();
			script.onerror = () => {
				cesiumPromise = null;
				script.remove();
				reject(new Error('Failed to load Cesium.js'));
			};
			document.head.appendChild(script);
		});
		return cesiumPromise;
	}

	// Initialize Cesium viewer
	onMount(async () => {
		try {
			await loadCesiumScript();

			const {
				Ion,
				Viewer: CesiumViewer,
				Terrain,
				SceneMode,
				createWorldImageryAsync
			} = window.Cesium;

			Ion.defaultAccessToken = CESIUM_ION_TOKEN;

			viewer = new CesiumViewer(cesiumContainer, {
				baseLayerPicker: false,
				geocoder: false,
				homeButton: false,
				infoBox: false,
				selectionIndicator: false,
				timeline: false,
				animation: false,
				navigationHelpButton: false,
				sceneModePicker: false,
				fullscreenButton: false,
				vrButton: false,
				terrain: Terrain.fromWorldTerrain(),
				sceneMode: SceneMode.SCENE3D
			});

			// Add imagery
			const imageryProvider = await createWorldImageryAsync();
			if (!viewer || viewer.isDestroyed()) return;
			viewer.imageryLayers.addImageryProvider(imageryProvider);

			// Initial preview
			updatePreview();

			// Fly to the geofence after a short delay
			setTimeout(() => {
				if (viewer) {
					flyToCenter();
				}
			}, 500);
		} catch (err) {
			console.error('Failed to initialize Cesium viewer:', err);
		}
	});

	onDestroy(() => {
		clearTimeout(elevationLookupTimeout);
		if (viewer) {
			viewer.destroy();
			viewer = null;
		}
	});

	// Update preview when form values change
	$effect(() => {
		// Track dependencies: name, centerLatitude, centerLongitude, layers
		void [name, centerLatitude, centerLongitude, JSON.stringify(layers)];
		updatePreview();
	});

	// Lookup ground elevation when center coordinates change
	$effect(() => {
		void [centerLatitude, centerLongitude];
		clearTimeout(elevationLookupTimeout);
		elevationLookupTimeout = setTimeout(() => {
			lookupGroundElevation(centerLatitude, centerLongitude);
		}, 500);
	});

	// Layer management
	function addLayer() {
		const lastLayer = layers[layers.length - 1];
		layers = [
			...layers,
			{
				floorFt: lastLayer ? lastLayer.ceilingFt : 0,
				ceilingFt: lastLayer ? lastLayer.ceilingFt + 5000 : 5000,
				radiusNm: lastLayer ? lastLayer.radiusNm + 2 : 5
			}
		];
	}

	function removeLayer(index: number) {
		if (layers.length <= 1) return;
		layers = layers.filter((_, i) => i !== index);
	}

	function updateLayer(index: number, field: keyof GeofenceLayer, value: number) {
		layers = layers.map((layer, i) => (i === index ? { ...layer, [field]: value } : layer));
	}

	// Validate form
	function validate(): string | null {
		if (!name.trim()) return 'Name is required';
		if (name.length > 255) return 'Name must be 255 characters or less';
		if (layers.length === 0) return 'At least one layer is required';
		for (let i = 0; i < layers.length; i++) {
			const layer = layers[i];
			if (layer.ceilingFt <= layer.floorFt) {
				return `Layer ${i + 1}: ceiling must be greater than floor`;
			}
			if (layer.radiusNm <= 0) {
				return `Layer ${i + 1}: radius must be positive`;
			}
		}
		if (centerLatitude < -90 || centerLatitude > 90) {
			return 'Latitude must be between -90 and 90';
		}
		if (centerLongitude < -180 || centerLongitude > 180) {
			return 'Longitude must be between -180 and 180';
		}
		return null;
	}

	// Handle form submission
	async function handleSubmit(event: Event) {
		event.preventDefault();

		const validationError = validate();
		if (validationError) {
			error = validationError;
			return;
		}

		submitting = true;
		error = '';

		try {
			// Build request - works for both create and update
			const request: CreateGeofenceRequest = {
				name: name.trim(),
				description: description.trim() || null,
				centerLatitude,
				centerLongitude,
				layers,
				clubId: null
			};
			await onSave(request);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to save geofence';
		} finally {
			submitting = false;
		}
	}
</script>

<div class="flex h-full flex-col gap-4 lg:flex-row">
	<!-- Form Panel -->
	<div class="w-full overflow-auto lg:w-1/2">
		<form onsubmit={handleSubmit} class="space-y-4">
			<!-- Error Display -->
			{#if error}
				<div class="variant-ghost-error card p-3">
					<p class="text-sm text-error-500">{error}</p>
				</div>
			{/if}

			<!-- Basic Info -->
			<div class="card p-4">
				<h3 class="mb-3 h4">Basic Information</h3>

				<div class="space-y-3">
					<label class="label">
						<span>Name *</span>
						<input
							type="text"
							class="input"
							bind:value={name}
							placeholder="Enter geofence name"
							maxlength="255"
							required
						/>
					</label>

					<label class="label">
						<span>Description</span>
						<textarea
							class="textarea"
							bind:value={description}
							placeholder="Optional description"
							rows="2"
						></textarea>
					</label>
				</div>
			</div>

			<!-- Center Point -->
			<div class="card p-4">
				<h3 class="mb-3 h4">Center Point</h3>

				<AirportSelector
					bind:value={airportSelectorValue}
					onSelect={handleAirportSelect}
					label="Set from Airport"
					placeholder="Search airports by name or identifier..."
					{initialAirportId}
				/>

				<div class="mt-3 grid grid-cols-2 gap-3">
					<label class="label">
						<span>Latitude</span>
						<input
							type="number"
							class="input"
							bind:value={centerLatitude}
							step="0.0001"
							min="-90"
							max="90"
							required
						/>
					</label>

					<label class="label">
						<span>Longitude</span>
						<input
							type="number"
							class="input"
							bind:value={centerLongitude}
							step="0.0001"
							min="-180"
							max="180"
							required
						/>
					</label>
				</div>

				{#if groundElevationFt !== null}
					<div class="mt-2 rounded bg-surface-200 px-3 py-1.5 text-sm dark:bg-surface-700">
						Ground elevation: <strong
							>{Math.round(groundElevationFt).toLocaleString()} ft MSL</strong
						>
					</div>
				{/if}

				<button type="button" onclick={flyToCenter} class="preset-ghost-surface mt-2 btn btn-sm">
					<MapPin class="h-4 w-4" />
					Fly to Center
				</button>
			</div>

			<!-- Layers -->
			<div class="card p-4">
				<div class="mb-3 flex items-center justify-between">
					<h3 class="h4">Altitude Layers</h3>
					<button type="button" onclick={addLayer} class="preset-ghost-surface btn btn-sm">
						<Plus class="h-4 w-4" />
						Add Layer
					</button>
				</div>

				<p class="text-surface-600-300-token mb-3 text-sm">
					Each layer defines an altitude range (MSL) and radius from the center point.
				</p>

				<div class="space-y-3">
					{#each layers as layer, index (index)}
						<div class="border-surface-300-600-token rounded border p-3">
							<div class="mb-2 flex items-center justify-between">
								<span class="text-sm font-medium">Layer {index + 1}</span>
								{#if layers.length > 1}
									<button
										type="button"
										onclick={() => removeLayer(index)}
										class="preset-ghost-error-500 btn p-1 btn-sm"
									>
										<Trash2 class="h-4 w-4" />
									</button>
								{/if}
							</div>

							<div class="grid grid-cols-3 gap-2">
								<label class="label">
									<span class="text-xs">Floor (ft MSL)</span>
									<input
										type="number"
										class="input-sm input"
										value={layer.floorFt}
										oninput={(e) =>
											updateLayer(index, 'floorFt', parseInt(e.currentTarget.value) || 0)}
										step="100"
									/>
								</label>

								<label class="label">
									<span class="text-xs">Ceiling (ft MSL)</span>
									<input
										type="number"
										class="input-sm input"
										value={layer.ceilingFt}
										oninput={(e) =>
											updateLayer(index, 'ceilingFt', parseInt(e.currentTarget.value) || 0)}
										step="100"
									/>
								</label>

								<label class="label">
									<span class="text-xs">Radius (nm)</span>
									<input
										type="number"
										class="input-sm input"
										value={layer.radiusNm}
										oninput={(e) =>
											updateLayer(index, 'radiusNm', parseFloat(e.currentTarget.value) || 0)}
										step="0.5"
										min="0.1"
									/>
								</label>
							</div>
						</div>
					{/each}
				</div>
			</div>

			<!-- Actions -->
			<div class="flex gap-2">
				<button type="submit" class="btn flex-1 preset-filled-primary-500" disabled={submitting}>
					{#if submitting}
						Saving...
					{:else}
						{isNew ? 'Create Geofence' : 'Save Changes'}
					{/if}
				</button>
				<button
					type="button"
					onclick={onCancel}
					class="preset-ghost-surface btn"
					disabled={submitting}
				>
					Cancel
				</button>
			</div>
		</form>
	</div>

	<!-- Cesium Preview Panel -->
	<div class="h-96 w-full lg:h-auto lg:w-1/2">
		<div class="h-full overflow-hidden card">
			<div class="border-surface-300-600-token border-b p-2">
				<h3 class="text-sm font-medium">3D Preview</h3>
			</div>
			<div bind:this={cesiumContainer} class="h-full min-h-80 w-full"></div>
		</div>
	</div>
</div>
