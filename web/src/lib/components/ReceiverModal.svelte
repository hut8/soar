<script lang="ts">
	import { X, Radio, MapPin, Info, Clock, ExternalLink } from '@lucide/svelte';
	import type { Receiver } from '$lib/types';

	// Props
	let { showModal = $bindable(), selectedReceiver = $bindable() } = $props<{
		showModal: boolean;
		selectedReceiver: Receiver | null;
	}>();

	function closeModal() {
		showModal = false;
		selectedReceiver = null;
	}

	function formatCoordinates(lat: number | null, lng: number | null): string {
		if (lat === null || lng === null) return 'Unknown';
		const latDir = lat >= 0 ? 'N' : 'S';
		const lngDir = lng >= 0 ? 'E' : 'W';
		return `${Math.abs(lat).toFixed(4)}\u00B0${latDir}, ${Math.abs(lng).toFixed(4)}\u00B0${lngDir}`;
	}

	function formatRelativeTime(dateStr: string | null): string {
		if (!dateStr) return 'Unknown';
		const date = new Date(dateStr);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMins = Math.floor(diffMs / 60000);
		const diffHours = Math.floor(diffMins / 60);
		const diffDays = Math.floor(diffHours / 24);

		if (diffMins < 1) return 'Just now';
		if (diffMins < 60) return `${diffMins}m ago`;
		if (diffHours < 24) return `${diffHours}h ago`;
		if (diffDays < 30) return `${diffDays}d ago`;
		return date.toLocaleDateString();
	}

	function formatDate(dateStr: string): string {
		return new Date(dateStr).toLocaleDateString(undefined, {
			year: 'numeric',
			month: 'short',
			day: 'numeric'
		});
	}

	function buildLocationString(receiver: Receiver): string {
		const parts: string[] = [];
		if (receiver.city) parts.push(receiver.city);
		if (receiver.region) parts.push(receiver.region);
		if (receiver.country) parts.push(receiver.country);
		return parts.join(', ') || 'Unknown';
	}
</script>

<!-- Receiver Modal -->
{#if showModal && selectedReceiver}
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
			aria-labelledby="receiver-modal-title"
			tabindex="-1"
		>
			<!-- Header -->
			<div
				class="flex items-center justify-between border-b border-surface-300 p-6 dark:border-surface-600"
			>
				<div class="flex items-center gap-3">
					<div
						class="flex h-10 w-10 items-center justify-center rounded-full bg-gray-500 text-white"
					>
						<Radio size={24} />
					</div>
					<div>
						<div class="flex items-center gap-2">
							<h2 id="receiver-modal-title" class="text-xl font-bold">
								{selectedReceiver.callsign}
							</h2>
							<a
								href={`/receivers/${selectedReceiver.id}`}
								target="_blank"
								rel="noopener noreferrer"
								class="preset-tonal-primary-500 btn btn-sm"
								title="View full receiver details"
							>
								<ExternalLink class="h-4 w-4" />
								Details
							</a>
						</div>
						{#if selectedReceiver.description}
							<p class="text-sm text-surface-600 dark:text-surface-400">
								{selectedReceiver.description}
							</p>
						{/if}
					</div>
				</div>
				<button class="preset-tonal-surface-500 btn btn-sm" onclick={closeModal}>
					<X size={20} />
				</button>
			</div>

			<div class="p-6">
				<div class="space-y-6">
					<!-- Station Info -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Info size={20} />
							Station Info
						</h3>

						<div class="space-y-3">
							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Callsign
									</dt>
									<dd class="mt-1 font-mono text-sm">{selectedReceiver.callsign}</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">Source</dt>
									<dd class="mt-1 text-sm">
										<span
											class="badge preset-filled-{selectedReceiver.fromOgnDb
												? 'success'
												: 'secondary'}"
										>
											{selectedReceiver.fromOgnDb ? 'OGN Database' : 'Auto-discovered'}
										</span>
									</dd>
								</div>
							</div>

							{#if selectedReceiver.ognDbCountry}
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										OGN DB Country
									</dt>
									<dd class="mt-1 text-sm">{selectedReceiver.ognDbCountry}</dd>
								</div>
							{/if}
						</div>
					</div>

					<!-- Location -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<MapPin size={20} />
							Location
						</h3>

						<div class="space-y-3">
							<div>
								<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">Location</dt>
								<dd class="mt-1 text-sm">{buildLocationString(selectedReceiver)}</dd>
							</div>

							<div>
								<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
									Coordinates
								</dt>
								<dd class="mt-1 font-mono text-sm">
									{formatCoordinates(selectedReceiver.latitude, selectedReceiver.longitude)}
								</dd>
							</div>
						</div>
					</div>

					<!-- Activity -->
					<div class="space-y-4">
						<h3 class="flex items-center gap-2 text-lg font-semibold">
							<Clock size={20} />
							Activity
						</h3>

						<div class="space-y-3">
							<div class="grid grid-cols-2 gap-4">
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										Last Packet
									</dt>
									<dd class="mt-1 text-sm">
										{formatRelativeTime(selectedReceiver.latestPacketAt)}
									</dd>
								</div>
								<div>
									<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
										First Seen
									</dt>
									<dd class="mt-1 text-sm">{formatDate(selectedReceiver.createdAt)}</dd>
								</div>
							</div>
						</div>
					</div>

					<!-- Contact -->
					{#if selectedReceiver.contact || selectedReceiver.email}
						<div class="space-y-4">
							<h3 class="text-lg font-semibold">Contact</h3>
							<div
								class="rounded-lg border border-surface-300 bg-surface-100 p-4 dark:border-surface-600 dark:bg-surface-800"
							>
								<div class="space-y-2">
									{#if selectedReceiver.contact}
										<div>
											<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
												Name
											</dt>
											<dd class="mt-1 text-sm">{selectedReceiver.contact}</dd>
										</div>
									{/if}
									{#if selectedReceiver.email}
										<div>
											<dt class="text-sm font-medium text-surface-600 dark:text-surface-400">
												Email
											</dt>
											<dd class="mt-1 text-sm">
												<a
													href={`mailto:${selectedReceiver.email}`}
													class="text-blue-600 hover:text-blue-800 hover:underline dark:text-blue-400 dark:hover:text-blue-300"
												>
													{selectedReceiver.email}
												</a>
											</dd>
										</div>
									{/if}
								</div>
							</div>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>
{/if}
