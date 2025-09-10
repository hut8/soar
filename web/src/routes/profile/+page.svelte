<script lang="ts">
	import { auth } from '$lib/stores/auth';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';

	// Redirect if not authenticated
	onMount(() => {
		if (!$auth.isAuthenticated) {
			goto(resolve('/login'));
		}
	});
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

		<div class="grid gap-6 md:grid-cols-2">
			<!-- User Info Card -->
			<div class="space-y-4 card p-6">
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
								<span class="variant-filled-warning ml-2 badge">Unverified</span>
							{:else}
								<span class="variant-filled-success ml-2 badge">Verified</span>
							{/if}
						</div>
					</div>

					<div>
						<div class="text-surface-700-200-token capitalize">
							Access Level: {$auth.user.access_level}
							{#if $auth.user.access_level === 'admin'}
								<span class="variant-filled-primary ml-2 badge">Administrator</span>
							{/if}
						</div>
					</div>

					{#if $auth.user.club_id}
						<div>
							<div class="text-surface-700-200-token">
								Member of club ID: {$auth.user.club_id}
							</div>
						</div>
					{:else}
						<div>
							<div class="text-surface-600-300-token">Not associated with a club</div>
						</div>
					{/if}
				</div>
			</div>

			<!-- Quick Actions Card -->
			<div class="space-y-4 card p-6">
				<h2 class="text-xl font-semibold">Quick Actions</h2>

				<div class="space-y-3">
					<a href={resolve('/clubs')} class="variant-ghost-primary btn w-full justify-start">
						🏢 Browse Clubs
					</a>
					<a href={resolve('/operations')} class="variant-ghost-primary btn w-full justify-start">
						🗺️ View Operations Map
					</a>

					{#if $auth.user.access_level === 'admin'}
						<hr class="!my-4" />
						<div class="text-surface-600-300-token text-sm font-medium">Administrator Tools</div>
						<button class="variant-ghost-secondary btn w-full justify-start" disabled>
							👥 Manage Users (Coming Soon)
						</button>
					{/if}
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
				<div class="text-lg font-semibold capitalize">
					{$auth.user.access_level}
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
		<a href={resolve('/login')} class="variant-filled-primary mt-4 btn"> Login </a>
	</div>
{/if}
