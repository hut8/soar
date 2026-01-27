<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { ArrowLeft } from '@lucide/svelte';
	import GeofenceEditor from '$lib/components/GeofenceEditor.svelte';
	import { createGeofence } from '$lib/api/geofences';
	import type { CreateGeofenceRequest } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'Geofences', 'New']);

	async function handleSave(request: CreateGeofenceRequest) {
		const response = await createGeofence(request);
		logger.info('Created geofence: {id}', { id: response.geofence.id });
		toaster.success({
			title: 'Geofence Created',
			description: `"${response.geofence.name}" has been created.`
		});
		goto(`/geofences/${response.geofence.id}`);
	}

	function handleCancel() {
		goto('/geofences');
	}
</script>

<svelte:head>
	<title>New Geofence - SOAR</title>
</svelte:head>

{#if $auth.isAuthenticated && $auth.user}
	<div class="container mx-auto h-full max-w-7xl space-y-4 p-4">
		<!-- Header -->
		<div class="flex items-center gap-4">
			<a href="/geofences" class="preset-ghost-surface btn btn-sm">
				<ArrowLeft class="h-4 w-4" />
				Back
			</a>
			<h1 class="h2">Create Geofence</h1>
		</div>

		<!-- Editor -->
		<div class="h-[calc(100vh-12rem)]">
			<GeofenceEditor onSave={handleSave} onCancel={handleCancel} isNew={true} />
		</div>
	</div>
{:else}
	<div class="container mx-auto max-w-2xl p-4 text-center">
		<h1 class="h1">Access Required</h1>
		<p class="text-surface-600-300-token mt-2">Please log in to create a geofence.</p>
		<a href="/login" class="mt-4 btn preset-filled-primary-500">Login</a>
	</div>
{/if}
