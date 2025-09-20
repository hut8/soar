<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { authApi } from '$lib/api/auth';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';

	let loading = $state(true);
	let success = $state(false);
	let error = $state('');
	let token = $state('');

	onMount(async () => {
		// Get token from URL params
		const urlToken = $page.url.searchParams.get('token');
		if (urlToken) {
			token = urlToken;
			await verifyEmail();
		} else {
			error = 'No verification token provided';
			loading = false;
		}
	});

	async function verifyEmail() {
		try {
			loading = true;
			error = '';

			await authApi.verifyEmail(token);
			success = true;

			// Redirect to login after 3 seconds
			setTimeout(() => {
				const message = 'Email verified successfully. Please log in.';
				goto(`/login?message=${encodeURIComponent(message)}`);
			}, 3000);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to verify email';
			success = false;
		} finally {
			loading = false;
		}
	}
</script>

<div class="container mx-auto max-w-md px-4 py-8">
	<div class="bg-surface-100-800-token rounded-lg p-6 shadow-lg">
		<div class="text-center">
			<h1 class="mb-6 text-2xl font-bold">Email Verification</h1>

			{#if loading}
				<div class="flex flex-col items-center space-y-4">
					<div class="h-12 w-12 animate-spin rounded-full border-b-2 border-primary-500"></div>
					<p>Verifying your email address...</p>
				</div>
			{:else if success}
				<div class="flex flex-col items-center space-y-4">
					<div class="flex h-16 w-16 items-center justify-center rounded-full bg-success-500">
						<svg class="h-8 w-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M5 13l4 4L19 7"
							></path>
						</svg>
					</div>
					<h2 class="text-xl font-semibold text-success-500">Email Verified!</h2>
					<p class="text-surface-600-300-token">
						Your email address has been successfully verified. You will be redirected to the login
						page shortly.
					</p>
				</div>
			{:else if error}
				<div class="flex flex-col items-center space-y-4">
					<div class="flex h-16 w-16 items-center justify-center rounded-full bg-error-500">
						<svg class="h-8 w-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							></path>
						</svg>
					</div>
					<h2 class="text-xl font-semibold text-error-500">Verification Failed</h2>
					<p class="text-surface-600-300-token mb-4">{error}</p>
					<div class="space-y-2">
						<button
							class="variant-filled-primary btn w-full"
							onclick={() => goto(resolve('/login'))}
						>
							Go to Login
						</button>
						<button
							class="variant-soft-surface btn w-full"
							onclick={() => goto(resolve('/register'))}
						>
							Register New Account
						</button>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>
