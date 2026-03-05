<script lang="ts">
	import { UserPlus, X } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';
	import { toaster } from '$lib/toaster';
	import type { User } from '$lib/types';

	const logger = getLogger(['soar', 'PilotAssignmentEditor']);

	interface PilotAssignment {
		pilotId: string;
		pilotName: string;
		role: string;
	}

	let {
		flightId = null,
		aircraftId,
		aircraftRegistration = '',
		members,
		isAirborne,
		onClose,
		onAssigned
	}: {
		flightId: string | null;
		aircraftId: string;
		aircraftRegistration: string;
		members: User[];
		isAirborne: boolean;
		onClose: () => void;
		onAssigned: () => void;
	} = $props();

	const roleOptions = [
		{ value: 'pilot', label: 'Pilot' },
		{ value: 'student', label: 'Student' },
		{ value: 'instructor', label: 'Instructor' },
		{ value: 'tow_pilot', label: 'Tow Pilot' }
	];

	let selectedPilotId = $state('');
	let selectedRole = $state('pilot');
	let submitting = $state(false);
	let error = $state('');

	// Load existing ground assignments from localStorage
	let groundAssignments = $state<PilotAssignment[]>([]);

	$effect(() => {
		if (!isAirborne) {
			const stored = localStorage.getItem(`soar:ground-pilots:${aircraftId}`);
			if (stored) {
				try {
					const parsed = JSON.parse(stored);
					groundAssignments = parsed.pilots || [];
				} catch {
					groundAssignments = [];
				}
			}
		}
	});

	function saveGroundAssignments(assignments: PilotAssignment[]) {
		localStorage.setItem(
			`soar:ground-pilots:${aircraftId}`,
			JSON.stringify({ pilots: assignments, updatedAt: new Date().toISOString() })
		);
		groundAssignments = assignments;
	}

	async function handleAssign() {
		if (!selectedPilotId) {
			error = 'Please select a member';
			return;
		}

		const pilot = members.find((m) => m.id === selectedPilotId);
		if (!pilot) return;

		error = '';

		if (isAirborne && flightId) {
			// Assign immediately to active flight
			submitting = true;
			try {
				await serverCall(`/flights/${flightId}/pilots`, {
					method: 'POST',
					body: JSON.stringify({
						pilot_id: selectedPilotId,
						isTowPilot: selectedRole === 'tow_pilot',
						is_student: selectedRole === 'student',
						isInstructor: selectedRole === 'instructor'
					})
				});
				toaster.success({ title: `${pilot.firstName} ${pilot.lastName} assigned to flight` });
				selectedPilotId = '';
				selectedRole = 'pilot';
				onAssigned();
			} catch (err) {
				const errorMessage = err instanceof Error ? err.message : 'Unknown error';
				error = `Failed to assign: ${errorMessage}`;
				logger.error('Error assigning pilot to flight: {error}', { error: err });
			} finally {
				submitting = false;
			}
		} else {
			// Store in localStorage for ground aircraft
			const assignment: PilotAssignment = {
				pilotId: selectedPilotId,
				pilotName: `${pilot.firstName} ${pilot.lastName}`,
				role: selectedRole
			};
			const updated = [...groundAssignments, assignment];
			saveGroundAssignments(updated);
			selectedPilotId = '';
			selectedRole = 'pilot';
			toaster.success({
				title: `${pilot.firstName} ${pilot.lastName} pre-assigned to ${aircraftRegistration || 'aircraft'}`
			});
		}
	}

	function removeGroundAssignment(index: number) {
		const updated = groundAssignments.filter((_, i) => i !== index);
		if (updated.length === 0) {
			localStorage.removeItem(`soar:ground-pilots:${aircraftId}`);
			groundAssignments = [];
		} else {
			saveGroundAssignments(updated);
		}
	}
</script>

<div
	class="space-y-3 rounded-lg border border-surface-200 bg-surface-50 p-3 dark:border-surface-700 dark:bg-surface-800"
>
	<div class="flex items-center justify-between">
		<span class="text-sm font-medium">
			{isAirborne ? 'Assign Pilot' : 'Pre-assign for Next Flight'}
		</span>
		<button onclick={onClose} class="btn-icon btn-sm">
			<X class="h-4 w-4" />
		</button>
	</div>

	<!-- Existing ground assignments -->
	{#if !isAirborne && groundAssignments.length > 0}
		<div class="space-y-1">
			{#each groundAssignments as assignment, i (assignment.pilotId)}
				<div
					class="flex items-center justify-between rounded bg-surface-100 px-2 py-1 text-sm dark:bg-surface-700"
				>
					<span>{assignment.pilotName} ({assignment.role})</span>
					<button onclick={() => removeGroundAssignment(i)} class="btn-icon btn-sm">
						<X class="h-3 w-3" />
					</button>
				</div>
			{/each}
			<p class="text-xs text-surface-500 italic">Will be assigned on next takeoff</p>
		</div>
	{/if}

	<!-- Assignment form -->
	<div class="flex flex-col gap-2">
		<select bind:value={selectedPilotId} class="select-sm select" disabled={submitting}>
			<option value="">-- Select member --</option>
			{#each members as member (member.id)}
				<option value={member.id}>
					{member.firstName}
					{member.lastName}
				</option>
			{/each}
		</select>

		<select bind:value={selectedRole} class="select-sm select" disabled={submitting}>
			{#each roleOptions as option (option.value)}
				<option value={option.value}>{option.label}</option>
			{/each}
		</select>

		{#if error}
			<p class="text-xs text-error-500">{error}</p>
		{/if}

		<button
			onclick={handleAssign}
			class="btn preset-filled-primary-500 btn-sm"
			disabled={!selectedPilotId || submitting}
		>
			<UserPlus class="h-3 w-3" />
			{submitting ? 'Assigning...' : isAirborne ? 'Assign' : 'Pre-assign'}
		</button>
	</div>
</div>
