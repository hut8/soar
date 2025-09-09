<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { authApi, AuthApiError } from '$lib/api/auth';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';

	let firstName = '';
	let lastName = '';
	let email = '';
	let password = '';
	let confirmPassword = '';
	let clubId = '';
	let clubs: Array<{ id: string; name: string }> = [];
	let error = '';
	let loading = false;
	let clubsLoading = true;

	onMount(async () => {
		try {
			// Load clubs for selection
			const response = await fetch('/clubs?limit=100');
			if (response.ok) {
				clubs = await response.json();
			}
		} catch (err) {
			console.warn('Failed to load clubs:', err);
		} finally {
			clubsLoading = false;
		}
	});

	async function handleRegister() {
		// Validation
		if (!firstName || !lastName || !email || !password) {
			error = 'Please fill in all required fields';
			return;
		}

		if (password !== confirmPassword) {
			error = 'Passwords do not match';
			return;
		}

		if (password.length < 8) {
			error = 'Password must be at least 8 characters long';
			return;
		}

		loading = true;
		error = '';

		try {
			const user = await authApi.register({
				first_name: firstName,
				last_name: lastName,
				email,
				password,
				club_id: clubId || undefined
			});

			// Auto-login after successful registration
			const loginResponse = await authApi.login({ email, password });
			auth.login(loginResponse.user, loginResponse.token);

			// Redirect to home page
			goto(resolve('/'));
		} catch (err) {
			if (err instanceof AuthApiError) {
				if (err.status === 409) {
					error = 'An account with this email already exists';
				} else {
					error = err.message;
				}
			} else {
				error = 'Registration failed. Please try again.';
			}
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Register - Glider Flights</title>
</svelte:head>

<div class="mx-auto max-w-md space-y-6 py-8">
	<div class="text-center">
		<h1 class="text-3xl font-bold">Create Account</h1>
		<p class="text-surface-600-300-token mt-2">Join the soaring community</p>
	</div>

	<div class="card p-6">
		{#if error}
			<div class="variant-filled-error mb-4 rounded-lg p-3 text-sm">
				{error}
			</div>
		{/if}

		<form on:submit|preventDefault={handleRegister} class="space-y-4">
			<div class="grid grid-cols-2 gap-4">
				<label class="label">
					<span>First Name *</span>
					<input
						class="input"
						type="text"
						placeholder="First name"
						bind:value={firstName}
						disabled={loading}
						required
					/>
				</label>

				<label class="label">
					<span>Last Name *</span>
					<input
						class="input"
						type="text"
						placeholder="Last name"
						bind:value={lastName}
						disabled={loading}
						required
					/>
				</label>
			</div>

			<label class="label">
				<span>Email *</span>
				<input
					class="input"
					type="email"
					placeholder="Enter your email"
					bind:value={email}
					disabled={loading}
					required
				/>
			</label>

			<label class="label">
				<span>Club (Optional)</span>
				<select class="select" bind:value={clubId} disabled={loading || clubsLoading}>
					<option value="">Select a club (optional)</option>
					{#each clubs as club}
						<option value={club.id}>{club.name}</option>
					{/each}
				</select>
				{#if clubsLoading}
					<div class="text-surface-600-300-token text-xs">Loading clubs...</div>
				{/if}
			</label>

			<label class="label">
				<span>Password *</span>
				<input
					class="input"
					type="password"
					placeholder="Enter your password"
					bind:value={password}
					disabled={loading}
					required
				/>
				<div class="text-surface-600-300-token text-xs">Must be at least 8 characters long</div>
			</label>

			<label class="label">
				<span>Confirm Password *</span>
				<input
					class="input"
					type="password"
					placeholder="Confirm your password"
					bind:value={confirmPassword}
					disabled={loading}
					required
				/>
			</label>

			<button type="submit" class="variant-filled-primary btn w-full" disabled={loading}>
				{loading ? 'Creating Account...' : 'Create Account'}
			</button>
		</form>

		<div class="mt-6 text-center text-sm">
			Already have an account?
			<a href={resolve('/login')} class="anchor">Sign in here</a>
		</div>
	</div>
</div>
