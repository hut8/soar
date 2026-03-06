<script lang="ts">
	import { Bell, BellOff, Loader2, Trash2, Save } from '@lucide/svelte';
	import { Switch } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';
	import { auth } from '$lib/stores/auth';
	import { toaster } from '$lib/toaster';
	import type { ReceiverAlertView, DataResponse } from '$lib/types';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'ReceiverAlertConfig']);

	let { receiverId }: { receiverId: string } = $props();

	let isAuthenticated = $state(false);
	let alert = $state<ReceiverAlertView | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let hasSubscription = $state(false);

	// Form state
	let alertOnDown = $state(true);
	let downAfterMinutes = $state(30);
	let alertOnHighCpu = $state(false);
	let cpuThreshold = $state(90);
	let alertOnHighTemperature = $state(false);
	let temperatureThresholdC = $state(70);
	let sendEmail = $state(true);
	let baseCooldownMinutes = $state(30);

	auth.subscribe((state) => {
		isAuthenticated = state.isAuthenticated;
		if (state.isAuthenticated) {
			loadAlert();
		} else {
			loading = false;
		}
	});

	async function loadAlert() {
		loading = true;
		try {
			const response = await serverCall<DataResponse<ReceiverAlertView>>(
				`/receivers/${receiverId}/alerts`
			);
			// serverCall returns {} for 204 No Content, so check for actual data
			if (response?.data?.id) {
				alert = response.data;
				hasSubscription = true;
				populateForm(alert);
			} else {
				hasSubscription = false;
			}
		} catch (err: unknown) {
			hasSubscription = false;
			logger.warn('Failed to load receiver alert: {error}', { error: err });
		} finally {
			loading = false;
		}
	}

	function populateForm(a: ReceiverAlertView) {
		alertOnDown = a.alertOnDown;
		downAfterMinutes = a.downAfterMinutes;
		alertOnHighCpu = a.alertOnHighCpu;
		cpuThreshold = Math.round(a.cpuThreshold * 100);
		alertOnHighTemperature = a.alertOnHighTemperature;
		temperatureThresholdC = a.temperatureThresholdC;
		sendEmail = a.sendEmail;
		baseCooldownMinutes = a.baseCooldownMinutes;
	}

	async function saveAlert() {
		saving = true;
		try {
			const response = await serverCall<DataResponse<ReceiverAlertView>>(
				`/receivers/${receiverId}/alerts`,
				{
					method: 'PUT',
					body: JSON.stringify({
						alertOnDown,
						downAfterMinutes,
						alertOnHighCpu,
						cpuThreshold: cpuThreshold / 100,
						alertOnHighTemperature,
						temperatureThresholdC,
						sendEmail,
						baseCooldownMinutes
					})
				}
			);
			alert = response.data;
			hasSubscription = true;
			toaster.success({ title: 'Alert subscription saved' });
		} catch (err) {
			logger.warn('Failed to save receiver alert: {error}', { error: err });
			toaster.error({ title: 'Failed to save alert subscription' });
		} finally {
			saving = false;
		}
	}

	async function deleteAlert() {
		saving = true;
		try {
			await serverCall(`/receivers/${receiverId}/alerts`, { method: 'DELETE' });
			alert = null;
			hasSubscription = false;
			// Reset to defaults
			alertOnDown = true;
			downAfterMinutes = 30;
			alertOnHighCpu = false;
			cpuThreshold = 90;
			alertOnHighTemperature = false;
			temperatureThresholdC = 70;
			sendEmail = true;
			baseCooldownMinutes = 30;
			toaster.success({ title: 'Alert subscription removed' });
		} catch (err) {
			logger.warn('Failed to delete receiver alert: {error}', { error: err });
			toaster.error({ title: 'Failed to remove alert subscription' });
		} finally {
			saving = false;
		}
	}
</script>

