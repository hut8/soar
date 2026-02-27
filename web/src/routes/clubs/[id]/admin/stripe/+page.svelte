<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		ArrowLeft,
		ExternalLink,
		CheckCircle,
		XCircle,
		AlertCircle,
		Loader
	} from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import type { ClubView } from '$lib/types/generated/ClubView';
	import type { DataResponse } from '$lib/types';

	interface StripeConnectStatus {
		connected: boolean;
		onboardingComplete: boolean;
		chargesEnabled: boolean;
		payoutsEnabled: boolean;
		detailsSubmitted: boolean;
		stripeAccountId: string | null;
	}

	let club = $state<ClubView | null>(null);
	let stripeStatus = $state<StripeConnectStatus | null>(null);
	let loading = $state(true);
	let error = $state('');
	let actionLoading = $state(false);

	let clubId = $derived($page.params.id || '');
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);

	onMount(async () => {
		if (clubId) {
			await Promise.all([loadClub(), loadStripeStatus()]);
		}
	});

	async function loadClub() {
		try {
			const response = await serverCall<DataResponse<ClubView>>(`/clubs/${clubId}`);
			club = response.data;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load club';
		}
	}

	async function loadStripeStatus() {
		loading = true;
		try {
			const response = await serverCall<DataResponse<StripeConnectStatus>>(
				`/clubs/${clubId}/stripe/status`
			);
			stripeStatus = response.data;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load Stripe status';
		} finally {
			loading = false;
		}
	}

	async function startOnboarding() {
		actionLoading = true;
		error = '';
		try {
			const response = await serverCall<DataResponse<{ url: string }>>(
				`/clubs/${clubId}/stripe/onboard`,
				{ method: 'POST' }
			);
			window.location.href = response.data.url;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to start Stripe onboarding';
			actionLoading = false;
		}
	}

	async function openDashboard() {
		actionLoading = true;
		error = '';
		try {
			const response = await serverCall<DataResponse<{ url: string }>>(
				`/clubs/${clubId}/stripe/dashboard`,
				{ method: 'POST' }
			);
			window.open(response.data.url, '_blank');
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to open Stripe dashboard';
		} finally {
			actionLoading = false;
		}
	}
</script>

<div class="container mx-auto max-w-4xl p-4">
	<div class="mb-6 flex items-center gap-4">
		<button onclick={() => goto(`/clubs/${clubId}`)} class="variant-ghost-surface btn p-2">
			<ArrowLeft class="h-5 w-5" />
		</button>
		<h1 class="h2">Stripe Connect</h1>
	</div>

	{#if loading}
		<div class="flex justify-center py-8">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
		</div>
	{:else if error}
		<div class="alert variant-filled-error mb-4">
			<AlertCircle class="h-5 w-5" />
			<div>{error}</div>
		</div>
	{/if}

	{#if club && stripeStatus}
		<div class="mb-6 card p-4">
			<h2 class="mb-2 h3">{club.name}</h2>
			<p class="text-sm opacity-75">Manage Stripe payment processing for your club</p>
		</div>

		{#if !stripeStatus.connected}
			<!-- Not connected -->
			<div class="card p-8 text-center">
				<h3 class="mb-4 h3">Connect with Stripe</h3>
				<p class="mb-6 opacity-75">
					Connect your club to Stripe to accept payments from members for tow charges, membership
					dues, and more.
				</p>
				<button
					onclick={startOnboarding}
					class="variant-filled-primary btn btn-lg"
					disabled={actionLoading || !userBelongsToClub}
				>
					{#if actionLoading}
						<Loader class="h-5 w-5 animate-spin" />
					{/if}
					<span>Connect with Stripe</span>
					<ExternalLink class="h-4 w-4" />
				</button>
			</div>
		{:else if !stripeStatus.onboardingComplete}
			<!-- Onboarding in progress -->
			<div class="card p-8 text-center">
				<AlertCircle class="mx-auto mb-4 h-12 w-12 text-warning-500" />
				<h3 class="mb-4 h3">Complete Stripe Onboarding</h3>
				<p class="mb-6 opacity-75">
					Your Stripe account setup is incomplete. Click below to continue the onboarding process.
				</p>
				<button
					onclick={startOnboarding}
					class="variant-filled-warning btn btn-lg"
					disabled={actionLoading || !userBelongsToClub}
				>
					{#if actionLoading}
						<Loader class="h-5 w-5 animate-spin" />
					{/if}
					<span>Continue Onboarding</span>
					<ExternalLink class="h-4 w-4" />
				</button>
			</div>
		{:else}
			<!-- Fully connected -->
			<div class="space-y-4">
				<div class="card p-6">
					<h3 class="mb-4 h4">Account Status</h3>
					<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
						<div class="flex items-center gap-2">
							{#if stripeStatus.chargesEnabled}
								<CheckCircle class="h-5 w-5 text-success-500" />
							{:else}
								<XCircle class="h-5 w-5 text-error-500" />
							{/if}
							<span>Charges {stripeStatus.chargesEnabled ? 'Enabled' : 'Disabled'}</span>
						</div>
						<div class="flex items-center gap-2">
							{#if stripeStatus.payoutsEnabled}
								<CheckCircle class="h-5 w-5 text-success-500" />
							{:else}
								<XCircle class="h-5 w-5 text-error-500" />
							{/if}
							<span>Payouts {stripeStatus.payoutsEnabled ? 'Enabled' : 'Disabled'}</span>
						</div>
						<div class="flex items-center gap-2">
							{#if stripeStatus.detailsSubmitted}
								<CheckCircle class="h-5 w-5 text-success-500" />
							{:else}
								<XCircle class="h-5 w-5 text-error-500" />
							{/if}
							<span>Details {stripeStatus.detailsSubmitted ? 'Submitted' : 'Pending'}</span>
						</div>
					</div>
				</div>

				<div class="card p-6">
					<h3 class="mb-4 h4">Stripe Dashboard</h3>
					<p class="mb-4 opacity-75">
						View your Stripe dashboard to manage payouts, see transaction history, and configure
						settings.
					</p>
					<button
						onclick={openDashboard}
						class="variant-filled-primary btn"
						disabled={actionLoading || !userBelongsToClub}
					>
						{#if actionLoading}
							<Loader class="h-5 w-5 animate-spin" />
						{/if}
						<span>Open Stripe Dashboard</span>
						<ExternalLink class="h-4 w-4" />
					</button>
				</div>
			</div>
		{/if}
	{/if}
</div>
