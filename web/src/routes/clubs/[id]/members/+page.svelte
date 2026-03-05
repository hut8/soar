<script lang="ts">
	import { onMount, getContext } from 'svelte';
	import { page } from '$app/stores';
	import { UserPlus, Check, X } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import type { User, ClubWithSoaring, DataListResponse } from '$lib/types';
	import { getLogger } from '$lib/logging';
	import type { Writable } from 'svelte/store';

	const logger = getLogger(['soar', 'ClubMembers']);

	interface LayoutData {
		club: ClubWithSoaring | null;
		loading: boolean;
		error: string;
	}

	const clubStore = getContext<Writable<LayoutData>>('clubLayout');

	let members = $state<User[]>([]);
	let loadingMembers = $state(true);
	let error = $state('');
	let showAddModal = $state(false);

	// Add member form state
	let formFirstName = $state('');
	let formLastName = $state('');
	let formIsLicensed = $state(false);
	let formIsInstructor = $state(false);
	let formIsTowPilot = $state(false);
	let formIsExaminer = $state(false);
	let formError = $state('');
	let submitting = $state(false);

	let clubId = $derived($page.params.id || '');
	let userBelongsToClub = $derived($auth.isAuthenticated && $auth.user?.clubId === clubId);

	let club = $derived($clubStore.club);

	onMount(async () => {
		if (clubId) {
			await loadMembers();
		}
	});

	async function loadMembers() {
		loadingMembers = true;

		try {
			const response = await serverCall<DataListResponse<User>>(`/clubs/${clubId}/pilots`);
			members = response.data || [];
		} catch (err) {
			logger.error('Error loading members: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load members';
		} finally {
			loadingMembers = false;
		}
	}

	function openAddModal() {
		formFirstName = '';
		formLastName = '';
		formIsLicensed = false;
		formIsInstructor = false;
		formIsTowPilot = false;
		formIsExaminer = false;
		formError = '';
		showAddModal = true;
	}

	function closeAddModal() {
		showAddModal = false;
	}

	async function handleAddMember() {
		if (!formFirstName.trim() || !formLastName.trim()) {
			formError = 'First name and last name are required';
			return;
		}

		submitting = true;
		formError = '';

		try {
			await serverCall('/pilots', {
				method: 'POST',
				body: JSON.stringify({
					firstName: formFirstName.trim(),
					lastName: formLastName.trim(),
					isLicensed: formIsLicensed,
					isInstructor: formIsInstructor,
					isTowPilot: formIsTowPilot,
					isExaminer: formIsExaminer,
					clubId: clubId
				})
			});

			await loadMembers();
			closeAddModal();
		} catch (err) {
			logger.error('Error adding member: {error}', { error: err });
			formError = err instanceof Error ? err.message : 'Failed to add member';
		} finally {
			submitting = false;
		}
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club'} Members - SOAR</title>
</svelte:head>

<!-- Header -->
<header class="flex items-center justify-between">
	<div>
		<h2 class="text-2xl font-bold">Members</h2>
		{#if !loadingMembers}
			<p class="text-surface-600-300-token text-sm">
				{members.length} member{members.length === 1 ? '' : 's'}
			</p>
		{/if}
	</div>

	{#if userBelongsToClub}
		<button onclick={openAddModal} class="btn preset-filled-primary-500">
			<UserPlus class="mr-2 h-5 w-5" />
			Add Member
		</button>
	{/if}
</header>

<!-- Loading State -->
{#if loadingMembers}
	<div class="flex items-center justify-center py-12">
		<div class="h-12 w-12 animate-spin rounded-full border-b-2 border-primary-500"></div>
	</div>
{:else if error}
	<div class="alert preset-filled-error-500">
		<p>{error}</p>
	</div>
{:else if !userBelongsToClub}
	<div class="alert preset-filled-warning-500">
		<p class="font-semibold">Access Restricted</p>
		<p>You must be a member of this club to view members.</p>
	</div>
{:else if members.length === 0}
	<div class="card p-8 text-center">
		<p class="text-surface-600-300-token mb-4 text-lg">No members found for this club.</p>
		<button onclick={openAddModal} class="btn preset-filled-primary-500">
			<UserPlus class="mr-2 h-5 w-5" />
			Add First Member
		</button>
	</div>
{:else}
	<div class="overflow-hidden card">
		<!-- Desktop: Table -->
		<div class="hidden md:block">
			<div class="table-container">
				<table class="table-hover table">
					<thead>
						<tr>
							<th>Name</th>
							<th>Licensed</th>
							<th>Instructor</th>
							<th>Tow Pilot</th>
							<th>Examiner</th>
						</tr>
					</thead>
					<tbody>
						{#each members as member (member.id)}
							<tr>
								<td class="font-medium">
									{member.firstName}
									{member.lastName}
								</td>
								<td>
									{#if member.isLicensed}
										<Check class="h-5 w-5 text-success-500" />
									{:else}
										<X class="h-5 w-5 text-surface-400" />
									{/if}
								</td>
								<td>
									{#if member.isInstructor}
										<Check class="h-5 w-5 text-success-500" />
									{:else}
										<X class="h-5 w-5 text-surface-400" />
									{/if}
								</td>
								<td>
									{#if member.isTowPilot}
										<Check class="h-5 w-5 text-success-500" />
									{:else}
										<X class="h-5 w-5 text-surface-400" />
									{/if}
								</td>
								<td>
									{#if member.isExaminer}
										<Check class="h-5 w-5 text-success-500" />
									{:else}
										<X class="h-5 w-5 text-surface-400" />
									{/if}
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>

		<!-- Mobile: Cards -->
		<div class="space-y-4 p-4 md:hidden">
			{#each members as member (member.id)}
				<div class="card p-4">
					<div class="mb-3 text-lg font-medium">
						{member.firstName}
						{member.lastName}
					</div>

					<div class="grid grid-cols-2 gap-3 text-sm">
						<div class="flex items-center gap-2">
							{#if member.isLicensed}
								<Check class="h-5 w-5 text-success-500" />
							{:else}
								<X class="h-5 w-5 text-surface-400" />
							{/if}
							<span class="text-surface-600-300-token">Licensed</span>
						</div>
						<div class="flex items-center gap-2">
							{#if member.isInstructor}
								<Check class="h-5 w-5 text-success-500" />
							{:else}
								<X class="h-5 w-5 text-surface-400" />
							{/if}
							<span class="text-surface-600-300-token">Instructor</span>
						</div>
						<div class="flex items-center gap-2">
							{#if member.isTowPilot}
								<Check class="h-5 w-5 text-success-500" />
							{:else}
								<X class="h-5 w-5 text-surface-400" />
							{/if}
							<span class="text-surface-600-300-token">Tow Pilot</span>
						</div>
						<div class="flex items-center gap-2">
							{#if member.isExaminer}
								<Check class="h-5 w-5 text-success-500" />
							{:else}
								<X class="h-5 w-5 text-surface-400" />
							{/if}
							<span class="text-surface-600-300-token">Examiner</span>
						</div>
					</div>
				</div>
			{/each}
		</div>
	</div>
{/if}

<!-- Add Member Modal -->
{#if showAddModal}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
		onclick={closeAddModal}
		role="button"
		tabindex="0"
		onkeydown={(e) => e.key === 'Escape' && closeAddModal()}
	>
		<div
			class="m-4 w-full max-w-md space-y-4 card p-6"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.stopPropagation()}
			role="dialog"
			aria-labelledby="add-member-heading"
			tabindex="-1"
		>
			<div class="flex items-center justify-between">
				<h2 id="add-member-heading" class="text-xl font-bold">Add Member</h2>
				<button onclick={closeAddModal} class="preset-tonal-surface-500 btn btn-sm">
					<X class="h-4 w-4" />
				</button>
			</div>

			<div class="space-y-4">
				<div>
					<label for="first-name" class="label">
						<span>First Name <span class="text-error-500">*</span></span>
					</label>
					<input
						id="first-name"
						type="text"
						bind:value={formFirstName}
						placeholder="John"
						class="input"
						disabled={submitting}
						required
					/>
				</div>

				<div>
					<label for="last-name" class="label">
						<span>Last Name <span class="text-error-500">*</span></span>
					</label>
					<input
						id="last-name"
						type="text"
						bind:value={formLastName}
						placeholder="Doe"
						class="input"
						disabled={submitting}
						required
					/>
				</div>

				<div class="space-y-2">
					<label class="flex items-center space-x-2">
						<input
							type="checkbox"
							bind:checked={formIsLicensed}
							class="checkbox"
							disabled={submitting}
						/>
						<span>Licensed Pilot</span>
					</label>

					<label class="flex items-center space-x-2">
						<input
							type="checkbox"
							bind:checked={formIsInstructor}
							class="checkbox"
							disabled={submitting}
						/>
						<span>Instructor</span>
					</label>

					<label class="flex items-center space-x-2">
						<input
							type="checkbox"
							bind:checked={formIsTowPilot}
							class="checkbox"
							disabled={submitting}
						/>
						<span>Tow Pilot</span>
					</label>

					<label class="flex items-center space-x-2">
						<input
							type="checkbox"
							bind:checked={formIsExaminer}
							class="checkbox"
							disabled={submitting}
						/>
						<span>Examiner</span>
					</label>
				</div>

				{#if formError}
					<div class="alert preset-filled-error-500">
						<p>{formError}</p>
					</div>
				{/if}

				<div class="flex justify-end gap-2">
					<button
						onclick={closeAddModal}
						class="preset-tonal-surface-500 btn"
						disabled={submitting}
					>
						Cancel
					</button>
					<button
						onclick={handleAddMember}
						class="btn preset-filled-primary-500"
						disabled={submitting}
					>
						{submitting ? 'Adding...' : 'Add Member'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
