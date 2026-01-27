<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { ArrowLeft, Plane, Users, Bell, Trash2 } from '@lucide/svelte';
	import GeofenceEditor from '$lib/components/GeofenceEditor.svelte';
	import {
		getGeofence,
		updateGeofence,
		deleteGeofence,
		getGeofenceAircraft,
		removeGeofenceAircraft,
		getGeofenceSubscribers,
		subscribeToGeofence,
		unsubscribeFromGeofence
	} from '$lib/api/geofences';
	import type { Geofence, GeofenceSubscriber } from '$lib/types';
	import { toaster } from '$lib/toaster';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'Geofences', 'Edit']);

	let { data } = $props();
	const geofenceId = data.geofenceId;

	let geofence: Geofence | null = $state(null);
	let aircraftIds: string[] = $state([]);
	let subscribers: GeofenceSubscriber[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let activeTab: 'edit' | 'aircraft' | 'subscribers' = $state('edit');

	// Check if current user is subscribed
	let userSubscription = $derived(
		$auth.user ? subscribers.find((s) => s.userId === $auth.user?.id) : null
	);

	onMount(async () => {
		await loadGeofence();
	});

	async function loadGeofence() {
		loading = true;
		error = '';
		try {
			const [geoResponse, aircraftResponse, subscribersResponse] = await Promise.all([
				getGeofence(geofenceId),
				getGeofenceAircraft(geofenceId),
				getGeofenceSubscribers(geofenceId)
			]);
			geofence = geoResponse.geofence;
			aircraftIds = aircraftResponse;
			subscribers = subscribersResponse;
		} catch (err) {
			logger.error('Failed to load geofence: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load geofence';
		} finally {
			loading = false;
		}
	}

	async function handleSave(request: Parameters<typeof updateGeofence>[1]) {
		const response = await updateGeofence(geofenceId, request);
		geofence = response.geofence;
		toaster.success({
			title: 'Geofence Updated',
			description: 'Your changes have been saved.'
		});
	}

	function handleCancel() {
		goto('/geofences');
	}

	async function handleDelete() {
		if (!geofence) return;
		if (!confirm(`Delete "${geofence.name}"? This cannot be undone.`)) return;

		try {
			await deleteGeofence(geofenceId);
			toaster.success({
				title: 'Geofence Deleted',
				description: `"${geofence.name}" has been deleted.`
			});
			goto('/geofences');
		} catch (err) {
			logger.error('Failed to delete geofence: {error}', { error: err });
			toaster.error({
				title: 'Delete Failed',
				description: err instanceof Error ? err.message : 'Failed to delete geofence'
			});
		}
	}

	async function handleRemoveAircraft(aircraftId: string) {
		if (!confirm('Remove this aircraft from the geofence?')) return;

		try {
			await removeGeofenceAircraft(geofenceId, aircraftId);
			aircraftIds = aircraftIds.filter((id) => id !== aircraftId);
			toaster.success({ title: 'Aircraft Removed' });
		} catch (err) {
			logger.error('Failed to remove aircraft: {error}', { error: err });
			toaster.error({
				title: 'Remove Failed',
				description: err instanceof Error ? err.message : 'Failed to remove aircraft'
			});
		}
	}

	async function handleToggleSubscription() {
		if (!$auth.user) return;

		try {
			if (userSubscription) {
				await unsubscribeFromGeofence(geofenceId, $auth.user.id);
				subscribers = subscribers.filter((s) => s.userId !== $auth.user?.id);
				toaster.success({
					title: 'Unsubscribed',
					description: 'You will no longer receive alerts.'
				});
			} else {
				const sub = await subscribeToGeofence(geofenceId, true);
				subscribers = [...subscribers, sub];
				toaster.success({ title: 'Subscribed', description: 'You will receive email alerts.' });
			}
		} catch (err) {
			logger.error('Failed to toggle subscription: {error}', { error: err });
			toaster.error({
				title: 'Error',
				description: err instanceof Error ? err.message : 'Failed to update subscription'
			});
		}
	}
</script>

<svelte:head>
	<title>{geofence?.name || 'Geofence'} - SOAR</title>
</svelte:head>

{#if $auth.isAuthenticated && $auth.user}
	<div class="container mx-auto h-full max-w-7xl space-y-4 p-4">
		<!-- Header -->
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-4">
				<a href="/geofences" class="preset-ghost-surface btn btn-sm">
					<ArrowLeft class="h-4 w-4" />
					Back
				</a>
				<h1 class="h2">{geofence?.name || 'Loading...'}</h1>
			</div>

			{#if geofence}
				<div class="flex gap-2">
					<button
						onclick={handleToggleSubscription}
						class="btn btn-sm {userSubscription
							? 'preset-filled-success-500'
							: 'preset-ghost-surface'}"
					>
						<Bell class="h-4 w-4" />
						{userSubscription ? 'Subscribed' : 'Subscribe'}
					</button>
					<button onclick={handleDelete} class="preset-ghost-error-500 btn btn-sm">
						<Trash2 class="h-4 w-4" />
						Delete
					</button>
				</div>
			{/if}
		</div>

		<!-- Loading/Error States -->
		{#if loading}
			<div class="card p-8 text-center">
				<p class="text-surface-600-300-token">Loading geofence...</p>
			</div>
		{:else if error}
			<div class="variant-ghost-error card p-4">
				<p class="text-error-500">Error: {error}</p>
				<button onclick={loadGeofence} class="preset-ghost-surface mt-2 btn">Retry</button>
			</div>
		{:else if geofence}
			<!-- Tabs -->
			<div class="border-surface-300-600-token flex gap-2 border-b pb-2">
				<button
					onclick={() => (activeTab = 'edit')}
					class="btn btn-sm {activeTab === 'edit'
						? 'preset-filled-primary-500'
						: 'preset-ghost-surface'}"
				>
					Edit
				</button>
				<button
					onclick={() => (activeTab = 'aircraft')}
					class="btn btn-sm {activeTab === 'aircraft'
						? 'preset-filled-primary-500'
						: 'preset-ghost-surface'}"
				>
					<Plane class="h-4 w-4" />
					Aircraft ({aircraftIds.length})
				</button>
				<button
					onclick={() => (activeTab = 'subscribers')}
					class="btn btn-sm {activeTab === 'subscribers'
						? 'preset-filled-primary-500'
						: 'preset-ghost-surface'}"
				>
					<Users class="h-4 w-4" />
					Subscribers ({subscribers.length})
				</button>
			</div>

			<!-- Tab Content -->
			{#if activeTab === 'edit'}
				<div class="h-[calc(100vh-16rem)]">
					<GeofenceEditor {geofence} onSave={handleSave} onCancel={handleCancel} isNew={false} />
				</div>
			{:else if activeTab === 'aircraft'}
				<div class="card p-4">
					<h3 class="mb-3 h4">Linked Aircraft</h3>
					<p class="text-surface-600-300-token mb-4 text-sm">
						Aircraft linked to this geofence will be monitored. When they exit the boundary, alerts
						will be sent to subscribers.
					</p>

					{#if aircraftIds.length === 0}
						<p class="text-surface-600-300-token text-sm">
							No aircraft linked yet. Use the aircraft page to add aircraft to this geofence.
						</p>
					{:else}
						<div class="space-y-2">
							{#each aircraftIds as aircraftId (aircraftId)}
								<div
									class="border-surface-300-600-token flex items-center justify-between rounded border p-2"
								>
									<a href="/aircraft/{aircraftId}" class="text-primary-500 hover:underline">
										{aircraftId}
									</a>
									<button
										onclick={() => handleRemoveAircraft(aircraftId)}
										class="preset-ghost-error-500 btn p-1 btn-sm"
									>
										<Trash2 class="h-4 w-4" />
									</button>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{:else if activeTab === 'subscribers'}
				<div class="card p-4">
					<h3 class="mb-3 h4">Subscribers</h3>
					<p class="text-surface-600-300-token mb-4 text-sm">
						Subscribers receive email alerts when aircraft exit this geofence.
					</p>

					{#if subscribers.length === 0}
						<p class="text-surface-600-300-token text-sm">No subscribers yet.</p>
					{:else}
						<div class="space-y-2">
							{#each subscribers as subscriber (subscriber.userId)}
								<div
									class="border-surface-300-600-token flex items-center justify-between rounded border p-2"
								>
									<span class="text-sm">
										{subscriber.userId === $auth.user?.id ? 'You' : subscriber.userId}
										{#if subscriber.sendEmail}
											<span class="ml-2 text-xs text-success-500">(Email enabled)</span>
										{/if}
									</span>
									<span class="text-surface-600-300-token text-xs">
										Since {new Date(subscriber.createdAt).toLocaleDateString()}
									</span>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		{/if}
	</div>
{:else}
	<div class="container mx-auto max-w-2xl p-4 text-center">
		<h1 class="h1">Access Required</h1>
		<p class="text-surface-600-300-token mt-2">Please log in to view this geofence.</p>
		<a href="/login" class="mt-4 btn preset-filled-primary-500">Login</a>
	</div>
{/if}
