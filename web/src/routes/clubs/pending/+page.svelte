<script lang="ts">
	import { onMount } from 'svelte';
	import { resolve } from '$app/paths';
	import { Clock, Check, X, ArrowLeft, Users } from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { toaster } from '$lib/toaster';
	import type { Club, DataListResponse, DataResponse } from '$lib/types';
	import { formatDateTime } from '$lib/utils/dateFormatters';
	import { resolvedTimezone } from '$lib/stores/timezone';

	let clubs = $state<Club[]>([]);
	let loading = $state(true);
	let error = $state('');
	let processingId = $state<string | null>(null);

	let isAdmin = $derived($auth.isAuthenticated && $auth.user?.isAdmin);

	onMount(async () => {
		if (!isAdmin) return;
		await loadPendingClubs();
	});

	async function loadPendingClubs() {
		loading = true;
		error = '';
		try {
			const response = await serverCall<DataListResponse<Club>>('/clubs/pending');
			clubs = response.data || [];
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load pending clubs';
		} finally {
			loading = false;
		}
	}

	async function approveClub(clubId: string) {
		processingId = clubId;
		try {
			await serverCall<DataResponse<Club>>(`/clubs/${clubId}/approve`, { method: 'PUT' });
			clubs = clubs.filter((c) => c.id !== clubId);
			toaster.success({ title: 'Club approved' });
		} catch (err) {
			toaster.error({
				title: 'Failed to approve club',
				description: err instanceof Error ? err.message : 'Unknown error'
			});
		} finally {
			processingId = null;
		}
	}

	async function rejectClub(clubId: string) {
		if (!confirm('Are you sure you want to reject this club? Members will be removed.')) {
			return;
		}
		processingId = clubId;
		try {
			await serverCall<DataResponse<Club>>(`/clubs/${clubId}/reject`, { method: 'PUT' });
			clubs = clubs.filter((c) => c.id !== clubId);
			toaster.success({ title: 'Club rejected' });
		} catch (err) {
			toaster.error({
				title: 'Failed to reject club',
				description: err instanceof Error ? err.message : 'Unknown error'
			});
		} finally {
			processingId = null;
		}
	}
</script>

<svelte:head>
	<title>Pending Clubs - SOAR</title>
</svelte:head>

<div class="container mx-auto max-w-4xl space-y-6 p-4">
	<a href={resolve('/clubs')} class="inline-flex items-center gap-1 anchor text-sm">
		<ArrowLeft class="h-4 w-4" />
		Back to Clubs
	</a>

	<header class="space-y-2">
		<h1 class="flex items-center gap-2 h1">
			<Clock class="h-8 w-8" />
			Pending Clubs
		</h1>
		<p class="text-surface-500-400-token">Review and approve or reject new club submissions.</p>
	</header>

	{#if !isAdmin}
		<div class="card p-8 text-center">
			<p class="text-surface-500-400-token">You must be an administrator to view pending clubs.</p>
		</div>
	{:else if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center gap-4">
				<Progress class="h-8 w-8" />
				<span class="text-lg">Loading pending clubs...</span>
			</div>
		</div>
	{:else if error}
		<div
			class="rounded border border-red-200 bg-red-50 p-4 text-red-600 dark:border-red-800 dark:bg-red-950 dark:text-red-400"
		>
			{error}
		</div>
	{:else if clubs.length === 0}
		<div class="space-y-4 card p-12 text-center">
			<Users class="mx-auto h-16 w-16 text-surface-400" />
			<div class="space-y-2">
				<h3 class="h3">No pending clubs</h3>
				<p class="text-surface-500-400-token">All club submissions have been reviewed.</p>
			</div>
		</div>
	{:else}
		<div class="space-y-4">
			{#each clubs as club (club.id)}
				<article class="card p-6">
					<div class="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
						<div class="space-y-2">
							<h3 class="text-lg font-semibold">
								<a
									href={resolve(`/clubs/${club.id}`)}
									class="anchor text-primary-500 hover:text-primary-600"
								>
									{club.name}
								</a>
							</h3>

							{#if club.homeBaseAirportIdent}
								<p class="text-sm text-surface-600 dark:text-surface-400">
									Home Base: <span class="font-mono">{club.homeBaseAirportIdent}</span>
								</p>
							{/if}

							{#if club.location}
								<p class="text-sm text-surface-500">
									{[club.location.city, club.location.state, club.location.zipCode]
										.filter(Boolean)
										.join(', ') || 'No address'}
								</p>
							{/if}

							<p class="text-xs text-surface-400">
								Created {formatDateTime(club.createdAt, $resolvedTimezone)}
							</p>
						</div>

						<div class="flex gap-2">
							<button
								class="btn preset-filled-primary-500"
								disabled={processingId === club.id}
								onclick={() => approveClub(club.id)}
							>
								{#if processingId === club.id}
									<Progress class="mr-1 h-4 w-4" />
								{:else}
									<Check class="mr-1 h-4 w-4" />
								{/if}
								Approve
							</button>
							<button
								class="btn preset-filled-error-500"
								disabled={processingId === club.id}
								onclick={() => rejectClub(club.id)}
							>
								<X class="mr-1 h-4 w-4" />
								Reject
							</button>
						</div>
					</div>
				</article>
			{/each}
		</div>
	{/if}
</div>
