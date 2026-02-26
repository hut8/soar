<script lang="ts">
	import { onMount } from 'svelte';
	import { Radio, Plus, Trash2, Edit2, Power, PowerOff, AlertCircle } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { getLogger } from '$lib/logging';
	import type { DataStream, StreamFormat, DataListResponse, DataResponse } from '$lib/types';

	const logger = getLogger(['soar', 'DataStreamsPage']);

	let streams = $state<DataStream[]>([]);
	let loading = $state(true);
	let error = $state('');
	let showAddModal = $state(false);
	let showEditModal = $state(false);
	let showDeleteModal = $state(false);

	// Form state
	let formName = $state('');
	let formFormat = $state<StreamFormat>('aprs');
	let formHost = $state('');
	let formPort = $state<number>(10152);
	let formEnabled = $state(true);
	let formCallsign = $state('');
	let formFilter = $state('');
	let formError = $state('');
	let submitting = $state(false);

	// Edit/Delete state
	let editingStream = $state<DataStream | null>(null);
	let deletingStream = $state<DataStream | null>(null);

	let isAprsFormat = $derived(formFormat === 'aprs');

	onMount(async () => {
		await loadStreams();
	});

	async function loadStreams() {
		loading = true;
		error = '';

		try {
			const response = await serverCall<DataListResponse<DataStream>>('/data-streams');
			streams = response.data || [];
		} catch (err) {
			logger.error('Error loading data streams: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load data streams';
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		formName = '';
		formFormat = 'aprs';
		formHost = '';
		formPort = 10152;
		formEnabled = true;
		formCallsign = '';
		formFilter = '';
		formError = '';
	}

	function openAddModal() {
		resetForm();
		showAddModal = true;
	}

	function closeAddModal() {
		showAddModal = false;
	}

	function openEditModal(stream: DataStream) {
		editingStream = stream;
		formName = stream.name;
		formFormat = stream.format;
		formHost = stream.host;
		formPort = stream.port;
		formEnabled = stream.enabled;
		formCallsign = stream.callsign || '';
		formFilter = stream.filter || '';
		formError = '';
		showEditModal = true;
	}

	function closeEditModal() {
		showEditModal = false;
		editingStream = null;
	}

	function openDeleteModal(stream: DataStream) {
		deletingStream = stream;
		showDeleteModal = true;
	}

	function closeDeleteModal() {
		showDeleteModal = false;
		deletingStream = null;
	}

	function validateForm(): boolean {
		if (!formName.trim()) {
			formError = 'Name is required';
			return false;
		}
		if (!formHost.trim()) {
			formError = 'Host is required';
			return false;
		}
		if (!formPort || formPort < 1 || formPort > 65535) {
			formError = 'Port must be between 1 and 65535';
			return false;
		}
		return true;
	}

	function buildRequestBody() {
		const body: Record<string, unknown> = {
			name: formName.trim(),
			format: formFormat,
			host: formHost.trim(),
			port: formPort,
			enabled: formEnabled
		};
		if (formFormat === 'aprs') {
			if (formCallsign.trim()) {
				body.callsign = formCallsign.trim();
			}
			if (formFilter.trim()) {
				body.filter = formFilter.trim();
			}
		}
		return body;
	}

	async function handleAddStream() {
		if (!validateForm()) return;

		submitting = true;
		formError = '';

		try {
			await serverCall<DataResponse<DataStream>>('/data-streams', {
				method: 'POST',
				body: JSON.stringify(buildRequestBody())
			});

			await loadStreams();
			closeAddModal();
		} catch (err) {
			logger.error('Error adding data stream: {error}', { error: err });
			formError = err instanceof Error ? err.message : 'Failed to add data stream';
		} finally {
			submitting = false;
		}
	}

	async function handleEditStream() {
		if (!editingStream) return;
		if (!validateForm()) return;

		submitting = true;
		formError = '';

		try {
			await serverCall<DataResponse<DataStream>>(`/data-streams/${editingStream.id}`, {
				method: 'PUT',
				body: JSON.stringify(buildRequestBody())
			});

			await loadStreams();
			closeEditModal();
		} catch (err) {
			logger.error('Error updating data stream: {error}', { error: err });
			formError = err instanceof Error ? err.message : 'Failed to update data stream';
		} finally {
			submitting = false;
		}
	}

	async function handleDeleteStream() {
		if (!deletingStream) return;

		submitting = true;

		try {
			await serverCall(`/data-streams/${deletingStream.id}`, {
				method: 'DELETE'
			});

			await loadStreams();
			closeDeleteModal();
		} catch (err) {
			logger.error('Error deleting data stream: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to delete data stream';
		} finally {
			submitting = false;
		}
	}

	async function toggleEnabled(stream: DataStream) {
		try {
			await serverCall(`/data-streams/${stream.id}`, {
				method: 'PUT',
				body: JSON.stringify({ enabled: !stream.enabled })
			});
			await loadStreams();
		} catch (err) {
			logger.error('Error toggling stream: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to toggle stream';
		}
	}

	function formatBadge(format: StreamFormat): string {
		switch (format) {
			case 'aprs':
				return 'APRS';
			case 'adsb':
				return 'ADS-B';
			case 'sbs':
				return 'SBS';
			default:
				return format;
		}
	}

	function formatBadgeClass(format: StreamFormat): string {
		switch (format) {
			case 'aprs':
				return 'variant-soft-primary';
			case 'adsb':
				return 'variant-soft-secondary';
			case 'sbs':
				return 'variant-soft-tertiary';
			default:
				return 'variant-soft-surface';
		}
	}

	function updateDefaultPort() {
		switch (formFormat) {
			case 'aprs':
				formPort = 10152;
				break;
			case 'adsb':
				formPort = 30005;
				break;
			case 'sbs':
				formPort = 30003;
				break;
		}
	}
</script>

<div class="container mx-auto max-w-4xl p-4">
	<!-- Header -->
	<div class="mb-6 flex items-center justify-between">
		<div class="flex items-center gap-3">
			<Radio class="h-6 w-6" />
			<h1 class="h2">Data Streams</h1>
		</div>
		<button onclick={openAddModal} class="variant-filled-primary btn">
			<Plus class="h-4 w-4" />
			<span>Add Stream</span>
		</button>
	</div>

	{#if loading}
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
	{:else if streams.length === 0}
		<div class="card p-8 text-center">
			<Radio class="mx-auto mb-4 h-12 w-12 opacity-50" />
			<p class="mb-2 text-lg">No data streams configured</p>
			<p class="text-sm opacity-75">Add your first data stream to start ingesting data</p>
		</div>
	{:else}
		<div class="card">
			<div class="table-container">
				<table class="table-hover table">
					<thead>
						<tr>
							<th>Name</th>
							<th>Format</th>
							<th>Endpoint</th>
							<th>Status</th>
							<th>Updated</th>
							<th class="text-right">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#each streams as stream (stream.id)}
							<tr class:opacity-50={!stream.enabled}>
								<td class="font-semibold">{stream.name}</td>
								<td>
									<span class="badge {formatBadgeClass(stream.format)}">
										{formatBadge(stream.format)}
									</span>
								</td>
								<td class="font-mono text-sm">{stream.host}:{stream.port}</td>
								<td>
									<button
										onclick={() => toggleEnabled(stream)}
										class="btn btn-sm {stream.enabled
											? 'variant-soft-success'
											: 'variant-soft-error'}"
										title={stream.enabled ? 'Disable stream' : 'Enable stream'}
									>
										{#if stream.enabled}
											<Power class="h-4 w-4" />
											<span>Enabled</span>
										{:else}
											<PowerOff class="h-4 w-4" />
											<span>Disabled</span>
										{/if}
									</button>
								</td>
								<td class="text-sm opacity-75">
									{new Date(stream.updatedAt).toLocaleDateString()}
								</td>
								<td class="text-right">
									<div class="flex justify-end gap-2">
										<button
											onclick={() => openEditModal(stream)}
											class="variant-ghost-surface btn btn-sm"
											title="Edit"
										>
											<Edit2 class="h-4 w-4" />
										</button>
										<button
											onclick={() => openDeleteModal(stream)}
											class="variant-ghost-error btn btn-sm"
											title="Delete"
										>
											<Trash2 class="h-4 w-4" />
										</button>
									</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>
	{/if}
</div>

<!-- Add Stream Modal -->
{#if showAddModal}
	<div class="modal-backdrop" onclick={closeAddModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Add Data Stream</h3>

			<label class="label mb-4">
				<span>Name</span>
				<input type="text" class="input" bind:value={formName} placeholder="e.g., OGN Full Feed" />
			</label>

			<label class="label mb-4">
				<span>Format</span>
				<select class="select" bind:value={formFormat} onchange={updateDefaultPort}>
					<option value="aprs">APRS (OGN)</option>
					<option value="adsb">ADS-B (Beast)</option>
					<option value="sbs">SBS (BaseStation)</option>
				</select>
			</label>

			<div class="mb-4 grid grid-cols-3 gap-4">
				<label class="col-span-2 label">
					<span>Host</span>
					<input
						type="text"
						class="input"
						bind:value={formHost}
						placeholder="e.g., aprs.glidernet.org"
					/>
				</label>
				<label class="label">
					<span>Port</span>
					<input type="number" class="input" bind:value={formPort} min="1" max="65535" />
				</label>
			</div>

			{#if isAprsFormat}
				<label class="label mb-4">
					<span>Callsign</span>
					<input type="text" class="input" bind:value={formCallsign} placeholder="N0CALL" />
				</label>

				<label class="label mb-4">
					<span>Filter</span>
					<input
						type="text"
						class="input"
						bind:value={formFilter}
						placeholder="Leave empty for full feed"
					/>
					<p class="mt-1 text-sm opacity-75">
						APRS filter string. Leave empty for full global feed (port 10152)
					</p>
				</label>
			{/if}

			{#if formError}
				<div class="alert variant-filled-error mb-4">
					<p>{formError}</p>
				</div>
			{/if}

			<div class="flex justify-end gap-2">
				<button onclick={closeAddModal} class="variant-ghost-surface btn" disabled={submitting}>
					Cancel
				</button>
				<button onclick={handleAddStream} class="variant-filled-primary btn" disabled={submitting}>
					{submitting ? 'Adding...' : 'Add Stream'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Edit Stream Modal -->
{#if showEditModal && editingStream}
	<div class="modal-backdrop" onclick={closeEditModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Edit Data Stream</h3>

			<label class="label mb-4">
				<span>Name</span>
				<input type="text" class="input" bind:value={formName} />
			</label>

			<label class="label mb-4">
				<span>Format</span>
				<select class="select" bind:value={formFormat} onchange={updateDefaultPort}>
					<option value="aprs">APRS (OGN)</option>
					<option value="adsb">ADS-B (Beast)</option>
					<option value="sbs">SBS (BaseStation)</option>
				</select>
			</label>

			<div class="mb-4 grid grid-cols-3 gap-4">
				<label class="col-span-2 label">
					<span>Host</span>
					<input type="text" class="input" bind:value={formHost} />
				</label>
				<label class="label">
					<span>Port</span>
					<input type="number" class="input" bind:value={formPort} min="1" max="65535" />
				</label>
			</div>

			{#if isAprsFormat}
				<label class="label mb-4">
					<span>Callsign</span>
					<input type="text" class="input" bind:value={formCallsign} placeholder="N0CALL" />
				</label>

				<label class="label mb-4">
					<span>Filter</span>
					<input
						type="text"
						class="input"
						bind:value={formFilter}
						placeholder="Leave empty for full feed"
					/>
				</label>
			{/if}

			<label class="label mb-4 flex items-center gap-2">
				<input type="checkbox" class="checkbox" bind:checked={formEnabled} />
				<span>Enabled</span>
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
				<button onclick={handleEditStream} class="variant-filled-primary btn" disabled={submitting}>
					{submitting ? 'Saving...' : 'Save Changes'}
				</button>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Confirmation Modal -->
{#if showDeleteModal && deletingStream}
	<div class="modal-backdrop" onclick={closeDeleteModal}></div>
	<div class="modal">
		<div class="w-full max-w-md card p-6" onclick={(e) => e.stopPropagation()}>
			<h3 class="mb-4 h3">Delete Data Stream</h3>

			<p class="mb-4">
				Are you sure you want to delete the data stream
				<strong>{deletingStream.name}</strong>
				({deletingStream.host}:{deletingStream.port})?
			</p>

			<div class="flex justify-end gap-2">
				<button onclick={closeDeleteModal} class="variant-ghost-surface btn" disabled={submitting}>
					Cancel
				</button>
				<button onclick={handleDeleteStream} class="variant-filled-error btn" disabled={submitting}>
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
