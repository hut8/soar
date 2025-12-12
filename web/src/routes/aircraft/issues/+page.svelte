<script lang="ts">
	import { AlertTriangle, Radio } from '@lucide/svelte';
	import { serverCall } from '$lib/api/server';
	import { onMount } from 'svelte';
	import type { Aircraft } from '$lib/types';

	interface AircraftIssuesResponse {
		duplicateDeviceAddresses: Aircraft[];
	}

	let duplicateDevices = $state<Aircraft[]>([]);
	let loading = $state(false);
	let error = $state('');

	async function loadIssues() {
		loading = true;
		error = '';

		try {
			const response = await serverCall<AircraftIssuesResponse>('/aircraft/issues');
			duplicateDevices = response.duplicateDeviceAddresses || [];
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load aircraft issues: ${errorMessage}`;
			console.error('Error loading aircraft issues:', err);
			duplicateDevices = [];
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadIssues();
	});

	function formatAddress(address: string): string {
		// Address is already in hex format, just ensure it's uppercase and padded
		return address.toUpperCase().padStart(6, '0');
	}

	function formatDate(dateStr: string | null | undefined): string {
		if (!dateStr) return '-';
		const date = new Date(dateStr);
		return date.toLocaleString('en-US', {
			year: 'numeric',
			month: '2-digit',
			day: '2-digit',
			hour: '2-digit',
			minute: '2-digit'
		});
	}
</script>

<svelte:head>
	<title>Aircraft Issues - SOAR Tracking</title>
</svelte:head>

<div class="container mx-auto max-w-7xl space-y-6 p-4">
	<!-- Header -->
	<header class="space-y-2 text-center">
		<h1 class="flex items-center justify-center gap-2 h1">
			<AlertTriangle class="h-8 w-8" />
			Aircraft Issues
		</h1>
		<p class="text-surface-500-400-token">
			Review aircraft data quality issues and duplicate addresses
		</p>
	</header>

	<!-- Duplicate Addresses Section -->
	<section class="space-y-4 card p-6">
		<h2 class="flex items-center gap-2 text-xl font-semibold text-error-500">
			<AlertTriangle class="h-6 w-6" />
			Duplicate Aircraft Addresses
		</h2>

		{#if loading}
			<div class="flex items-center justify-center p-12">
				<div
					class="h-12 w-12 animate-spin rounded-full border-4 border-primary-500 border-t-transparent"
				></div>
			</div>
		{:else if error}
			<div class="rounded border border-error-200 bg-error-50 p-4 text-error-700">
				{error}
			</div>
		{:else if duplicateDevices.length === 0}
			<div class="space-y-4 card p-12 text-center">
				<Radio class="mx-auto mb-4 h-16 w-16 text-success-500" />
				<div class="space-y-2">
					<h3 class="h3 text-success-500">No Issues Found</h3>
					<p class="text-surface-500-400-token">
						All aircraft addresses are unique. No duplicate addresses detected.
					</p>
				</div>
			</div>
		{:else}
			<div class="space-y-3">
				<p class="text-surface-500-400-token">
					The following addresses appear multiple times with different address types. This may
					indicate data quality issues that need attention.
				</p>

				<div class="overflow-x-auto">
					<table class="w-full table-auto">
						<thead>
							<tr class="bg-surface-200-700-token">
								<th class="p-3 text-left font-semibold">Address (Hex)</th>
								<th class="p-3 text-left font-semibold">Address Type</th>
								<th class="p-3 text-left font-semibold">Registration</th>
								<th class="p-3 text-left font-semibold">Aircraft Model</th>
								<th class="p-3 text-left font-semibold">From DDB</th>
								<th class="p-3 text-left font-semibold">Tracked</th>
								<th class="p-3 text-left font-semibold">Last Fix</th>
							</tr>
						</thead>
						<tbody>
							{#each duplicateDevices as device (device.id || device.address)}
								<tr class="border-surface-200-700-token hover:bg-surface-100-800-token border-b">
									<td class="p-3 font-mono font-semibold">{formatAddress(device.address)}</td>
									<td class="p-3">{device.address_type}</td>
									<td class="p-3">
										{#if device.id}
											<a href="/aircraft/{device.id}" class="text-primary-500 hover:underline">
												{device.registration}
											</a>
										{:else}
											{device.registration}
										{/if}
									</td>
									<td class="p-3">{device.aircraft_model || '-'}</td>
									<td class="p-3">{device.from_ddb ? 'Yes' : 'No'}</td>
									<td class="p-3">{device.tracked ? 'Yes' : 'No'}</td>
									<td class="p-3">{formatDate(device.last_fix_at)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="bg-surface-100-800-token mt-4 rounded p-4">
					<p class="text-surface-600-300-token text-sm">
						<strong>Total Devices with Duplicate Addresses:</strong>
						{duplicateDevices.length}
					</p>
				</div>
			</div>
		{/if}
	</section>
</div>
