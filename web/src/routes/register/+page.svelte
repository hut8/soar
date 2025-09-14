<script lang="ts">
	import { goto } from '$app/navigation';
	import { authApi, AuthApiError } from '$lib/api/auth';
	import { resolve } from '$app/paths';
	import { ClubSelector } from '$lib';
	import { page } from '$app/state';

	let firstName = '';
	let lastName = '';
	let email = '';
	let password = '';
	let confirmPassword = '';
	let selectedClub: string[] = [];
	let error = '';
	let loading = false;

	// Handle club selection
	function handleClubChange(e: { value: string[] }) {
		selectedClub = e.value;
	}

	// Get the club ID for registration
	$: clubId = selectedClub.length > 0 ? selectedClub[0] : '';

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
			await authApi.register({
				first_name: firstName,
				last_name: lastName,
				email,
				password,
				club_id: clubId || undefined
			});

			const message = 'Email verified successfully. Please log in.';
            const href = `/login?message=${encodeURIComponent(message)}`;
			void goto(href);
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

			<div class="label">
				<ClubSelector
					value={selectedClub}
					onValueChange={handleClubChange}
					label="Club (Optional)"
					placeholder="Select a club (optional)"
					disabled={loading}
				/>
			</div>

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
