<script lang="ts">
	import { auth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { serverCall } from '$lib/api/server';
	import type { Club } from '$lib/types';
	import { X, Mail, Trash2 } from '@lucide/svelte';

	let clubName: string | null = null;
	let loadingClub = false;
	let clubError: string | null = null;
	let clearingClub = false;

	// Change email modal state
	let showChangeEmailModal = false;
	let newEmail = '';
	let confirmPassword = '';
	let changingEmail = false;
	let emailChangeError = '';

	// Delete account modal state
	let showDeleteAccountModal = false;
	let deleteConfirmPassword = '';
	let deleteConfirmText = '';
	let deletingAccount = false;
	let deleteAccountError = '';

	// Redirect if not authenticated and load club name
	onMount(() => {
		if (!$auth.isAuthenticated) {
			goto(resolve('/login'));
		} else if ($auth.user?.club_id) {
			loadClubName($auth.user.club_id);
		}
	});

	async function loadClubName(clubId: string) {
		loadingClub = true;
		clubError = null;
		try {
			const club = await serverCall<Club>(`/clubs/${clubId}`);
			clubName = club.name;
		} catch (err) {
			clubError = err instanceof Error ? err.message : 'Failed to load club';
		} finally {
			loadingClub = false;
		}
	}

	async function clearClub() {
		if (!$auth.user) return;

		clearingClub = true;
		try {
			// Make API call to clear club membership
			await serverCall(`/users/${$auth.user.id}/club`, {
				method: 'DELETE'
			});

			// Update local auth state
			auth.updateUser({
				...$auth.user,
				club_id: undefined
			});

			clubName = null;
		} catch (err) {
			alert(err instanceof Error ? err.message : 'Failed to clear club membership');
		} finally {
			clearingClub = false;
		}
	}

	function openChangeEmailModal() {
		newEmail = $auth.user?.email || '';
		confirmPassword = '';
		emailChangeError = '';
		showChangeEmailModal = true;
	}

	function closeChangeEmailModal() {
		showChangeEmailModal = false;
		newEmail = '';
		confirmPassword = '';
		emailChangeError = '';
	}

	async function handleChangeEmail() {
		if (!$auth.user) return;

		emailChangeError = '';

		if (!newEmail || !newEmail.includes('@')) {
			emailChangeError = 'Please enter a valid email address';
			return;
		}

		if (!confirmPassword) {
			emailChangeError = 'Please enter your password to confirm';
			return;
		}

		changingEmail = true;
		try {
			await serverCall(`/users/${$auth.user.id}/email`, {
				method: 'PUT',
				body: JSON.stringify({
					email: newEmail,
					password: confirmPassword
				})
			});

			// Update local auth state
			auth.updateUser({
				...$auth.user,
				email: newEmail,
				email_verified: false // Email needs to be verified again
			});

			closeChangeEmailModal();
			alert('Email updated successfully. Please check your new email for verification.');
		} catch (err) {
			emailChangeError = err instanceof Error ? err.message : 'Failed to update email';
		} finally {
			changingEmail = false;
		}
	}

	function openDeleteAccountModal() {
		deleteConfirmPassword = '';
		deleteConfirmText = '';
		deleteAccountError = '';
		showDeleteAccountModal = true;
	}

	function closeDeleteAccountModal() {
		showDeleteAccountModal = false;
		deleteConfirmPassword = '';
		deleteConfirmText = '';
		deleteAccountError = '';
	}

	async function handleDeleteAccount() {
		if (!$auth.user) return;

		deleteAccountError = '';

		if (!deleteConfirmPassword) {
			deleteAccountError = 'Please enter your password to confirm';
			return;
		}

		if (deleteConfirmText.toLowerCase() !== 'delete') {
			deleteAccountError = 'Please type "DELETE" to confirm account deletion';
			return;
		}

		deletingAccount = true;
		try {
			await serverCall(`/users/${$auth.user.id}`, {
				method: 'DELETE',
				body: JSON.stringify({
					password: deleteConfirmPassword
				})
			});

			// Log out and redirect
			auth.logout();
			goto(resolve('/'));
		} catch (err) {
			deleteAccountError = err instanceof Error ? err.message : 'Failed to delete account';
		} finally {
			deletingAccount = false;
		}
	}
</script>

<svelte:head>
	<title>Profile - Glider Flights</title>
</svelte:head>

{#if $auth.isAuthenticated && $auth.user}
	<div class="space-y-6">
		<div class="text-center">
			<h1 class="text-3xl font-bold">Welcome, {$auth.user.first_name}!</h1>
			<p class="text-surface-600-300-token mt-2">Your account dashboard</p>
		</div>

		<!-- User Info Card -->
		<div class="card mx-auto max-w-2xl space-y-4 p-6">
			<h2 class="text-xl font-semibold">Account Information</h2>

			<div class="space-y-3">
				<div>
					<div class="text-surface-700-200-token">
						Full name:
						{$auth.user.first_name}
						{$auth.user.last_name}
					</div>
				</div>

				<div>
					<div class="text-surface-700-200-token">
						Email: {$auth.user.email}
						{#if !$auth.user.email_verified}
							<span class="badge preset-filled-warning-500 ml-2">Unverified</span>
						{:else}
							<span class="badge preset-filled-success-500 ml-2">Verified</span>
						{/if}
					</div>
				</div>

				<div>
					<div class="text-surface-700-200-token">
						Access Level: {$auth.user.access_level === 'admin' ? 'Admin' : 'User'}
						{#if $auth.user.access_level === 'admin'}
							<span class="badge preset-filled-primary-500 ml-2">Administrator</span>
						{/if}
					</div>
				</div>

				{#if $auth.user.club_id}
					<div>
						<div class="text-surface-700-200-token">
							Member of club:
							{#if loadingClub}
								<span class="text-surface-500">Loading...</span>
							{:else if clubError}
								<span class="text-error-500">{clubError}</span>
							{:else if clubName}
								<a
									href={resolve(`/clubs/${$auth.user.club_id}`)}
									class="text-primary-500 hover:text-primary-600 underline"
								>
									{clubName}
								</a>
							{:else}
								<span class="text-surface-500">Unknown</span>
							{/if}
						</div>
						<button
							onclick={clearClub}
							disabled={clearingClub}
							class="btn preset-filled-error-500 btn-sm mt-2"
							title="Clear club membership"
						>
							<X class="h-3 w-3" />
							{clearingClub ? 'Clearing...' : 'Clear Club'}
						</button>
					</div>
				{:else}
					<div>
						<div class="text-surface-600-300-token">Not associated with a club</div>
					</div>
				{/if}
			</div>

			<!-- Account Actions -->
			<div class="border-surface-300-600-token space-y-2 border-t pt-4">
				<h3 class="text-surface-700-200-token text-sm font-semibold">Account Actions</h3>
				<div class="flex flex-col gap-2 sm:flex-row">
					<button onclick={openChangeEmailModal} class="btn preset-filled-primary-500 btn-sm">
						<Mail class="h-4 w-4" />
						Change Email
					</button>
					<button onclick={openDeleteAccountModal} class="btn preset-filled-error-500 btn-sm">
						<Trash2 class="h-4 w-4" />
						Delete Account
					</button>
				</div>
			</div>
		</div>

		<!-- Status Cards -->
		<div class="grid gap-4 sm:grid-cols-3">
			<div class="card p-4 text-center">
				<div class="text-surface-600-300-token text-sm">Account Status</div>
				<div class="text-lg font-semibold">
					{$auth.user.email_verified ? 'Verified' : 'Pending Verification'}
				</div>
			</div>

			<div class="card p-4 text-center">
				<div class="text-surface-600-300-token text-sm">Access Level</div>
				<div class="text-lg font-semibold">
					{$auth.user.access_level === 'admin' ? 'Admin' : 'User'}
				</div>
			</div>

			<div class="card p-4 text-center">
				<div class="text-surface-600-300-token text-sm">Club Membership</div>
				<div class="text-lg font-semibold">
					{$auth.user.club_id ? 'Member' : 'Independent'}
				</div>
			</div>
		</div>
	</div>
{:else}
	<div class="text-center">
		<h1 class="text-2xl font-bold">Access Required</h1>
		<p class="text-surface-600-300-token mt-2">Please log in to view your profile.</p>
		<a href={resolve('/login')} class="btn preset-filled-primary-500 mt-4"> Login </a>
	</div>
{/if}

<!-- Change Email Modal -->
{#if showChangeEmailModal}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
		onclick={closeChangeEmailModal}
	>
		<div class="card m-4 w-full max-w-md space-y-4 p-6" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between">
				<h2 class="text-xl font-bold">Change Email</h2>
				<button onclick={closeChangeEmailModal} class="preset-tonal-surface-500 btn btn-sm">
					<X class="h-4 w-4" />
				</button>
			</div>

			<div class="space-y-4">
				<div>
					<label for="new-email" class="label">
						<span>New Email Address</span>
					</label>
					<input
						id="new-email"
						type="email"
						bind:value={newEmail}
						placeholder="your.email@example.com"
						class="input"
						disabled={changingEmail}
					/>
				</div>

				<div>
					<label for="confirm-password" class="label">
						<span>Confirm Password</span>
					</label>
					<input
						id="confirm-password"
						type="password"
						bind:value={confirmPassword}
						placeholder="Enter your password"
						class="input"
						disabled={changingEmail}
					/>
				</div>

				{#if emailChangeError}
					<div class="alert preset-filled-error-500">
						<p>{emailChangeError}</p>
					</div>
				{/if}

				<div class="flex justify-end gap-2">
					<button
						onclick={closeChangeEmailModal}
						class="preset-tonal-surface-500 btn"
						disabled={changingEmail}
					>
						Cancel
					</button>
					<button
						onclick={handleChangeEmail}
						class="btn preset-filled-primary-500"
						disabled={changingEmail}
					>
						{changingEmail ? 'Updating...' : 'Update Email'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}

<!-- Delete Account Modal -->
{#if showDeleteAccountModal}
	<div
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
		onclick={closeDeleteAccountModal}
	>
		<div class="card m-4 w-full max-w-md space-y-4 p-6" onclick={(e) => e.stopPropagation()}>
			<div class="flex items-center justify-between">
				<h2 class="text-error-500 text-xl font-bold">Delete Account</h2>
				<button onclick={closeDeleteAccountModal} class="preset-tonal-surface-500 btn btn-sm">
					<X class="h-4 w-4" />
				</button>
			</div>

			<div class="space-y-4">
				<div class="alert preset-filled-warning-500">
					<p class="font-semibold">Warning: This action cannot be undone!</p>
					<p class="text-sm">All your data will be permanently deleted.</p>
				</div>

				<div>
					<label for="delete-password" class="label">
						<span>Confirm Password</span>
					</label>
					<input
						id="delete-password"
						type="password"
						bind:value={deleteConfirmPassword}
						placeholder="Enter your password"
						class="input"
						disabled={deletingAccount}
					/>
				</div>

				<div>
					<label for="delete-confirm" class="label">
						<span>Type "DELETE" to confirm</span>
					</label>
					<input
						id="delete-confirm"
						type="text"
						bind:value={deleteConfirmText}
						placeholder="DELETE"
						class="input"
						disabled={deletingAccount}
					/>
				</div>

				{#if deleteAccountError}
					<div class="alert preset-filled-error-500">
						<p>{deleteAccountError}</p>
					</div>
				{/if}

				<div class="flex justify-end gap-2">
					<button
						onclick={closeDeleteAccountModal}
						class="preset-tonal-surface-500 btn"
						disabled={deletingAccount}
					>
						Cancel
					</button>
					<button
						onclick={handleDeleteAccount}
						class="btn preset-filled-error-500"
						disabled={deletingAccount}
					>
						{deletingAccount ? 'Deleting...' : 'Delete Account'}
					</button>
				</div>
			</div>
		</div>
	</div>
{/if}
