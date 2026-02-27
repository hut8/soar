<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import {
		ArrowLeft,
		Plus,
		AlertCircle,
		DollarSign,
		Clock,
		CheckCircle,
		XCircle
	} from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import type { ClubView } from '$lib/types/generated/ClubView';
	import type { UserView } from '$lib/types/generated/UserView';
	import type { DataResponse, DataListResponse, PaymentView } from '$lib/types';

	let club = $state<ClubView | null>(null);
	let charges = $state<PaymentView[]>([]);
	let members = $state<UserView[]>([]);
	let loading = $state(true);
	let error = $state('');
	let showAddModal = $state(false);
	let submitting = $state(false);

	// Form state
	let formUserId = $state('');
	let formAmount = $state('');
	let formPaymentType = $state('tow_charge');
	let formDescription = $state('');
	let formError = $state('');

	let clubId = $derived($page.params.id || '');
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);

	onMount(async () => {
		if (clubId) {
			await Promise.all([loadClub(), loadCharges(), loadMembers()]);
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

	async function loadCharges() {
		loading = true;
		try {
			const response = await serverCall<DataListResponse<PaymentView>>(`/clubs/${clubId}/charges`);
			charges = response.data || [];
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load charges';
		} finally {
			loading = false;
		}
	}

	async function loadMembers() {
		try {
			const response = await serverCall<DataListResponse<UserView>>(`/clubs/${clubId}/users`);
			members = response.data || [];
		} catch {
			// Members are optional for the page to function
		}
	}

	function openAddModal() {
		formUserId = '';
		formAmount = '';
		formPaymentType = 'tow_charge';
		formDescription = '';
		formError = '';
		showAddModal = true;
	}

	function closeAddModal() {
		showAddModal = false;
	}

	async function handleCreateCharge() {
		if (!formUserId) {
			formError = 'Please select a member';
			return;
		}

		const amount = parseFloat(formAmount);
		if (isNaN(amount) || amount <= 0) {
			formError = 'Amount must be greater than 0';
			return;
		}

		submitting = true;
		formError = '';

		try {
			await serverCall(`/clubs/${clubId}/charges`, {
				method: 'POST',
				body: JSON.stringify({
					userId: formUserId,
					amountCents: Math.round(amount * 100),
					paymentType: formPaymentType,
					description: formDescription || null
				})
			});
			await loadCharges();
			closeAddModal();
		} catch (err) {
			formError = err instanceof Error ? err.message : 'Failed to create charge';
		} finally {
			submitting = false;
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

	function getMemberName(userId: string): string {
		const member = members.find((m) => m.id === userId);
		if (member) {
			return `${member.firstName} ${member.lastName}`;
		}
		return userId.slice(0, 8) + '...';
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
	<div class="mb-6 flex items-center gap-4">
		<button onclick={() => goto(`/clubs/${clubId}`)} class="variant-ghost-surface btn p-2">
			<ArrowLeft class="h-5 w-5" />
		</button>
		<h1 class="h2">Charges</h1>
	</div>

	{#if error}
		<div class="alert variant-filled-error mb-4">
			<AlertCircle class="h-5 w-5" />
			<div>{error}</div>
		</div>
	{/if}

	{#if club}
		<div class="mb-6 card p-4">
			<div class="flex items-center justify-between">
				<div>
					<h2 class="h3">{club.name}</h2>
					<p class="text-sm opacity-75">Create and manage charges for club members</p>
				</div>
				{#if userBelongsToClub}
					<button onclick={openAddModal} class="variant-filled-primary btn">
						<Plus class="h-4 w-4" />
						<span>Create Charge</span>
					</button>
				{/if}
			</div>
		</div>
	{/if}

	{#if loading}
		<div class="flex justify-center py-8">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
		</div>
	{:else if charges.length === 0}
		<div class="card p-8 text-center">
			<DollarSign class="mx-auto mb-4 h-12 w-12 opacity-50" />
			<p class="mb-2 text-lg">No charges yet</p>
			<p class="text-sm opacity-75">Create a charge for a club member to get started</p>
		</div>
	{:else}
		<div class="card">
			<div class="table-container">
				<table class="table-hover table">
					<thead>
						<tr>
							<th>Member</th>
							<th>Type</th>
							<th>Amount</th>
							<th>Status</th>
							<th>Description</th>
							<th>Date</th>
						</tr>
					</thead>
					<tbody>
						{#each charges as charge (charge.id)}
							<tr>
								<td>{getMemberName(charge.userId)}</td>
								<td>{formatPaymentType(charge.paymentType)}</td>
								<td class="font-semibold">{formatCurrency(charge.amountCents)}</td>
								<td>
									<span class="flex items-center gap-1 {getStatusColor(charge.status)}">
										{#if charge.status === 'succeeded'}
											<CheckCircle class="h-4 w-4" />
										{:else if charge.status === 'failed' || charge.status === 'canceled'}
											<XCircle class="h-4 w-4" />
										{:else if charge.status === 'pending'}
											<Clock class="h-4 w-4" />
										{:else}
											<AlertCircle class="h-4 w-4" />
										{/if}
										{charge.status}
									</span>
								</td>
								<td class="max-w-[200px] truncate text-sm opacity-75">
									{charge.description || '-'}
								</td>
								<td class="text-sm opacity-75">
									{formatDate(charge.createdAt)}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}
</div>

{#if showAddModal}
	<div class="modal-backdrop" onclick={closeAddModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Create Charge</h3>

			<label class="label mb-4">
				<span>Member</span>
				<select class="select" bind:value={formUserId}>
					<option value="">Select a member</option>
					{#each members as member (member.id)}
						<option value={member.id}>{member.firstName} {member.lastName}</option>
					{/each}
				</select>
			</label>

			<label class="label mb-4">
				<span>Payment Type</span>
				<select class="select" bind:value={formPaymentType}>
					<option value="tow_charge">Tow Charge</option>
					<option value="membership_dues">Membership Dues</option>
					<option value="other">Other</option>
				</select>
			</label>

			<label class="label mb-4">
				<span>Amount ($)</span>
				<input
					type="number"
					class="input"
					bind:value={formAmount}
					placeholder="0.00"
					min="0.01"
					step="0.01"
					required
				/>
			</label>

			<label class="label mb-4">
				<span>Description (optional)</span>
				<input
					type="text"
					class="input"
					bind:value={formDescription}
					placeholder="e.g., Tow to 3000 ft"
				/>
			</label>

			{#if formError}
				<div class="alert variant-filled-error mb-4">
					<p>{formError}</p>
				</div>
			{/if}

			<div class="flex justify-end gap-2">
				<button onclick={closeAddModal} class="variant-ghost-surface btn" disabled={submitting}>
					Cancel
				</button>
				<button
					onclick={handleCreateCharge}
					class="variant-filled-primary btn"
					disabled={submitting}
				>
					{submitting ? 'Creating...' : 'Create Charge'}
				</button>
			</div>
		</div>
	</div>
{/if}

<style>
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		background: rgba(0, 0, 0, 0.5);
		z-index: 998;
	}

	.modal {
		position: fixed;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 999;
		max-height: 90vh;
		overflow-y: auto;
	}
</style>
