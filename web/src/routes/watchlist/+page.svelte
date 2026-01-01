<script lang="ts">
	import { onMount } from 'svelte';
	import { auth } from '$lib/stores/auth';
	import { watchlist } from '$lib/stores/watchlist';
	import { Bell, BellOff, Trash2, Plus } from '@lucide/svelte';
	import WatchlistModal from '$lib/components/WatchlistModal.svelte';

	let showAddModal = false;
	let loading = true;

	onMount(async () => {
		await watchlist.load();
		loading = false;
	});

	async function toggleEmailNotification(aircraftId: string, currentValue: boolean) {
		try {
			await watchlist.updateEmailPreference(aircraftId, !currentValue);
		} catch (err) {
			console.error('Failed to toggle email notification:', err);
		}
	}

	async function removeAircraft(aircraftId: string) {
		if (confirm('Remove this aircraft from your watchlist?')) {
			try {
				await watchlist.remove(aircraftId);
			} catch (err) {
				console.error('Failed to remove aircraft:', err);
			}
		}
	}

	async function clearWatchlist() {
		if (confirm('Remove all aircraft from your watchlist?')) {
			try {
				await watchlist.clear();
			} catch (err) {
				console.error('Failed to clear watchlist:', err);
			}
		}
	}
</script>

<svelte:head>
	<title>Watchlist - SOAR</title>
</svelte:head>

{#if $auth.isAuthenticated && $auth.user}
	<div class="container mx-auto max-w-7xl space-y-6 p-4">
		<!-- Header -->
		<div class="flex items-center justify-between">
			<div>
				<h1 class="h1">My Watchlist</h1>
				<p class="text-surface-600-300-token mt-2">
					Track aircraft and receive email notifications when flights complete.
				</p>
			</div>
			<div class="flex gap-2">
				<button onclick={() => (showAddModal = true)} class="btn preset-filled-primary-500">
					<Plus class="h-4 w-4" />
					Add Aircraft
				</button>
				{#if $watchlist.entries.length > 0}
					<button onclick={clearWatchlist} class="preset-ghost-error-500 btn">
						<Trash2 class="h-4 w-4" />
						Clear All
					</button>
				{/if}
			</div>
		</div>

		<!-- Loading State -->
		{#if loading}
			<div class="card p-8 text-center">
				<p class="text-surface-600-300-token">Loading watchlist...</p>
			</div>
		{:else if $watchlist.error}
			<div class="variant-ghost-error card p-4">
				<p class="text-error-500">Error: {$watchlist.error}</p>
			</div>
		{:else if $watchlist.entries.length === 0}
			<!-- Empty State -->
			<div class="card p-8 text-center">
				<p class="text-surface-600-300-token mb-4">
					Your watchlist is empty. Add aircraft to get started!
				</p>
				<button onclick={() => (showAddModal = true)} class="btn preset-filled-primary-500">
					<Plus class="h-4 w-4" />
					Add Aircraft
				</button>
			</div>
		{:else}
			<!-- Watchlist Grid -->
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
				{#each $watchlist.entries as entry (entry.aircraftId)}
					<div class="card p-4">
						<!-- Aircraft Info -->
						<div class="mb-3">
							{#if entry.aircraft}
								<a
									href="/aircraft/{entry.aircraftId}"
									class="font-semibold text-primary-500 hover:underline"
								>
									{entry.aircraft.registration || entry.aircraft.address}
								</a>
								{#if entry.aircraft.registration && entry.aircraft.registration !== entry.aircraft.address}
									<p class="text-surface-600-300-token text-sm">
										{entry.aircraft.address}
									</p>
								{/if}
							{:else}
								<p class="font-semibold">{entry.aircraftId}</p>
							{/if}
						</div>

						<!-- Email Toggle -->
						<div class="mb-3">
							<button
								onclick={() => toggleEmailNotification(entry.aircraftId, entry.sendEmail)}
								class="btn w-full btn-sm {entry.sendEmail
									? 'preset-filled-success-500'
									: 'preset-ghost-surface'}"
							>
								{#if entry.sendEmail}
									<Bell class="h-4 w-4" />
									Email notifications on
								{:else}
									<BellOff class="h-4 w-4" />
									Email notifications off
								{/if}
							</button>
						</div>

						<!-- Remove Button -->
						<button
							onclick={() => removeAircraft(entry.aircraftId)}
							class="preset-ghost-error-500 btn w-full btn-sm"
						>
							<Trash2 class="h-4 w-4" />
							Remove
						</button>

						<!-- Added Date -->
						<p class="text-surface-600-300-token mt-2 text-xs">
							Added {new Date(entry.createdAt).toLocaleDateString()}
						</p>
					</div>
				{/each}
			</div>
		{/if}

		<!-- Info Card -->
		<div class="card p-4">
			<h2 class="mb-2 h3">Email Notifications</h2>
			<p class="text-surface-600-300-token text-sm">
				When email notifications are enabled for an aircraft, you'll receive an email when that
				aircraft completes a flight. The email includes:
			</p>
			<ul class="text-surface-600-300-token mt-2 list-inside list-disc space-y-1 text-sm">
				<li>Flight details and link</li>
				<li>KML file attachment (viewable in Google Earth)</li>
				<li>Link to manage your watchlist</li>
			</ul>
		</div>
	</div>

	<!-- Add Aircraft Modal -->
	<WatchlistModal bind:showModal={showAddModal} />
{:else}
	<!-- Fallback (shouldn't be reached due to +page.ts redirect) -->
	<div class="container mx-auto max-w-2xl p-4 text-center">
		<h1 class="h1">Access Required</h1>
		<p class="text-surface-600-300-token mt-2">Please log in to view your watchlist.</p>
		<a href="/login" class="mt-4 btn preset-filled-primary-500">Login</a>
	</div>
{/if}
