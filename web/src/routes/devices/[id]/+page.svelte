<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
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
					linkedAircraft = await serverCall<Aircraft>(`/aircraft/registration/${device.registration}`);
				} catch (aircraftErr) {
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
		goto('/devices');
	}
</script>

<svelte:head>
	<title>{device?.registration || 'Device'} ({deviceHexId}) - Device Details</title>
</svelte:head>

<div class="container mx-auto p-4 space-y-6 max-w-6xl">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn btn-sm variant-soft" on:click={goBack}>
			<ArrowLeft class="w-4 h-4 mr-2" />
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
					<button class="btn variant-filled" on:click={loadDevice}>
						Try Again
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Device Details -->
	{#if !loading && !error && device}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex items-start justify-between flex-wrap gap-4">
					<div class="flex-1">
						<div class="flex items-center gap-3 mb-2">
							<Radio class="w-8 h-8 text-primary-500" />
							<div>
								<h1 class="h1">{device.registration}</h1>
								<p class="text-surface-600-300-token font-mono text-sm">Device ID: {formatDeviceId(device.device_id)}</p>
							</div>
						</div>
						
						<div class="flex flex-wrap gap-2 mt-3">
							<span class="badge {device.tracked ? 'variant-filled-success' : 'variant-filled-surface'}">
								{device.tracked ? 'Tracked' : 'Not Tracked'}
							</span>
							<span class="badge {device.identified ? 'variant-filled-primary' : 'variant-filled-surface'}">
								{device.identified ? 'Identified' : 'Unidentified'}
							</span>
							<span class="badge variant-soft">
								{device.device_type}
							</span>
						</div>
					</div>
				</div>
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
				<!-- Device Information -->
				<div class="card p-6 space-y-4">
					<h2 class="h2 flex items-center gap-2">
						<Settings class="w-6 h-6" />
						Device Information
					</h2>
					
					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Info class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Device Type</p>
								<p>{device.device_type}</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<Plane class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Aircraft Model</p>
								<p>{device.aircraft_model}</p>
							</div>
						</div>

						{#if device.competition_number}
							<div class="flex items-start gap-3">
								<Activity class="w-4 h-4 mt-1 text-surface-500" />
								<div>
									<p class="text-sm text-surface-600-300-token mb-1">Competition Number</p>
									<p class="font-mono">{device.competition_number}</p>
								</div>
							</div>
						{/if}

						<div class="flex items-start gap-3">
							<User class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Owner</p>
								<p>{device.user_id ? 'Assigned to user' : 'Unassigned'}</p>
							</div>
						</div>
					</div>
				</div>

				<!-- Linked Aircraft Information -->
				<div class="card p-6 space-y-4">
					<h2 class="h2 flex items-center gap-2">
						<Plane class="w-6 h-6" />
						Aircraft Registration
					</h2>
					
					{#if linkedAircraft}
						<div class="space-y-3">
							<div class="flex items-start gap-3">
								<Info class="w-4 h-4 mt-1 text-surface-500" />
								<div>
									<p class="text-sm text-surface-600-300-token mb-1">Registration Number</p>
									<p class="font-mono font-semibold">{linkedAircraft.registration_number}</p>
								</div>
							</div>

							{#if linkedAircraft.manufacturer_model_code}
								<div class="flex items-start gap-3">
									<Plane class="w-4 h-4 mt-1 text-surface-500" />
									<div>
										<p class="text-sm text-surface-600-300-token mb-1">Manufacturer Model</p>
										<p>{linkedAircraft.manufacturer_model_code}</p>
									</div>
								</div>
							{/if}

							{#if linkedAircraft.year_manufactured}
								<div class="flex items-start gap-3">
									<Calendar class="w-4 h-4 mt-1 text-surface-500" />
									<div>
										<p class="text-sm text-surface-600-300-token mb-1">Year Manufactured</p>
										<p>{linkedAircraft.year_manufactured}</p>
									</div>
								</div>
							{/if}

							{#if linkedAircraft.registrant_name}
								<div class="flex items-start gap-3">
									<User class="w-4 h-4 mt-1 text-surface-500" />
									<div>
										<p class="text-sm text-surface-600-300-token mb-1">Owner</p>
										<p>{linkedAircraft.registrant_name}</p>
									</div>
								</div>
							{/if}

							{#if linkedAircraft.transponder_code}
								<div class="flex items-start gap-3">
									<Radio class="w-4 h-4 mt-1 text-surface-500" />
									<div>
										<p class="text-sm text-surface-600-300-token mb-1">Transponder Code</p>
										<p class="font-mono">{linkedAircraft.transponder_code.toString(16).toUpperCase()}</p>
									</div>
								</div>
							{/if}
						</div>
					{:else}
						<div class="text-center py-8 text-surface-600-300-token">
							<Plane class="w-12 h-12 mx-auto mb-4 text-surface-400" />
							<p>No aircraft registration found for {device.registration}</p>
							<p class="text-sm mt-2">The device may be linked to an aircraft not in our database</p>
						</div>
					{/if}
				</div>

				<!-- Timestamps -->
				<div class="card p-6 space-y-4">
					<h2 class="h2 flex items-center gap-2">
						<Calendar class="w-6 h-6" />
						Record Information
					</h2>
					
					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Calendar class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Created</p>
								<p>{formatDate(device.created_at)}</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<Calendar class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Last Updated</p>
								<p>{formatDate(device.updated_at)}</p>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	{/if}
</div>