<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { auth } from '$lib/stores/auth';
	import { authApi, AuthApiError } from '$lib/api/auth';
	import { resolve } from '$app/paths';

	let email = '';
	let password = '';
	let error = '';
	let loading = false;
	let message = '';

	onMount(() => {
		// Check for success message from URL parameters
		const urlMessage = $page.url.searchParams.get('message');
		if (urlMessage) {
			message = urlMessage;
		}
	});

	async function handleLogin() {
		if (!email || !password) {
			error = 'Please fill in all fields';
			return;
		}

		loading = true;
		error = '';

		try {
			const response = await authApi.login({ email, password });
			auth.login(response.user, response.token);

			// Redirect to home page after successful login
			goto(resolve('/'));
		} catch (err) {
			if (err instanceof AuthApiError) {
				if (err.status === 401) {
					error = 'Invalid email or password';
				} else {
					error = err.message;
				}
			} else {
				error = 'Login failed. Please try again.';
			}
		} finally {
			loading = false;
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter') {
			handleLogin();
		}
	}
</script>

<svelte:head>
	<title>Login - Glider Flights</title>
</svelte:head>

<div class="mx-auto max-w-md space-y-6 py-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold">Login</h1>
		<p class="text-surface-600-300-token mt-2">Sign in to your account</p>
	</div>

	<div class="card p-6">
		{#if message}
			<div class="variant-filled-success mb-4 rounded-lg p-3 text-sm">
				{message}
			</div>
		{/if}

		{#if error}
			<div class="variant-filled-error mb-4 rounded-lg p-3 text-sm">
				{error}
			</div>
		{/if}

		<form on:submit|preventDefault={handleLogin} class="space-y-4">
			<label class="label">
				<span>Email</span>
				<input
					class="input"
					type="email"
					placeholder="Enter your email"
					bind:value={email}
					on:keydown={handleKeydown}
					disabled={loading}
					required
				/>
			</label>

			<label class="label">
				<span>Password</span>
				<input
					class="input"
					type="password"
					placeholder="Enter your password"
					bind:value={password}
					on:keydown={handleKeydown}
					disabled={loading}
					required
				/>
			</label>

			<button type="submit" class="variant-filled-primary btn w-full" disabled={loading}>
				{loading ? 'Signing in...' : 'Sign In'}
			</button>
		</form>

		<div class="mt-6 space-y-2 text-center text-sm">
			<div>
				<a href={resolve('/forgot-password')} class="anchor"> Forgot your password? </a>
			</div>
			<div>
				Don't have an account?
				<a href={resolve('/register')} class="anchor">Sign up here</a>
			</div>
		</div>
	</div>
</div>
