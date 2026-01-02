<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { UserPlus, ArrowLeft, Check, X } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import type { User } from '$lib/types';

	interface Club {
		id: string;
		name: string;
	}

	let club = $state<Club | null>(null);
	let pilots = $state<User[]>([]);
	let loadingClub = $state(true);
	let loadingPilots = $state(true);
	let error = $state('');
	let showAddModal = $state(false);

	// Add pilot form state
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

	onMount(async () => {
		if (clubId) {
			await loadClub();
			await loadPilots();
		}
	});

	async function loadClub() {
		loadingClub = true;
		error = '';

		try {
			const response = await serverCall<Club>(`/clubs/${clubId}`);
			club = response;
		} catch (err) {
			console.error('Error loading club:', err);
			error = err instanceof Error ? err.message : 'Failed to load club';
		} finally {
			loadingClub = false;
		}
	}

	async function loadPilots() {
		loadingPilots = true;

		try {
			const response = await serverCall<{ pilots: User[] }>(`/clubs/${clubId}/pilots`);
			pilots = response.pilots || [];
		} catch (err) {
			console.error('Error loading pilots:', err);
			error = err instanceof Error ? err.message : 'Failed to load pilots';
		} finally {
			loadingPilots = false;
		}
	}

	function openAddModal() {
		// Reset form
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

	async function handleAddPilot() {
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

			// Reload pilots list
			await loadPilots();

			// Close modal
			closeAddModal();
		} catch (err) {
			console.error('Error adding pilot:', err);
			formError = err instanceof Error ? err.message : 'Failed to add pilot';
		} finally {
			submitting = false;
		}
	}

	function goBack() {
		goto(`/clubs/${clubId}`);
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club'} Pilots - Aircraft Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-6xl p-4">
	<!-- Header -->
	<header class="mb-6 space-y-4">
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-4">
				<button onclick={goBack} class="preset-tonal-surface-500 btn btn-sm" title="Back to club">
					<ArrowLeft class="h-4 w-4" />
				</button>
				<div>
					<h1 class="text-3xl font-bold">Pilots</h1>
					{#if club}
						<p class="text-surface-600-300-token">{club.name}</p>
					{/if}
				</div>
			</div>

			{#if userBelongsToClub}
				<button onclick={openAddModal} class="btn preset-filled-primary-500">
					<UserPlus class="mr-2 h-5 w-5" />
					Add Pilot
				</button>
			{/if}
		</div>
	</header>

	<!-- Loading State -->
	{#if loadingClub || loadingPilots}
		<div class="flex items-center justify-center py-12">
			<div class="h-12 w-12 animate-spin rounded-full border-b-2 border-primary-500"></div>
		</div>
	{:else if error}
		<!-- Error State -->
		<div class="alert preset-filled-error-500">
			<p>{error}</p>
		</div>
	{:else if !userBelongsToClub}
		<!-- Not Authorized -->
		<div class="alert preset-filled-warning-500">
			<p class="font-semibold">Access Restricted</p>
			<p>You must be a member of this club to view pilots.</p>
		</div>
	{:else if pilots.length === 0}
		<!-- Empty State -->
		<div class="card p-8 text-center">
			<p class="text-surface-600-300-token mb-4 text-lg">No pilots found for this club.</p>
			<button onclick={openAddModal} class="btn preset-filled-primary-500">
				<UserPlus class="mr-2 h-5 w-5" />
				Add First Pilot
			</button>
		</div>
	{:else}
		<!-- Pilots List -->
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
							{#each pilots as pilot (pilot.id)}
								<tr>
									<td class="font-medium">
										{pilot.firstName}
										{pilot.lastName}
									</td>
									<td>
										{#if pilot.isLicensed}
											<Check class="h-5 w-5 text-success-500" />
										{:else}
											<X class="h-5 w-5 text-surface-400" />
										{/if}
									</td>
									<td>
										{#if pilot.isInstructor}
											<Check class="h-5 w-5 text-success-500" />
										{:else}
											<X class="h-5 w-5 text-surface-400" />
										{/if}
									</td>
									<td>
										{#if pilot.isTowPilot}
											<Check class="h-5 w-5 text-success-500" />
										{:else}
											<X class="h-5 w-5 text-surface-400" />
										{/if}
									</td>
									<td>
										{#if pilot.isExaminer}
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
				{#each pilots as pilot (pilot.id)}
					<div class="card p-4">
						<div class="mb-3 text-lg font-medium">
							{pilot.firstName}
							{pilot.lastName}
						</div>

						<div class="grid grid-cols-2 gap-3 text-sm">
							<div class="flex items-center gap-2">
								{#if pilot.isLicensed}
									<Check class="h-5 w-5 text-success-500" />
								{:else}
									<X class="h-5 w-5 text-surface-400" />
								{/if}
								<span class="text-surface-600-300-token">Licensed</span>
							</div>
							<div class="flex items-center gap-2">
								{#if pilot.isInstructor}
									<Check class="h-5 w-5 text-success-500" />
								{:else}
									<X class="h-5 w-5 text-surface-400" />
								{/if}
								<span class="text-surface-600-300-token">Instructor</span>
							</div>
							<div class="flex items-center gap-2">
								{#if pilot.isTowPilot}
									<Check class="h-5 w-5 text-success-500" />
								{:else}
									<X class="h-5 w-5 text-surface-400" />
								{/if}
								<span class="text-surface-600-300-token">Tow Pilot</span>
							</div>
							<div class="flex items-center gap-2">
								{#if pilot.isExaminer}
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
</div>

<!-- Add Pilot Modal -->
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
			aria-labelledby="add-pilot-heading"
			tabindex="-1"
		>
			<div class="flex items-center justify-between">
				<h2 id="add-pilot-heading" class="text-xl font-bold">Add Pilot</h2>
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
						onclick={handleAddPilot}
						class="btn preset-filled-primary-500"
						disabled={submitting}
					>
						{submitting ? 'Adding...' : 'Add Pilot'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
