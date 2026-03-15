<script lang="ts">
	import { onMount } from 'svelte';
	import { Bell, BellOff, BellRing } from '@lucide/svelte';
	import {
		isPushSupported,
		getPermissionState,
		subscribeToPush,
		unsubscribeFromPush,
		isCurrentlySubscribed,
		deleteSubscription,
		getSubscriptions
	} from '$lib/services/pushNotifications';
	import { toaster } from '$lib/toaster';

	let supported = $state(false);
	let subscribed = $state(false);
	let loading = $state(true);
	let permissionState = $state<NotificationPermission | 'unsupported'>('unsupported');

	onMount(async () => {
		supported = isPushSupported();
		if (supported) {
			permissionState = getPermissionState();
			subscribed = await isCurrentlySubscribed();
		}
		loading = false;
	});

	async function toggle() {
		if (loading) return;
		loading = true;

		try {
			if (subscribed) {
				// Unsubscribe this browser and remove all server-side records for this user.
				// This disables notifications across all devices — acceptable since the toggle
				// is a simple on/off for the user, not per-device.
				await unsubscribeFromPush();
				const subs = await getSubscriptions();
				for (const sub of subs) {
					await deleteSubscription(sub.id);
				}
				subscribed = false;
				toaster.success({ title: 'Flight notifications disabled' });
			} else {
				// Subscribe
				const result = await subscribeToPush();
				if (result) {
					subscribed = true;
					toaster.success({ title: 'Flight notifications enabled' });
				} else {
					permissionState = getPermissionState();
					if (permissionState === 'denied') {
						toaster.error({
							title: 'Notifications blocked',
							description: 'Enable notifications in your browser settings'
						});
					}
				}
			}
		} catch (err) {
			const msg = err instanceof Error ? err.message : 'Unknown error';
			toaster.error({ title: `Notification error: ${msg}` });
		} finally {
			loading = false;
		}
	}
</script>

{#if supported}
	<button
		onclick={toggle}
		disabled={loading || permissionState === 'denied'}
		class="btn gap-2 text-sm {subscribed ? 'preset-tonal-primary-500' : 'preset-tonal-surface-500'}"
		title={subscribed
			? 'Disable flight notifications'
			: permissionState === 'denied'
				? 'Notifications blocked in browser settings'
				: 'Get notified when aircraft take off or land'}
	>
		{#if loading}
			<div
				class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></div>
		{:else if subscribed}
			<BellRing class="h-4 w-4" />
		{:else if permissionState === 'denied'}
			<BellOff class="h-4 w-4" />
		{:else}
			<Bell class="h-4 w-4" />
		{/if}
		{subscribed ? 'Notifications On' : 'Notify Me'}
	</button>
{/if}
