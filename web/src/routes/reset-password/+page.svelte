<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { authApi, AuthApiError } from '$lib/api/auth';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';

	let token = '';
	let newPassword = '';
	let confirmPassword = '';
	let error = '';
	let success = false;
	let loading = false;

	onMount(() => {
		// Get token from URL params
		const urlToken = $page.url.searchParams.get('token');
		if (urlToken) {
			token = urlToken;
		} else {
			error = 'Invalid reset link. Please request a new password reset.';
		}
	});

	async function handleResetPassword() {
		if (!token) {
			error = 'Invalid reset token';
			return;
		}

		if (!newPassword) {
			error = 'Please enter a new password';
			return;
		}

		if (newPassword !== confirmPassword) {
			error = 'Passwords do not match';
			return;
		}

		if (newPassword.length < 8) {
			error = 'Password must be at least 8 characters long';
			return;
		}

		loading = true;
		error = '';

		try {
			await authApi.confirmPasswordReset({
				token,
				newPassword: newPassword
			});
			success = true;

			// Redirect to login after a delay
			setTimeout(() => {
				goto(resolve('/login'));
			}, 3000);
		} catch (err) {
			console.error('Password reset error:', err);
			if (err instanceof AuthApiError) {
				if (err.status === 400) {
					error = 'Reset link has expired or is invalid. Please request a new one.';
				} else {
					error = err.message;
				}
			} else {
				const errorMessage = err instanceof Error ? err.message : 'Unknown error';
				error = `Failed to reset password: ${errorMessage}`;
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Reset Password - Glider Flights</title>
</svelte:head>

<div class="mx-auto max-w-md space-y-6 py-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold">Reset Password</h1>
		<p class="text-surface-600-300-token mt-2">Enter your new password</p>
	</div>

	<div class="card p-6">
		{#if success}
			<div class="mb-4 rounded-lg preset-filled-success-500 p-4 text-center text-sm">
				<div class="font-medium">Password updated successfully!</div>
				<div class="mt-1">You will be redirected to the login page...</div>
			</div>

			<div class="text-center">
				<a href={resolve('/login')} class="anchor"> Continue to login </a>
			</div>
		{:else if !token}
			<div class="mb-4 rounded-lg preset-filled-error-500 p-3 text-sm">
				{error}
			</div>

			<div class="text-center">
				<a href={resolve('/forgot-password')} class="anchor"> Request a new password reset </a>
			</div>
		{:else}
			{#if error}
				<div class="mb-4 rounded-lg preset-filled-error-500 p-3 text-sm">
					{error}
				</div>
			{/if}

			<form on:submit|preventDefault={handleResetPassword} class="space-y-4">
				<label class="label">
					<span>New Password</span>
					<input
						class="input"
						type="password"
						placeholder="Enter your new password"
						bind:value={newPassword}
						disabled={loading}
						required
					/>
					<div class="text-surface-600-300-token text-xs">Must be at least 8 characters long</div>
				</label>

				<label class="label">
					<span>Confirm New Password</span>
					<input
						class="input"
						type="password"
						placeholder="Confirm your new password"
						bind:value={confirmPassword}
						disabled={loading}
						required
					/>
				</label>

				<button type="submit" class="btn w-full preset-filled-primary-500" disabled={loading}>
					{loading ? 'Updating Password...' : 'Update Password'}
				</button>
			</form>
		{/if}
	</div>
</div>