{#if !isAuthenticated}
	<!-- Don't render anything if not logged in -->
{:else if loading}
	<div class="card p-6">
		<div class="flex items-center gap-2">
			<Loader2 class="h-5 w-5 animate-spin" />
			<span>Loading alerts...</span>
		</div>
	</div>
{:else}
	<div class="card p-6">
		<div class="mb-4 flex items-center justify-between">
			<h2 class="flex items-center gap-2 h2">
				{#if hasSubscription}
					<Bell class="h-6 w-6 text-primary-500" />
				{:else}
					<BellOff class="h-6 w-6" />
				{/if}
				Alert Subscription
			</h2>
			{#if hasSubscription && alert?.consecutiveAlerts && alert.consecutiveAlerts > 0}
				<span class="badge preset-filled-warning-500 text-xs">
					Active alert ({alert.consecutiveAlerts} sent)
				</span>
			{/if}
		</div>

		<div class="space-y-6">
			<!-- Down Detection -->
			<div class="space-y-3">
				<Switch
					class="flex w-full items-center justify-between"
					checked={alertOnDown}
					onCheckedChange={(details) => {
						alertOnDown = details.checked;
					}}
				>
					<div>
						<Switch.Label class="font-semibold">Receiver Offline Detection</Switch.Label>
						<p class="text-surface-600-300-token text-sm">
							Alert when no data is received for a period of time
						</p>
					</div>
					<Switch.Control>
						<Switch.Thumb />
					</Switch.Control>
					<Switch.HiddenInput name="alert-on-down" />
				</Switch>
				{#if alertOnDown}
					<div class="ml-4 flex items-center gap-2">
						<label for="down-minutes" class="text-sm whitespace-nowrap">Alert after</label>
						<input
							id="down-minutes"
							type="number"
							min="5"
							max="1440"
							bind:value={downAfterMinutes}
							class="input w-24 px-2 py-1 text-sm"
						/>
						<span class="text-sm">minutes offline</span>
					</div>
				{/if}
			</div>

			<!-- High CPU -->
			<div class="space-y-3">
				<Switch
					class="flex w-full items-center justify-between"
					checked={alertOnHighCpu}
					onCheckedChange={(details) => {
						alertOnHighCpu = details.checked;
					}}
				>
					<div>
						<Switch.Label class="font-semibold">High CPU Load</Switch.Label>
						<p class="text-surface-600-300-token text-sm">
							Alert when CPU usage exceeds a threshold
						</p>
					</div>
					<Switch.Control>
						<Switch.Thumb />
					</Switch.Control>
					<Switch.HiddenInput name="alert-on-cpu" />
				</Switch>
				{#if alertOnHighCpu}
					<div class="ml-4 flex items-center gap-2">
						<label for="cpu-threshold" class="text-sm whitespace-nowrap">Alert above</label>
						<input
							id="cpu-threshold"
							type="number"
							min="1"
							max="100"
							bind:value={cpuThreshold}
							class="input w-24 px-2 py-1 text-sm"
						/>
						<span class="text-sm">% CPU</span>
					</div>
				{/if}
			</div>

			<!-- High Temperature -->
			<div class="space-y-3">
				<Switch
					class="flex w-full items-center justify-between"
					checked={alertOnHighTemperature}
					onCheckedChange={(details) => {
						alertOnHighTemperature = details.checked;
					}}
				>
					<div>
						<Switch.Label class="font-semibold">High Temperature</Switch.Label>
						<p class="text-surface-600-300-token text-sm">
							Alert when CPU temperature exceeds a threshold
						</p>
					</div>
					<Switch.Control>
						<Switch.Thumb />
					</Switch.Control>
					<Switch.HiddenInput name="alert-on-temp" />
				</Switch>
				{#if alertOnHighTemperature}
					<div class="ml-4 flex items-center gap-2">
						<label for="temp-threshold" class="text-sm whitespace-nowrap">Alert above</label>
						<input
							id="temp-threshold"
							type="number"
							min="0"
							max="200"
							bind:value={temperatureThresholdC}
							class="input w-24 px-2 py-1 text-sm"
						/>
						<span class="text-sm">&deg;C</span>
					</div>
				{/if}
			</div>

			<!-- Email & Cooldown -->
			<div class="border-surface-200-700-token space-y-3 border-t pt-4">
				<Switch
					class="flex w-full items-center justify-between"
					checked={sendEmail}
					onCheckedChange={(details) => {
						sendEmail = details.checked;
					}}
				>
					<div>
						<Switch.Label class="font-semibold">Email Notifications</Switch.Label>
						<p class="text-surface-600-300-token text-sm">
							Receive email alerts when conditions are triggered
						</p>
					</div>
					<Switch.Control>
						<Switch.Thumb />
					</Switch.Control>
					<Switch.HiddenInput name="send-email" />
				</Switch>
				{#if sendEmail}
					<div class="ml-4 flex items-center gap-2">
						<label for="cooldown" class="text-sm whitespace-nowrap">Base cooldown</label>
						<input
							id="cooldown"
							type="number"
							min="5"
							max="1440"
							bind:value={baseCooldownMinutes}
							class="input w-24 px-2 py-1 text-sm"
						/>
						<span class="text-sm">minutes</span>
					</div>
					<p class="text-surface-500-400-token ml-4 text-xs">
						Repeat alerts use exponential backoff: {baseCooldownMinutes}min, {baseCooldownMinutes *
							2}min, {baseCooldownMinutes * 4}min, ... up to 24h
					</p>
				{/if}
			</div>

			<!-- Last alert info -->
			{#if hasSubscription && alert?.lastAlertedAt}
				<div class="border-surface-200-700-token border-t pt-4">
					<p class="text-surface-500-400-token text-xs">
						Last alert sent: {new Date(alert.lastAlertedAt).toLocaleString()}
						{#if alert.lastCondition}
							({alert.lastCondition})
						{/if}
					</p>
				</div>
			{/if}

			<!-- Action Buttons -->
			<div class="flex gap-3 pt-2">
				<button class="btn preset-filled-primary-500" onclick={saveAlert} disabled={saving}>
					{#if saving}
						<Loader2 class="mr-2 h-4 w-4 animate-spin" />
					{:else}
						<Save class="mr-2 h-4 w-4" />
					{/if}
					{hasSubscription ? 'Update' : 'Subscribe'}
				</button>
				{#if hasSubscription}
					<button class="btn preset-tonal-error" onclick={deleteAlert} disabled={saving}>
						<Trash2 class="mr-2 h-4 w-4" />
						Remove
					</button>
				{/if}
			</div>
		</div>
	</div>
{/if}
