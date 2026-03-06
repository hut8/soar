<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { ArrowLeft, Check, X, Clock } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { getLogger } from '$lib/logging';
	import { resolvedTimezone } from '$lib/stores/timezone';
	import { formatShortDateTime } from '$lib/utils/dateFormatters';
	import { toaster } from '$lib/toaster';
	import type { ClubJoinRequestView, DataResponse, DataListResponse } from '$lib/types';
	import type { ClubView } from '$lib/types/generated/ClubView';

	const logger = getLogger(['soar', 'ClubJoinRequestsPage']);

	let club = $state<ClubView | null>(null);
	let requests = $state<ClubJoinRequestView[]>([]);
	let loadingClub = $state(true);
	let loadingRequests = $state(true);
	let error = $state('');
	let approvingId = $state<string | null>(null);
	let rejectingId = $state<string | null>(null);

	let clubId = $derived($page.params.id || '');
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);

	onMount(async () => {
		if (clubId) {
			await loadClub();
			await loadRequests();
		}
	});

	async function loadClub() {
		loadingClub = true;
		error = '';
		try {
			const response = await serverCall<DataResponse<ClubView>>(`/clubs/${clubId}`);
			club = response.data;
		} catch (err) {
			logger.error('Error loading club: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load club';
		} finally {
			loadingClub = false;
		}
	}

	async function loadRequests() {
		loadingRequests = true;
		try {
			const response = await serverCall<DataListResponse<ClubJoinRequestView>>(
				`/clubs/${clubId}/join-requests`
			);
			requests = response.data || [];
		} catch (err) {
			logger.error('Error loading join requests: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load join requests';
		} finally {
			loadingRequests = false;
		}
	}

	async function approveRequest(requestId: string) {
		approvingId = requestId;
		try {
			await serverCall(`/clubs/${clubId}/join-requests/${requestId}/approve`, {
				method: 'PUT'
			});
			requests = requests.filter((r) => r.id !== requestId);
			toaster.success({ title: 'Join request approved' });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			logger.error('Error approving request: {error}', { error: err });
			toaster.error({ title: 'Failed to approve request', description: errorMessage });
		} finally {
			approvingId = null;
		}
	}

	async function rejectRequest(requestId: string) {
		rejectingId = requestId;
		try {
			await serverCall(`/clubs/${clubId}/join-requests/${requestId}/reject`, {
				method: 'PUT'
			});
			requests = requests.filter((r) => r.id !== requestId);
			toaster.success({ title: 'Join request rejected' });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			logger.error('Error rejecting request: {error}', { error: err });
			toaster.error({ title: 'Failed to reject request', description: errorMessage });
		} finally {
			rejectingId = null;
		}
	}

	function formatDate(dateStr: string): string {
		return formatShortDateTime(dateStr, $resolvedTimezone);
	}

	function getUserName(request: ClubJoinRequestView): string {
		if (request.userFirstName && request.userLastName) {
			return `${request.userFirstName} ${request.userLastName}`;
		}
		if (request.userFirstName) return request.userFirstName;
		return request.userId.slice(0, 8) + '...';
	}

	function goBack() {
		goto(resolve(`/clubs/${clubId}`));
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club'} Join Requests - SOAR</title>
</svelte:head>

<div class="container mx-auto max-w-4xl p-4">
	<!-- Header -->
	<header class="mb-6 space-y-4">
		<div class="flex items-center gap-4">
			<button onclick={goBack} class="preset-tonal-surface-500 btn btn-sm" title="Back to club">
				<ArrowLeft class="h-4 w-4" />
			</button>
			<div>
				<h1 class="text-3xl font-bold">Join Requests</h1>
				{#if club}
					<p class="text-surface-600-300-token">{club.name}</p>
				{/if}
			</div>
		</div>
	</header>

	<!-- Loading State -->
	{#if loadingClub || loadingRequests}
		<div class="flex items-center justify-center py-12">
			<div class="h-12 w-12 animate-spin rounded-full border-b-2 border-primary-500"></div>
		</div>
	{:else if error}
		<div class="alert preset-filled-error-500">
			<p>{error}</p>
		</div>
	{:else if !userBelongsToClub && !$auth.user?.isAdmin}
		<div class="alert preset-filled-warning-500">
			<p class="font-semibold">Access Restricted</p>
			<p>You must be a member of this club to manage join requests.</p>
		</div>
	{:else if requests.length === 0}
		<div class="card p-8 text-center">
			<Clock class="mx-auto mb-4 h-12 w-12 text-surface-400" />
			<p class="text-surface-600-300-token text-lg">No pending join requests.</p>
		</div>
	{:else}
		<div class="space-y-4">
			{#each requests as request (request.id)}
				<div class="card p-4">
					<div class="flex flex-wrap items-center justify-between gap-4">
						<div class="flex-1 space-y-1">
							<p class="text-lg font-semibold">{getUserName(request)}</p>
							{#if request.message}
								<p class="text-surface-600-300-token text-sm italic">"{request.message}"</p>
							{/if}
							<p class="text-xs text-surface-500">
								Requested {formatDate(request.createdAt)}
							</p>
						</div>

						<div class="flex gap-2">
							<button
								class="btn preset-filled-success-500 btn-sm"
								onclick={() => approveRequest(request.id)}
								disabled={approvingId === request.id || rejectingId === request.id}
								title="Approve"
							>
								{#if approvingId === request.id}
									<div class="h-4 w-4 animate-spin rounded-full border-b-2 border-white"></div>
								{:else}
									<Check class="h-4 w-4" />
								{/if}
								Approve
							</button>
							<button
								class="btn preset-filled-error-500 btn-sm"
								onclick={() => rejectRequest(request.id)}
								disabled={approvingId === request.id || rejectingId === request.id}
								title="Reject"
							>
								{#if rejectingId === request.id}
									<div class="h-4 w-4 animate-spin rounded-full border-b-2 border-white"></div>
								{:else}
									<X class="h-4 w-4" />
								{/if}
								Reject
							</button>
						</div>
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
