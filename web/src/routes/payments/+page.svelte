<script lang="ts">
	import { onMount } from 'svelte';
	import {
		DollarSign,
		CreditCard,
		Clock,
		CheckCircle,
		XCircle,
		AlertCircle,
		Loader
	} from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import type { DataListResponse, DataResponse, PaymentView, CheckoutResponse } from '$lib/types';

	let payments = $state<PaymentView[]>([]);
	let loading = $state(true);
	let error = $state('');
	let checkingOut = $state<string | null>(null);

	onMount(async () => {
		await loadPayments();
	});

	async function loadPayments() {
		loading = true;
		error = '';
		try {
			const response = await serverCall<DataListResponse<PaymentView>>('/payments');
			payments = response.data || [];
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load payments';
		} finally {
			loading = false;
		}
	}

	async function handlePay(paymentId: string) {
		checkingOut = paymentId;
		error = '';
		try {
			const response = await serverCall<DataResponse<CheckoutResponse>>(
				`/payments/${paymentId}/checkout`,
				{ method: 'POST' }
			);
			window.location.href = response.data.checkoutUrl;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to start checkout';
			checkingOut = null;
		}
	}

	function formatCurrency(cents: number): string {
		return `$${(cents / 100).toFixed(2)}`;
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString();
	}

	function getStatusColor(status: string): string {
		switch (status) {
			case 'succeeded':
				return 'text-success-500';
			case 'failed':
			case 'canceled':
				return 'text-error-500';
			case 'pending':
				return 'text-warning-500';
			default:
				return 'text-surface-500';
		}
	}

	function formatPaymentType(type: string): string {
		switch (type) {
			case 'tow_charge':
				return 'Tow Charge';
			case 'membership_dues':
				return 'Membership Dues';
			case 'other':
				return 'Other';
			default:
				return type;
		}
	}
</script>

<div class="container mx-auto max-w-4xl p-4">
	<h1 class="mb-6 h2">My Payments</h1>

	{#if error}
		<div class="alert variant-filled-error mb-4">
			<AlertCircle class="h-5 w-5" />
			<div>{error}</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center py-8">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
		</div>
	{:else if payments.length === 0}
		<div class="card p-8 text-center">
			<DollarSign class="mx-auto mb-4 h-12 w-12 opacity-50" />
			<p class="mb-2 text-lg">No payments</p>
			<p class="text-sm opacity-75">You don't have any charges or payments yet</p>
		</div>
	{:else}
		<div class="space-y-4">
			{#each payments as payment (payment.id)}
				<div class="card p-4">
					<div class="flex items-center justify-between">
						<div class="flex-1">
							<div class="flex items-center gap-3">
								<span class="text-lg font-semibold">
									{formatCurrency(payment.amountCents)}
								</span>
								<span class="flex items-center gap-1 text-sm {getStatusColor(payment.status)}">
									{#if payment.status === 'succeeded'}
										<CheckCircle class="h-4 w-4" />
									{:else if payment.status === 'failed' || payment.status === 'canceled'}
										<XCircle class="h-4 w-4" />
									{:else if payment.status === 'pending'}
										<Clock class="h-4 w-4" />
									{:else}
										<AlertCircle class="h-4 w-4" />
									{/if}
									{payment.status}
								</span>
							</div>
							<div class="mt-1 flex items-center gap-4 text-sm opacity-75">
								<span>{formatPaymentType(payment.paymentType)}</span>
								{#if payment.description}
									<span>{payment.description}</span>
								{/if}
								<span>{formatDate(payment.createdAt)}</span>
							</div>
						</div>
						{#if payment.status === 'pending'}
							<button
								onclick={() => handlePay(payment.id)}
								class="variant-filled-primary btn"
								disabled={checkingOut !== null}
							>
								{#if checkingOut === payment.id}
									<Loader class="h-4 w-4 animate-spin" />
								{:else}
									<CreditCard class="h-4 w-4" />
								{/if}
								<span>Pay</span>
							</button>
						{/if}
					</div>
				</div>
			{/each}
		</div>
	{/if}
</div>
