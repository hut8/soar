<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { Plus, Trash2, Pencil, Plane, Users, MapPin } from '@lucide/svelte';
	import { listGeofences, deleteGeofence } from '$lib/api/geofences';
	import type { GeofenceWithCounts } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'Geofences']);

	let geofences: GeofenceWithCounts[] = [];
	let loading = true;
	let error = '';

	onMount(async () => {
		await loadGeofences();
	});

	async function loadGeofences() {
		loading = true;
		error = '';
		try {
			const response = await listGeofences();
			geofences = response.geofences;
		} catch (err) {
			logger.error('Failed to load geofences: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load geofences';
		} finally {
			loading = false;
		}
	}

	async function handleDelete(geofence: GeofenceWithCounts) {
		if (!confirm(`Delete "${geofence.name}"? This cannot be undone.`)) {
			return;
		}

		try {
			await deleteGeofence(geofence.id);
			geofences = geofences.filter((g) => g.id !== geofence.id);
			toaster.success({
				title: 'Geofence Deleted',
				description: `"${geofence.name}" has been deleted.`
			});
		} catch (err) {
			logger.error('Failed to delete geofence: {error}', { error: err });
			toaster.error({
				title: 'Delete Failed',
				description: err instanceof Error ? err.message : 'Failed to delete geofence'
			});
		}
	}

	function formatLayerSummary(layers: { floorFt: number; ceilingFt: number; radiusNm: number }[]) {
		if (layers.length === 0) return 'No layers';
		if (layers.length === 1) {
			const l = layers[0];
			return `${l.floorFt.toLocaleString()}-${l.ceilingFt.toLocaleString()} ft, ${l.radiusNm} nm`;
		}
		const minFloor = Math.min(...layers.map((l) => l.floorFt));
		const maxCeiling = Math.max(...layers.map((l) => l.ceilingFt));
		const maxRadius = Math.max(...layers.map((l) => l.radiusNm));
		return `${layers.length} layers, ${minFloor.toLocaleString()}-${maxCeiling.toLocaleString()} ft, up to ${maxRadius} nm`;
	}
</script>

<svelte:head>
	<title>Geofences - SOAR</title>
</svelte:head>

{#if $auth.isAuthenticated && $auth.user}
	<div class="container mx-auto max-w-7xl space-y-6 p-4">
		<!-- Header -->
		<div class="flex items-center justify-between">
			<div>
				<h1 class="h1">Geofences</h1>
				<p class="text-surface-600-300-token mt-2">
					Define boundaries and receive alerts when aircraft exit them.
				</p>
			</div>
			<a href="/geofences/new" class="btn preset-filled-primary-500">
				<Plus class="h-4 w-4" />
				Create Geofence
			</a>
		</div>

		<!-- Loading State -->
		{#if loading}
			<div class="card p-8 text-center">
				<p class="text-surface-600-300-token">Loading geofences...</p>
			</div>
		{:else if error}
			<div class="variant-ghost-error card p-4">
				<p class="text-error-500">Error: {error}</p>
				<button onclick={loadGeofences} class="preset-ghost-surface mt-2 btn">Retry</button>
			</div>
		{:else if geofences.length === 0}
			<!-- Empty State -->
			<div class="card p-8 text-center">
				<MapPin class="mx-auto mb-4 h-12 w-12 text-surface-400" />
				<h2 class="mb-2 h3">No Geofences Yet</h2>
				<p class="text-surface-600-300-token mb-4">
					Create a geofence to define a boundary and receive alerts when aircraft exit it.
				</p>
				<a href="/geofences/new" class="btn preset-filled-primary-500">
					<Plus class="h-4 w-4" />
					Create Your First Geofence
				</a>
			</div>
		{:else}
			<!-- Geofence Grid -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each geofences as geofence (geofence.id)}
					<div class="card p-4">
						<!-- Header -->
						<div class="mb-3">
							<a
								href="/geofences/{geofence.id}"
								class="font-semibold text-primary-500 hover:underline"
							>
								{geofence.name}
							</a>
							{#if geofence.description}
								<p class="text-surface-600-300-token mt-1 line-clamp-2 text-sm">
									{geofence.description}
								</p>
							{/if}
						</div>

						<!-- Layer Summary -->
						<p class="text-surface-600-300-token mb-3 text-sm">
							{formatLayerSummary(geofence.layers)}
						</p>

						<!-- Stats -->
						<div class="mb-3 flex gap-4 text-sm">
							<div class="flex items-center gap-1" title="Linked aircraft">
								<Plane class="h-4 w-4 text-surface-400" />
								<span class="text-surface-600-300-token">{geofence.aircraftCount}</span>
							</div>
							<div class="flex items-center gap-1" title="Subscribers">
								<Users class="h-4 w-4 text-surface-400" />
								<span class="text-surface-600-300-token">{geofence.subscriberCount}</span>
							</div>
						</div>

						<!-- Actions -->
						<div class="flex gap-2">
							<a href="/geofences/{geofence.id}" class="preset-ghost-surface btn flex-1 btn-sm">
								<Pencil class="h-4 w-4" />
								Edit
							</a>
							<button
								onclick={() => handleDelete(geofence)}
								class="preset-ghost-error-500 btn btn-sm"
							>
								<Trash2 class="h-4 w-4" />
							</button>
						</div>

						<!-- Created Date -->
						<p class="text-surface-600-300-token mt-2 text-xs">
							Created {new Date(geofence.createdAt).toLocaleDateString()}
						</p>
					</div>
				{/each}
			</div>
		{/if}

		<!-- Info Card -->
		<div class="card p-4">
			<h2 class="mb-2 h3">About Geofences</h2>
			<p class="text-surface-600-300-token text-sm">
				Geofences allow you to define boundaries with multiple altitude layers. Each layer can have
				a different radius from the center point, creating an "upside-down birthday cake" shape
				similar to Class B airspace.
			</p>
			<ul class="text-surface-600-300-token mt-2 list-inside list-disc space-y-1 text-sm">
				<li>Link aircraft to monitor them against the geofence boundary</li>
				<li>Subscribe to receive email alerts when an aircraft exits</li>
				<li>View exit events in the geofence detail page</li>
			</ul>
		</div>
	</div>
{:else}
	<!-- Fallback (shouldn't be reached due to +page.ts redirect) -->
	<div class="container mx-auto max-w-2xl p-4 text-center">
		<h1 class="h1">Access Required</h1>
		<p class="text-surface-600-300-token mt-2">Please log in to view geofences.</p>
		<a href="/login" class="mt-4 btn preset-filled-primary-500">Login</a>
	</div>
{/if}
