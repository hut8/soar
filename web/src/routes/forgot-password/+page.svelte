<script lang="ts">
	import { authApi, AuthApiError } from '$lib/api/auth';
	import { resolve } from '$app/paths';

	let email = '';
	let error = '';
	let success = false;
	let loading = false;

	async function handleRequestReset() {
		if (!email) {
			error = 'Please enter your email address';
			return;
		}

		loading = true;
		error = '';

		try {
			await authApi.requestPasswordReset({ email });
			success = true;
		} catch (err) {
			if (err instanceof AuthApiError) {
				error = err.message;
			} else {
				error = 'Failed to send reset email. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Forgot Password - Glider Flights</title>
</svelte:head>

<div class="mx-auto max-w-md space-y-6 py-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold">Forgot Password</h1>
		<p class="text-surface-600-300-token mt-2">Enter your email to receive a password reset link</p>
	</div>

	<div class="card p-6">
		{#if success}
			<div class="mb-4 rounded-lg preset-filled-success-500 p-4 text-sm">
				<div class="font-medium">Check your email!</div>
				<div class="mt-1">
					We've sent a password reset link to <strong>{email}</strong>
				</div>
			</div>

			<div class="text-center">
				<a href={resolve('/login')} class="anchor"> ← Back to login </a>
			</div>
		{:else}
			{#if error}
				<div class="mb-4 rounded-lg preset-filled-error-500 p-3 text-sm">
					{error}
				</div>
			{/if}

			<form on:submit|preventDefault={handleRequestReset} class="space-y-4">
				<label class="label">
					<span>Email Address</span>
					<input
						class="input"
						type="email"
						placeholder="Enter your email"
						bind:value={email}
						disabled={loading}
						required
					/>
				</label>

				<button type="submit" class="preset-filled-primary btn w-full" disabled={loading}>
					{loading ? 'Sending...' : 'Send Reset Link'}
				</button>
			</form>

			<div class="mt-6 text-center text-sm">
				Remember your password?
				<a href={resolve('/login')} class="anchor">Sign in here</a>
			</div>
		{/if}
	</div>
</div>
