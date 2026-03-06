<script lang="ts">
	import { setContext } from 'svelte';
	import { page } from '$app/stores';
	import { resolve } from '$app/paths';
	import {
		Building,
		Plane,
		Info,
		ClipboardList,
		Users,
		ChevronDown,
		Shield,
		CreditCard
	} from '@lucide/svelte';
	import { Progress } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { getLogger } from '$lib/logging';
	import type { ClubWithSoaring, DataResponse, StripeConnectStatusView } from '$lib/types';
	import { writable } from 'svelte/store';

	const logger = getLogger(['soar', 'ClubLayout']);

	interface LayoutData {
		club: ClubWithSoaring | null;
		loading: boolean;
		error: string;
	}

	const clubStore = writable<LayoutData>({ club: null, loading: true, error: '' });
	setContext('clubLayout', clubStore);

	let club = $state<ClubWithSoaring | null>(null);
	let loading = $state(true);
	let error = $state('');
	let adminDropdownOpen = $state(false);
	let clubHasPayments = $state(false);

	let clubId = $derived($page.params.id || '');
	let isMember = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);
	let isClubAdmin = $derived(
		$auth.isAuthenticated &&
			(($auth.user?.clubId === clubId && $auth.user?.isClubAdmin) || $auth.user?.isAdmin)
	);
	let currentPath = $derived($page.url.pathname);

	let activeTab = $derived(
		(() => {
			if (currentPath.includes('/operations')) return 'operations';
			if (currentPath.includes('/members')) return 'members';
			if (currentPath.includes('/payments')) return 'payments';
			if (currentPath.includes('/admin')) return 'admin';
			return 'info';
		})()
	);

	const adminSubItems = [
		{ href: 'admin/join-requests', label: 'Join Requests' },
		{ href: 'admin/users', label: 'Users' },
		{ href: 'admin/fees', label: 'Fees' },
		{ href: 'admin/charges', label: 'Charges' },
		{ href: 'admin/stripe', label: 'Stripe' }
	];

	$effect(() => {
		if (clubId) {
			adminDropdownOpen = false;
			loadClub();
		}
	});

	$effect(() => {
		if (isMember && clubId) {
			loadStripeStatus();
		}
	});

	async function loadClub() {
		loading = true;
		error = '';
		clubStore.set({ club: null, loading: true, error: '' });

		try {
			const response = await serverCall<DataResponse<ClubWithSoaring>>(`/clubs/${clubId}`);
			club = response.data;
			clubStore.set({ club, loading: false, error: '' });
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load club: ${errorMessage}`;
			logger.error('Error loading club: {error}', { error: err });
			clubStore.set({ club: null, loading: false, error });
		} finally {
			loading = false;
		}
	}

	async function loadStripeStatus() {
		clubHasPayments = false;
		const requestedClubId = clubId;
		try {
			const response = await serverCall<DataResponse<StripeConnectStatusView>>(
				`/clubs/${requestedClubId}/stripe/status`
			);
			if (clubId === requestedClubId) {
				clubHasPayments = response.data.chargesEnabled;
			}
		} catch {
			if (clubId === requestedClubId) {
				clubHasPayments = false;
			}
		}
	}

	function closeAdminDropdown() {
		adminDropdownOpen = false;
	}

	let { children } = $props();
</script>

<svelte:window onclick={closeAdminDropdown} />

<div class="max-w-8xl container mx-auto space-y-6 p-4">
	<!-- Club Header -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<Progress class="h-8 w-8" />
				<span class="text-lg">Loading club...</span>
			</div>
		</div>
	{:else if error}
		<div class="alert preset-filled-error-500">
			<div class="alert-message">
				<h3 class="h3">Error Loading Club</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn preset-filled" onclick={loadClub}>Try Again</button>
				</div>
			</div>
		</div>
	{:else if club}
		<header class="card p-6">
			<div class="flex flex-wrap items-center gap-3">
				<Building class="h-7 w-7 text-primary-500" />
				<h1 class="h1">{club.name}</h1>
				{#if club.isSoaring}
					<span
						class="inline-flex items-center gap-1.5 rounded-full bg-primary-500 px-3 py-1 text-sm text-white"
					>
						<Plane class="h-3.5 w-3.5" />
						Soaring Club
					</span>
				{/if}
				{#if club.homeBaseAirportIdent}
					<span class="text-surface-500-400-token text-sm">
						{club.homeBaseAirportIdent}
					</span>
				{/if}
			</div>

			<!-- Tab Navigation -->
			<nav
				class="mt-4 flex flex-wrap items-center gap-1 border-t border-surface-200 pt-4 dark:border-surface-700"
			>
				<a
					href={resolve(`/clubs/${clubId}`)}
					class="btn btn-sm {activeTab === 'info' ? 'preset-filled-primary-500' : 'preset-tonal'}"
				>
					<Info class="h-4 w-4" />
					Club Info
				</a>

				{#if isMember}
					<a
						href={resolve(`/clubs/${clubId}/operations`)}
						class="btn btn-sm {activeTab === 'operations'
							? 'preset-filled-primary-500'
							: 'preset-tonal'}"
					>
						<ClipboardList class="h-4 w-4" />
						Operations
					</a>

					<a
						href={resolve(`/clubs/${clubId}/members`)}
						class="btn btn-sm {activeTab === 'members'
							? 'preset-filled-primary-500'
							: 'preset-tonal'}"
					>
						<Users class="h-4 w-4" />
						Members
					</a>

					{#if clubHasPayments}
						<a
							href={resolve('/payments')}
							class="btn btn-sm {activeTab === 'payments'
								? 'preset-filled-primary-500'
								: 'preset-tonal'}"
						>
							<CreditCard class="h-4 w-4" />
							My Payments
						</a>
					{/if}

					<!-- Admin dropdown (club admins and site admins only) -->
					{#if isClubAdmin}
						<div class="relative">
							<button
								class="btn btn-sm {activeTab === 'admin'
									? 'preset-filled-primary-500'
									: 'preset-tonal'}"
								onclick={(e) => {
									e.stopPropagation();
									adminDropdownOpen = !adminDropdownOpen;
								}}
							>
								<Shield class="h-4 w-4" />
								Admin
								<ChevronDown class="h-3 w-3" />
							</button>

							{#if adminDropdownOpen}
								<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
								<div
									class="absolute top-full left-0 z-10 mt-1 min-w-[160px] rounded-lg border border-surface-200 bg-surface-50 py-1 shadow-lg dark:border-surface-700 dark:bg-surface-900"
									onclick={(e) => e.stopPropagation()}
								>
									{#each adminSubItems as item (item.href)}
										<a
											href={resolve(`/clubs/${clubId}/${item.href}`)}
											class="block px-4 py-2 text-sm hover:bg-surface-200 dark:hover:bg-surface-700 {currentPath.includes(
												item.href
											)
												? 'font-semibold text-primary-500'
												: ''}"
											onclick={closeAdminDropdown}
										>
											{item.label}
										</a>
									{/each}
								</div>
							{/if}
						</div>
					{/if}
				{/if}
			</nav>
		</header>

		{@render children()}
	{/if}
</div>
