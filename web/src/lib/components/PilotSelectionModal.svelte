<script lang="ts">
	import { User, X } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';

	let {
		isOpen = $bindable(false),
		clubId,
		flightId,
		onSuccess = () => {}
	}: {
		isOpen?: boolean;
		clubId: string;
		flightId: string;
		onSuccess?: () => void;
	} = $props();

	interface Pilot {
		id: string;
		first_name: string;
		last_name: string;
		is_licensed: boolean;
		club_id: string | null;
	}

	let pilots: Pilot[] = $state([]);
	let selectedPilotId = $state('');
	let selectedRole = $state<'pilot' | 'student' | 'instructor' | 'tow_pilot'>('pilot');
	let loading = $state(false);
	let submitting = $state(false);
	let error = $state('');

	const roleOptions = [
		{ value: 'pilot', label: 'Pilot', description: 'Regular pilot' },
		{ value: 'student', label: 'Student', description: 'Student pilot under instruction' },
		{ value: 'instructor', label: 'Instructor', description: 'Flight instructor' },
		{ value: 'tow_pilot', label: 'Tow Pilot', description: 'Tow plane pilot' }
	];

	async function loadPilots() {
		if (!clubId) return;

		loading = true;
		error = '';

		try {
			const response = await serverCall<{ pilots: Pilot[] }>(`/clubs/${clubId}/pilots`);
			pilots = response.pilots || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load pilots: ${errorMessage}`;
			console.error('Error loading pilots:', err);
		} finally {
			loading = false;
		}
	}

	async function handleSubmit() {
		if (!selectedPilotId) {
			error = 'Please select a pilot';
			return;
		}

		submitting = true;
		error = '';

		try {
			await serverCall(`/flights/${flightId}/pilots`, {
				method: 'POST',
				body: JSON.stringify({
					pilot_id: selectedPilotId,
					is_tow_pilot: selectedRole === 'tow_pilot',
					is_student: selectedRole === 'student',
					is_instructor: selectedRole === 'instructor'
				})
			});

			// Reset and close
			selectedPilotId = '';
			selectedRole = 'pilot';
			isOpen = false;
			onSuccess();
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to add pilot: ${errorMessage}`;
			console.error('Error adding pilot to flight:', err);
		} finally {
			submitting = false;
		}
	}

	function handleClose() {
		isOpen = false;
		selectedPilotId = '';
		selectedRole = 'pilot';
		error = '';
	}

	// Load pilots when modal opens
	$effect(() => {
		if (isOpen && clubId) {
			loadPilots();
		}
	});
</script>

{#if isOpen}
	<div
		class="bg-opacity-50 fixed inset-0 z-50 flex items-center justify-center bg-black"
		onclick={(e) => {
			if (e.target === e.currentTarget) handleClose();
		}}
		role="dialog"
		aria-modal="true"
		tabindex="-1"
	>
		<div class="w-full max-w-2xl card p-6" onclick={(e) => e.stopPropagation()}>
			<header class="mb-6 flex items-center justify-between">
				<div class="flex items-center gap-3">
					<User class="h-6 w-6 text-primary-500" />
					<h2 class="h2">Add Pilot to Flight</h2>
				</div>
				<button
					onclick={handleClose}
					class="btn-icon btn-sm hover:bg-surface-200"
					aria-label="Close"
				>
					<X class="h-5 w-5" />
				</button>
			</header>

			{#if loading}
				<div class="space-y-4 py-12 text-center">
					<div
						class="mx-auto h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
					></div>
					<p class="text-surface-500-400-token">Loading pilots...</p>
				</div>
			{:else if error}
				<div class="mb-4 rounded border border-red-200 bg-red-50 p-4 text-red-600">
					{error}
				</div>
			{/if}

			<div class="space-y-6">
				<!-- Pilot Selection -->
				<div class="space-y-2">
					<label for="pilot-select" class="label">
						<span class="font-medium">Select Pilot</span>
					</label>
					<select
						id="pilot-select"
						bind:value={selectedPilotId}
						class="select"
						disabled={loading || pilots.length === 0}
					>
						<option value="">-- Select a pilot --</option>
						{#each pilots as pilot (pilot.id)}
							<option value={pilot.id}>
								{pilot.first_name}
								{pilot.last_name}
								{pilot.is_licensed ? '(Licensed)' : '(Unlicensed)'}
							</option>
						{/each}
					</select>
					{#if pilots.length === 0 && !loading}
						<p class="text-surface-500-400-token text-sm">No pilots available for this club.</p>
					{/if}
				</div>

				<!-- Role Selection -->
				<div class="space-y-2">
					<span class="label font-medium">Role</span>
					<div class="space-y-2">
						{#each roleOptions as option (option.value)}
							<label class="flex items-center gap-3 rounded p-3 hover:bg-surface-100">
								<input
									type="radio"
									name="role"
									value={option.value}
									bind:group={selectedRole}
									class="radio"
								/>
								<div class="flex flex-col">
									<span class="font-medium">{option.label}</span>
									<span class="text-surface-500-400-token text-sm">{option.description}</span>
								</div>
							</label>
						{/each}
					</div>
				</div>
			</div>

			<!-- Actions -->
			<footer class="mt-6 flex justify-end gap-3">
				<button onclick={handleClose} class="btn preset-tonal" disabled={submitting}>
					Cancel
				</button>
				<button
					onclick={handleSubmit}
					class="btn preset-filled-primary-500"
					disabled={!selectedPilotId || submitting || loading}
				>
					{submitting ? 'Adding...' : 'Add Pilot'}
				</button>
			</footer>
		</div>
	</div>
{/if}
