<script lang="ts">
	import { X, Shield, MapPin, Info } from '@lucide/svelte';
	import type { Airspace } from '$lib/types';

	// Props
	let { showModal = $bindable(), selectedAirspace = $bindable() } = $props<{
		showModal: boolean;
		selectedAirspace: Airspace | null;
	}>();

	function closeModal() {
		showModal = false;
		selectedAirspace = null;
	}

	function getAirspaceClassDisplay(airspaceClass: string | null): string {
		if (!airspaceClass) return 'Unknown';
		return `Class ${airspaceClass}`;
	}

	function getAirspaceTypeDisplay(type: string): string {
		// Convert snake_case or PascalCase to Title Case
		return type
			.replace(/([A-Z])/g, ' $1')
			.replace(/_/g, ' ')
			.trim()
			.split(' ')
			.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
			.join(' ');
	}

	function getAirspaceColor(airspaceClass: string | null): string {
		switch (airspaceClass) {
			case 'A':
			case 'B':
			case 'C':
			case 'D':
				return 'error'; // Red - Controlled airspace
			case 'E':
				return 'warning'; // Amber - Class E
			case 'F':
			case 'G':
				return 'success'; // Green - Uncontrolled
			default:
				return 'secondary'; // Gray - Other/SUA
		}
	}
</script>

<!-- Airspace Modal -->
{#if showModal && selectedAirspace}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center bg-surface-950-50/50 pt-20"
		onclick={closeModal}
		onkeydown={(e) => e.key === 'Escape' && closeModal()}
		role="presentation"
	>
		<div
			class="max-h-[calc(90vh-5rem)] w-full max-w-2xl overflow-y-auto card bg-surface-50 text-surface-900 shadow-xl dark:bg-surface-900 dark:text-surface-50"
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => e.key === 'Escape' && closeModal()}
			role="dialog"
			aria-modal="true"
			aria-labelledby="airspace-modal-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between border-b border-surface-300 p-6 dark:border-surface-600"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex h-10 w-10 items-center justify-center rounded-full bg-blue-500 text-white"
					>
						<Shield size={24} />
					</div>
					<div>
						<h2 id="airspace-modal-title" class="text-xl font-bold">
							{selectedAirspace.properties.name}
						</h2>
						<p class="text-sm text-surface-600 dark:text-surface-400">
							{getAirspaceTypeDisplay(selectedAirspace.properties.airspace_type)}
						</p>
					</div>
				</div>
				<button class="preset-tonal-surface-500 btn btn-sm" onclick={closeModal}>
					<X size={20} />
				</button>
			</div>

			<div class="p-6">
				<div class="space-y-6">
					<!-- Airspace Classification -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Info size={20} />
							Classification
						</h3>

						<div class="space-y-3">
							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Airspace Class
									</dt>
									<dd class="mt-1">
										<span
											class="badge preset-filled-{getAirspaceColor(
												selectedAirspace.properties.airspace_class
											)}"
										>
											{getAirspaceClassDisplay(selectedAirspace.properties.airspace_class)}
										</span>
									</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">Type</dt>
									<dd class="mt-1 text-sm">
										{getAirspaceTypeDisplay(selectedAirspace.properties.airspace_type)}
									</dd>
								</div>
							</div>

							{#if selectedAirspace.properties.country_code}
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Country
									</dt>
									<dd class="mt-1 font-mono text-sm">
										{selectedAirspace.properties.country_code}
									</dd>
								</div>
							{/if}

							{#if selectedAirspace.properties.activity_type}
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Activity Type
									</dt>
									<dd class="mt-1 text-sm">
										{getAirspaceTypeDisplay(selectedAirspace.properties.activity_type)}
									</dd>
								</div>
							{/if}
						</div>
					</div>

					<!-- Altitude Limits -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<MapPin size={20} />
							Altitude Limits
						</h3>

						<div class="space-y-3">
							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Lower Limit
									</dt>
									<dd class="mt-1 font-mono text-sm">
										{selectedAirspace.properties.lower_limit}
									</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Upper Limit
									</dt>
									<dd class="mt-1 font-mono text-sm">
										{selectedAirspace.properties.upper_limit}
									</dd>
								</div>
							</div>
						</div>
					</div>

					<!-- Remarks -->
					{#if selectedAirspace.properties.remarks}
						<div class="space-y-4">
							<h3 class="text-lg font-semibold">Remarks</h3>
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<p class="text-sm whitespace-pre-wrap">{selectedAirspace.properties.remarks}</p>
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
