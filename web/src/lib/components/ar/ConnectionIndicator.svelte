<script lang="ts">
	import { Wifi, WifiOff, RotateCcw } from '@lucide/svelte';
	import { websocketStatus } from '$lib/stores/websocket-status';

	function formatDelay(ms: number): string {
		return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
	}

	function delayColor(ms: number): string {
		if (ms < 2000) return '#22c55e';
		if (ms <= 10000) return '#eab308';
		return '#ef4444';
	}
</script>

<div class="connection-indicator">
	{#if $websocketStatus.connected}
		{@const delay = $websocketStatus.delayMs}
		<div
			class="indicator connected"
			title="WebSocket connected{delay !== null ? `, delay: ${formatDelay(delay)}` : ''}"
		>
			<Wifi size={14} />
			<span class="label">Live</span>
			<span class="delay" style:color={delay !== null ? delayColor(delay) : '#22c55e'}>
				{delay !== null ? formatDelay(delay) : ''}
			</span>
		</div>
	{:else if $websocketStatus.reconnecting}
		<div class="indicator reconnecting" title="Reconnecting...">
			<RotateCcw size={14} class="spin" />
			<span class="label">Reconnecting</span>
			<span class="delay"></span>
		</div>
	{:else}
		<div class="indicator disconnected" title={$websocketStatus.error ?? 'Disconnected'}>
			<WifiOff size={14} />
			<span class="label">Offline</span>
			<span class="delay"></span>
		</div>
	{/if}
</div>

<style>
	.connection-indicator {
		flex-shrink: 0;
	}

	.indicator {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0.5rem;
		border-radius: 0.375rem;
		font-size: 0.6875rem;
		font-weight: 600;
		white-space: nowrap;
	}

	.indicator.connected {
		background: rgba(34, 197, 94, 0.15);
		color: #22c55e;
	}

	.indicator.reconnecting {
		background: rgba(234, 179, 8, 0.15);
		color: #eab308;
	}

	.indicator.disconnected {
		background: rgba(239, 68, 68, 0.15);
		color: #ef4444;
	}

	.label {
		width: 2.25rem;
	}

	.delay {
		width: 2.75rem;
		text-align: right;
	}

	:global(.spin) {
		animation: spin 1s linear infinite;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
