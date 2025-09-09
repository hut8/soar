<script lang="ts">
	import '../app.css';
	import { AppBar, Avatar } from '@skeletonlabs/skeleton-svelte';
	import favicon from '$lib/assets/favicon.svg';
	import { resolve } from '$app/paths';
	import { auth } from '$lib/stores/auth';
	import { onMount } from 'svelte';

	const base = resolve('/');
	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');
	const loginPath = resolve('/login');
	const registerPath = resolve('/register');
	const profilePath = resolve('/profile');

	let { children } = $props();
	let showUserMenu = $state(false);

	// Initialize auth from localStorage on mount
	onMount(() => {
		auth.initFromStorage();

		// Add click outside listener
		document.addEventListener('click', handleClickOutside);
		return () => {
			document.removeEventListener('click', handleClickOutside);
		};
	});

	function handleLogout() {
		auth.logout();
		showUserMenu = false;
	}

	function getInitials(firstName: string, lastName: string): string {
		return `${firstName.charAt(0)}${lastName.charAt(0)}`.toUpperCase();
	}

	// Close user menu when clicking outside
	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.user-menu')) {
			showUserMenu = false;
		}
	}
</script>

<svelte:head>
	<title>Glider Flights - Soaring Club Directory</title>
	<meta name="description" content="Find soaring clubs and airports near you" />
	<link rel="icon" href={favicon} />
</svelte:head>

<AppBar>
	{#snippet lead()}
		<a href={base} class="flex items-center space-x-2">
			<div class="text-xl font-bold text-primary-500">✈️ Glider.flights</div>
		</a>
	{/snippet}
	{#snippet trail()}
		<nav class="hidden space-x-4 md:flex">
			<a href={clubsPath} class="variant-ghost-surface btn btn-sm">Clubs</a>
			<a href={operationsPath} class="variant-ghost-surface btn btn-sm">🗺️ Operations</a>
		</nav>

		{#if $auth.isAuthenticated && $auth.user}
			<div class="user-menu relative">
				<button
					class="variant-ghost-surface btn flex items-center space-x-2 btn-sm"
					onclick={() => (showUserMenu = !showUserMenu)}
				>
					<Avatar
						initials={[0, 1]}
						background="bg-primary-500"
						name="{$auth.user.first_name} {$auth.user.last_name}"
						size="sm"
					/>
					<span class="hidden sm:inline">{$auth.user.first_name}</span>
				</button>

				{#if showUserMenu}
					<div class="absolute top-12 right-0 z-10 w-48 card p-2">
						<div class="space-y-1">
							<div class="px-3 py-2 text-sm">
								<div class="font-medium">{$auth.user.first_name} {$auth.user.last_name}</div>
								<div class="text-surface-600-300-token">{$auth.user.email}</div>
							</div>
							<hr class="!my-2" />
							<a href={profilePath} class="variant-ghost-surface btn w-full justify-start btn-sm">
								👤 Profile
							</a>
							<button
								class="variant-ghost-error btn w-full justify-start btn-sm"
								onclick={handleLogout}
							>
								Sign out
							</button>
						</div>
					</div>
				{/if}
			</div>
		{:else}
			<div class="flex space-x-2">
				<a href={loginPath} class="variant-ghost-surface btn btn-sm">Login</a>
				<a href={registerPath} class="variant-filled-primary btn btn-sm">Sign Up</a>
			</div>
		{/if}
	{/snippet}
</AppBar>

<main class="container mx-auto space-y-4 p-4">
	{@render children?.()}
</main>

<footer class="bg-surface-100-800-token p-4 text-center text-sm">
	<p>&copy; 2025 Liam Bowen</p>
</footer>
