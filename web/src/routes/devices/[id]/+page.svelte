<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import {
		ArrowLeft,
		Radio,
		Plane,
		User,
		Calendar,
		Info,
		Activity,
		Settings
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';

	interface Device {
		device_id: number;
		device_type: string;
		aircraft_model: string;
		registration: string;
		competition_number: string;
		tracked: boolean;
		identified: boolean;
		user_id?: string;
		created_at: string;
		updated_at: string;
	}

	interface Aircraft {
		registration_number: string;
		serial_number: string;
		manufacturer_model_code?: string;
		engine_manufacturer_model_code?: string;
		year_manufactured?: number;
		registrant_type?: string;
		registrant_name?: string;
		aircraft_type?: string;
		engine_type?: number;
		status_code?: string;
		transponder_code?: number;
		airworthiness_class?: string;
		airworthiness_date?: string;
		certificate_issue_date?: string;
		expiration_date?: string;
		club_id?: string;
		home_base_airport_id?: string;
		kit_manufacturer_name?: string;
		kit_model_name?: string;
		other_names: string[];
	}

	let device: Device | null = null;
	let linkedAircraft: Aircraft | null = null;
	let loading = true;
	let error = '';
	let deviceHexId = '';

	$: deviceHexId = $page.params.id || '';

	function parseDeviceId(hexString: string): number | null {
		const cleaned = hexString.replace(/[^a-fA-F0-9]/g, '');
		if (cleaned.length !== 6) return null;

		const parsed = parseInt(cleaned, 16);
		return isNaN(parsed) ? null : parsed;
	}

	function formatDeviceId(deviceId: number): string {
		return deviceId.toString(16).toUpperCase().padStart(6, '0');
	}

	onMount(async () => {
		if (deviceHexId) {
			await loadDevice();
		}
	});

	async function loadDevice() {
		const deviceId = parseDeviceId(deviceHexId);
		if (deviceId === null) {
			error = 'Invalid device ID format';
			loading = false;
			return;
		}

		loading = true;
		error = '';

		try {
			device = await serverCall<Device>(`/devices/${deviceId}`);

			// Try to load linked aircraft information
			if (device.registration) {
				try {
					linkedAircraft = await serverCall<Aircraft>(
						`/aircraft/registration/${device.registration}`
					);
				} catch {
					// Aircraft not found is okay, don't show error for this
					console.log('No aircraft found for registration:', device.registration);
				}
			}
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load device: ${errorMessage}`;
			console.error('Error loading device:', err);
		} finally {
			loading = false;
		}
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'long',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function goBack() {
		goto(resolve('/devices'));
	}
</script>

<svelte:head>
	<title>{device?.registration || 'Device'} ({deviceHexId}) - Device Details</title>
</svelte:head>

<div class="container mx-auto max-w-6xl space-y-6 p-4">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="variant-soft btn btn-sm" on:click={goBack}>
			<ArrowLeft class="mr-2 h-4 w-4" />
			Back to Devices
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Loading device details...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert variant-filled-error">
			<div class="alert-message">
				<h3 class="h3">Error Loading Device</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="variant-filled btn" on:click={loadDevice}> Try Again </button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Device Details -->
	{#if !loading && !error && device}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex flex-wrap items-start justify-between gap-4">
					<div class="flex-1">
						<div class="mb-2 flex items-center gap-3">
							<Radio class="h-8 w-8 text-primary-500" />
							<div>
								<h1 class="h1">{device.registration}</h1>
								<p class="text-surface-600-300-token font-mono text-sm">
									Device ID: {formatDeviceId(device.device_id)}
								</p>
							</div>
						</div>

						<div class="mt-3 flex flex-wrap gap-2">
							<span
								class="badge {device.tracked ? 'variant-filled-success' : 'variant-filled-surface'}"
							>
								{device.tracked ? 'Tracked' : 'Not Tracked'}
							</span>
							<span
								class="badge {device.identified
									? 'variant-filled-primary'
									: 'variant-filled-surface'}"
							>
								{device.identified ? 'Identified' : 'Unidentified'}
							</span>
							<span class="variant-soft badge">
								{device.device_type}
							</span>
						</div>
					</div>
				</div>
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 gap-6 lg:grid-cols-2">
				<!-- Device Information -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Settings class="h-6 w-6" />
						Device Information
					</h2>

					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Info class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Device Type</p>
								<p>{device.device_type}</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<Plane class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Aircraft Model</p>
								<p>{device.aircraft_model}</p>
							</div>
						</div>

						{#if device.competition_number}
							<div class="flex items-start gap-3">
								<Activity class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Competition Number</p>
									<p class="font-mono">{device.competition_number}</p>
								</div>
							</div>
						{/if}

						<div class="flex items-start gap-3">
							<User class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Owner</p>
								<p>{device.user_id ? 'Assigned to user' : 'Unassigned'}</p>
							</div>
						</div>
					</div>
				</div>

				<!-- Linked Aircraft Information -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Plane class="h-6 w-6" />
						Aircraft Registration
					</h2>

					{#if linkedAircraft}
						<div class="space-y-3">
							<div class="flex items-start gap-3">
								<Info class="mt-1 h-4 w-4 text-surface-500" />
								<div>
									<p class="text-surface-600-300-token mb-1 text-sm">Registration Number</p>
									<p class="font-mono font-semibold">{linkedAircraft.registration_number}</p>
								</div>
							</div>

							{#if linkedAircraft.manufacturer_model_code}
								<div class="flex items-start gap-3">
									<Plane class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Manufacturer Model</p>
										<p>{linkedAircraft.manufacturer_model_code}</p>
									</div>
								</div>
							{/if}

							{#if linkedAircraft.year_manufactured}
								<div class="flex items-start gap-3">
									<Calendar class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Year Manufactured</p>
										<p>{linkedAircraft.year_manufactured}</p>
									</div>
								</div>
							{/if}

							{#if linkedAircraft.registrant_name}
								<div class="flex items-start gap-3">
									<User class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Owner</p>
										<p>{linkedAircraft.registrant_name}</p>
									</div>
								</div>
							{/if}

							{#if linkedAircraft.transponder_code}
								<div class="flex items-start gap-3">
									<Radio class="mt-1 h-4 w-4 text-surface-500" />
									<div>
										<p class="text-surface-600-300-token mb-1 text-sm">Transponder Code</p>
										<p class="font-mono">
											{linkedAircraft.transponder_code.toString(16).toUpperCase()}
										</p>
									</div>
								</div>
							{/if}
						</div>
					{:else}
						<div class="text-surface-600-300-token py-8 text-center">
							<Plane class="mx-auto mb-4 h-12 w-12 text-surface-400" />
							<p>No aircraft registration found for {device.registration}</p>
							<p class="mt-2 text-sm">
								The device may be linked to an aircraft not in our database
							</p>
						</div>
					{/if}
				</div>

				<!-- Timestamps -->
				<div class="space-y-4 card p-6">
					<h2 class="flex items-center gap-2 h2">
						<Calendar class="h-6 w-6" />
						Record Information
					</h2>

					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Calendar class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Created</p>
								<p>{formatDate(device.created_at)}</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<Calendar class="mt-1 h-4 w-4 text-surface-500" />
							<div>
								<p class="text-surface-600-300-token mb-1 text-sm">Last Updated</p>
								<p>{formatDate(device.updated_at)}</p>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>
