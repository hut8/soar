<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { DollarSign, Plus, ArrowLeft, Trash2, Edit2, AlertCircle } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { getLogger } from '$lib/logging';
	import type { ClubView } from '$lib/types/generated/ClubView';
	import type { TowFeeView } from '$lib/types/generated/TowFeeView';

	const logger = getLogger(['soar', 'ClubTowFeesPage']);

	let club = $state<ClubView | null>(null);
	let towFees = $state<TowFeeView[]>([]);
	let loadingClub = $state(true);
	let loadingFees = $state(true);
	let error = $state('');
	let showAddModal = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);

	// Form state
	let formMaxAltitude = $state<number | null>(null);
	let formCost = $state('');
	let formError = $state('');
	let submitting = $state(false);

	// Edit/Delete state
	let editingFee = $state<TowFeeView | null>(null);
	let deletingFee = $state<TowFeeView | null>(null);

	let clubId = $derived($page.params.id || '');
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);

	// Check if there's already a fallback tier (NULL max_altitude)
	let hasFallbackTier = $derived(towFees.some((fee) => fee.maxAltitude === null));

	onMount(async () => {
		if (clubId) {
			await loadClub();
			await loadTowFees();
		}
	});

	async function loadClub() {
		loadingClub = true;
		error = '';

		try {
			const response = await serverCall<{ data: ClubView }>(`/clubs/${clubId}`);
			club = response.data;
		} catch (err) {
			logger.error('Error loading club: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load club';
		} finally {
			loadingClub = false;
		}
	}

	async function loadTowFees() {
		loadingFees = true;

		try {
			const response = await serverCall<{ data: TowFeeView[] }>(`/clubs/${clubId}/tow-fees`);
			towFees = response.data || [];
		} catch (err) {
			logger.error('Error loading tow fees: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load tow fees';
		} finally {
			loadingFees = false;
		}
	}

	function openAddModal() {
		// Reset form
		formMaxAltitude = null;
		formCost = '';
		formError = '';
		showAddModal = true;
	}

	function closeAddModal() {
		showAddModal = false;
	}

	function openEditModal(fee: TowFeeView) {
		editingFee = fee;
		formMaxAltitude = fee.maxAltitude;
		formCost = fee.cost;
		formError = '';
		showEditModal = true;
	}

	function closeEditModal() {
		showEditModal = false;
		editingFee = null;
	}

	function openDeleteModal(fee: TowFeeView) {
		deletingFee = fee;
		showDeleteModal = true;
	}

	function closeDeleteModal() {
		showDeleteModal = false;
		deletingFee = null;
	}

	async function handleAddTowFee() {
		if (!formCost.trim()) {
			formError = 'Cost is required';
			return;
		}

		const cost = parseFloat(formCost);
		if (isNaN(cost) || cost < 0) {
			formError = 'Cost must be a non-negative number';
			return;
		}

		if (formMaxAltitude !== null && formMaxAltitude <= 0) {
			formError = 'Altitude must be greater than 0';
			return;
		}

		submitting = true;
		formError = '';

		try {
			await serverCall(`/clubs/${clubId}/tow-fees`, {
				method: 'POST',
				body: JSON.stringify({
					maxAltitude: formMaxAltitude,
					cost: formCost
				})
			});

			await loadTowFees();
			closeAddModal();
		} catch (err) {
			logger.error('Error adding tow fee: {error}', { error: err });
			formError = err instanceof Error ? err.message : 'Failed to add tow fee';
		} finally {
			submitting = false;
		}
	}

	async function handleEditTowFee() {
		if (!editingFee) return;

		if (!formCost.trim()) {
			formError = 'Cost is required';
			return;
		}

		const cost = parseFloat(formCost);
		if (isNaN(cost) || cost < 0) {
			formError = 'Cost must be a non-negative number';
			return;
		}

		if (formMaxAltitude !== null && formMaxAltitude <= 0) {
			formError = 'Altitude must be greater than 0';
			return;
		}

		submitting = true;
		formError = '';

		try {
			await serverCall(`/clubs/${clubId}/tow-fees/${editingFee.id}`, {
				method: 'PUT',
				body: JSON.stringify({
					maxAltitude: formMaxAltitude,
					cost: formCost
				})
			});

			await loadTowFees();
			closeEditModal();
		} catch (err) {
			logger.error('Error updating tow fee: {error}', { error: err });
			formError = err instanceof Error ? err.message : 'Failed to update tow fee';
		} finally {
			submitting = false;
		}
	}

	async function handleDeleteTowFee() {
		if (!deletingFee) return;

		submitting = true;

		try {
			await serverCall(`/clubs/${clubId}/tow-fees/${deletingFee.id}`, {
				method: 'DELETE'
			});

			await loadTowFees();
			closeDeleteModal();
		} catch (err) {
			logger.error('Error deleting tow fee: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to delete tow fee';
		} finally {
			submitting = false;
		}
	}

	function formatAltitude(altitude: number | null): string {
		if (altitude === null) {
			return 'Unlimited';
		}
		return `${altitude.toLocaleString()} ft AGL`;
	}

	function formatCost(cost: string): string {
		const num = parseFloat(cost);
		if (isNaN(num)) return cost;
		return `$${num.toFixed(2)}`;
	}
</script>

<div class="container mx-auto max-w-4xl p-4">
	<!-- Header -->
	<div class="mb-6 flex items-center gap-4">
		<button onclick={() => goto(`/clubs/${clubId}`)} class="variant-ghost-surface btn p-2">
			<ArrowLeft class="h-5 w-5" />
		</button>
		<h1 class="h2">Tow Fee Management</h1>
	</div>

	{#if loadingClub}
		<div class="flex justify-center py-8">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
			></div>
		</div>
	{:else if error}
		<div class="alert variant-filled-error">
			<AlertCircle class="h-5 w-5" />
			<div>
				<h3 class="h3">Error</h3>
				<p>{error}</p>
			</div>
		</div>
	{:else if club}
		<div class="mb-6 card p-4">
			<div class="flex items-center justify-between">
				<div>
					<h2 class="h3">{club.name}</h2>
					<p class="text-sm opacity-75">Configure tow fees based on altitude tiers</p>
				</div>
				{#if userBelongsToClub}
					<button onclick={openAddModal} class="variant-filled-primary btn">
						<Plus class="h-4 w-4" />
						<span>Add Tier</span>
					</button>
				{/if}
			</div>
		</div>

		<!-- Tow Fees Table -->
		{#if loadingFees}
			<div class="flex justify-center py-8">
				<div
					class="h-8 w-8 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
				></div>
			</div>
		{:else if towFees.length === 0}
			<div class="card p-8 text-center">
				<DollarSign class="mx-auto mb-4 h-12 w-12 opacity-50" />
				<p class="mb-2 text-lg">No tow fees configured</p>
				<p class="text-sm opacity-75">Add your first tow fee tier to get started</p>
			</div>
		{:else}
			<div class="card">
				<div class="table-container">
					<table class="table-hover table">
						<thead>
							<tr>
								<th>Maximum Altitude</th>
								<th>Cost</th>
								<th>Updated</th>
								{#if userBelongsToClub}
									<th class="text-right">Actions</th>
								{/if}
							</tr>
						</thead>
						<tbody>
							{#each towFees as fee (fee.id)}
								<tr>
									<td>
										{formatAltitude(fee.maxAltitude)}
										{#if fee.maxAltitude === null}
											<span class="variant-soft-primary ml-2 badge">Fallback</span>
										{/if}
									</td>
									<td class="font-semibold">{formatCost(fee.cost)}</td>
									<td class="text-sm opacity-75">
										{new Date(fee.updatedAt).toLocaleDateString()}
									</td>
									{#if userBelongsToClub}
										<td class="text-right">
											<div class="flex justify-end gap-2">
												<button
													onclick={() => openEditModal(fee)}
													class="variant-ghost-surface btn btn-sm"
													title="Edit"
												>
													<Edit2 class="h-4 w-4" />
												</button>
												<button
													onclick={() => openDeleteModal(fee)}
													class="variant-ghost-error btn btn-sm"
													title="Delete"
												>
													<Trash2 class="h-4 w-4" />
												</button>
											</div>
										</td>
									{/if}
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			</div>

			<!-- Info about fallback tier -->
			{#if !hasFallbackTier}
				<div class="alert variant-ghost-warning mt-4">
					<AlertCircle class="h-5 w-5" />
					<div class="alert-message">
						<h3 class="h4">No Fallback Tier</h3>
						<p>
							Consider adding a fallback tier (with no maximum altitude) for tows above your highest
							tier.
						</p>
					</div>
				</div>
			{/if}
		{/if}
	{/if}
</div>

<!-- Add Tow Fee Modal -->
{#if showAddModal}
	<div class="modal-backdrop" onclick={closeAddModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Add Tow Fee Tier</h3>

			<label class="label mb-4">
				<span>Maximum Altitude (ft AGL)</span>
				<input
					type="number"
					class="input"
					bind:value={formMaxAltitude}
					placeholder="Leave empty for fallback tier"
					min="1"
					step="100"
				/>
				<p class="mt-1 text-sm opacity-75">
					Leave empty to create a fallback tier for tows above the highest altitude
				</p>
			</label>

			<label class="label mb-4">
				<span>Cost ($)</span>
				<input
					type="number"
					class="input"
					bind:value={formCost}
					placeholder="0.00"
					min="0"
					step="0.01"
					required
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
				<button onclick={handleAddTowFee} class="variant-filled-primary btn" disabled={submitting}>
					{submitting ? 'Adding...' : 'Add Tier'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Tow Fee Modal -->
{#if showEditModal && editingFee}
	<div class="modal-backdrop" onclick={closeEditModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Edit Tow Fee Tier</h3>

			<label class="label mb-4">
				<span>Maximum Altitude (ft AGL)</span>
				<input
					type="number"
					class="input"
					bind:value={formMaxAltitude}
					placeholder="Leave empty for fallback tier"
					min="1"
					step="100"
				/>
				<p class="mt-1 text-sm opacity-75">
					Leave empty to create a fallback tier for tows above the highest altitude
				</p>
			</label>

			<label class="label mb-4">
				<span>Cost ($)</span>
				<input
					type="number"
					class="input"
					bind:value={formCost}
					placeholder="0.00"
					min="0"
					step="0.01"
					required
				/>
			</label>

			{#if formError}
				<div class="alert variant-filled-error mb-4">
					<p>{formError}</p>
				</div>
			{/if}

			<div class="flex justify-end gap-2">
				<button onclick={closeEditModal} class="variant-ghost-surface btn" disabled={submitting}>
					Cancel
				</button>
				<button onclick={handleEditTowFee} class="variant-filled-primary btn" disabled={submitting}>
					{submitting ? 'Saving...' : 'Save Changes'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirmation Modal -->
{#if showDeleteModal && deletingFee}
	<div class="modal-backdrop" onclick={closeDeleteModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Delete Tow Fee Tier</h3>

			<p class="mb-4">
				Are you sure you want to delete the tow fee tier for
				<strong>{formatAltitude(deletingFee.maxAltitude)}</strong>
				at <strong>{formatCost(deletingFee.cost)}</strong>?
			</p>

			<div class="flex justify-end gap-2">
				<button onclick={closeDeleteModal} class="variant-ghost-surface btn" disabled={submitting}>
					Cancel
				</button>
				<button onclick={handleDeleteTowFee} class="variant-filled-error btn" disabled={submitting}>
					{submitting ? 'Deleting...' : 'Delete'}
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
