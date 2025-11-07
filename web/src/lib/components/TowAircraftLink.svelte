<script lang="ts">
	import { onMount } from 'svelte';
	import { Plane, ExternalLink } from '@lucide/svelte';
	import { DeviceRegistry } from '$lib/services/DeviceRegistry';
	import { serverCall } from '$lib/api/server';
	import type { Device } from '$lib/types';

	export let deviceId: string;
	export let size: 'sm' | 'md' | 'lg' = 'md';

	let device: Device | null = null;
	let loading = true;

	// Size classes
	const sizeClasses = {
		sm: 'text-xs',
		md: 'text-sm',
		lg: 'text-base'
	};

	const iconSizes = {
		sm: 'h-3 w-3',
		md: 'h-4 w-4',
		lg: 'h-5 w-5'
	};

	onMount(async () => {
		// Try to get device from registry cache first
		const registry = DeviceRegistry.getInstance();
		device = registry.getDevice(deviceId);

		if (device) {
			loading = false;
			return;
		}

		// If not in cache, fetch from server
		try {
			device = await serverCall<Device>(`/devices/${deviceId}`);
			loading = false;
		} catch (error) {
			console.error(`Failed to load device ${deviceId}:`, error);
			loading = false;
		}
	});
</script>

{#if loading}
	<span class="text-surface-500 {sizeClasses[size]}">Loading...</span>
{:else if device}
	<a
		href="/devices/{deviceId}"
		target="_blank"
		rel="noopener noreferrer"
		class="inline-flex items-center gap-1 anchor {sizeClasses[size]}"
		title="View towplane device: {device.registration || deviceId}"
	>
		{#if device.registration}
			<span>{device.registration}</span>
		{:else}
			<Plane class={iconSizes[size]} />
		{/if}
		<ExternalLink class={iconSizes[size]} />
	</a>
{:else}
	<a
		href="/devices/{deviceId}"
		target="_blank"
		rel="noopener noreferrer"
		class="inline-flex items-center gap-1 anchor {sizeClasses[size]}"
		title="View towplane device"
	>
		<Plane class={iconSizes[size]} />
		<ExternalLink class={iconSizes[size]} />
	</a>
{/if}
