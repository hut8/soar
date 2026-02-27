<script lang="ts">
	import { goto } from '$app/navigation';
	import { XCircle } from '@lucide/svelte';
</script>

<div class="container mx-auto max-w-lg p-4">
	<div class="card p-8 text-center">
		<XCircle class="mx-auto mb-4 h-16 w-16 text-warning-500" />
		<h1 class="mb-4 h2">Payment Canceled</h1>
		<p class="mb-6 opacity-75">
			Your payment was not completed. You can try again at any time from the payments page.
		</p>
		<div class="flex justify-center gap-4">
			<button onclick={() => goto('/payments')} class="variant-filled-primary btn">
				View Payments
			</button>
			<button onclick={() => goto('/')} class="variant-ghost-surface btn"> Go Home </button>
		</div>
	</div>
</div>
