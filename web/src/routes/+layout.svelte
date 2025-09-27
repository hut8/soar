<script lang="ts">
	import '../app.css';
	import { AppBar, Avatar } from '@skeletonlabs/skeleton-svelte';
	import favicon from '$lib/assets/favicon.svg';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth';
	import { websocketStatus } from '$lib/stores/watchlist';
	import { onMount } from 'svelte';
	import {
		Radar,
		Users,
		Plane,
		UserPlus,
		UserCheck,
		Radio,
		Wifi,
		WifiOff,
		RotateCcw,
		AlertCircle
	} from '@lucide/svelte';

	const base = resolve('/');
	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');
	const devicesPath = resolve('/devices');
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

<div class="flex min-h-screen flex-col">
	<AppBar classes="preset-glass-neutral">
		{#snippet lead()}
			<a href={base} class="flex items-center space-x-2">
				<div class="flex items-center gap-3 text-xl font-bold text-primary-500">
					<Plane />
					Glider.flights
				</div>
			</a>
		{/snippet}
		{#snippet trail()}
			<nav class="hidden space-x-4 md:flex">
				<a href={clubsPath} class="btn preset-filled-primary-500 btn-sm">
					<Users /> Clubs
				</a>
				<a href={operationsPath} class="btn preset-filled-primary-500 btn-sm">
					<Radar /> Operations
				</a>
				<a href={devicesPath} class="btn preset-filled-primary-500 btn-sm">
					<Radio /> Devices
				</a>
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
						/>
						<span class="hidden sm:inline">{$auth.user.first_name}</span>
					</button>

					{#if showUserMenu}
						<div class="absolute top-12 right-0 z-10 w-48 card preset-filled-primary-50-950 p-2">
							<div class="space-y-1">
								<div class="px-3 py-2 text-sm">
									<div class="font-medium">{$auth.user.first_name} {$auth.user.last_name}</div>
									<div class="text-surface-600-300-token">{$auth.user.email}</div>
								</div>
								<hr class="!my-2" />
								<a
									href={profilePath}
									class="btn w-full justify-start preset-filled-primary-500 btn-sm"
								>
									👤 Profile
								</a>
								<button
									class="btn w-full justify-start preset-filled-primary-500 btn-sm"
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
					<a href={loginPath} class="btn preset-filled-primary-500 btn-sm"><UserCheck /> Login</a>
					<a href={registerPath} class="btn preset-filled-primary-500 btn-sm"
						><UserPlus /> Sign Up</a
					>
				</div>
			{/if}
		{/snippet}
		<!-- WebSocket Status Indicator for larger screens -->
		<div class="hidden w-full items-center justify-center lg:flex">
			{#if $websocketStatus.connected}
				<div
					class="flex items-center space-x-1 rounded bg-success-500/20 px-2 py-1 text-success-600 dark:text-success-400"
				>
					<Wifi size={16} />
					<span class="text-xs font-medium">Live</span>
				</div>
			{:else if $websocketStatus.reconnecting}
				<div
					class="flex items-center space-x-1 rounded bg-warning-500/20 px-2 py-1 text-warning-600 dark:text-warning-400"
				>
					<RotateCcw size={16} class="animate-spin" />
					<span class="text-xs font-medium">Reconnecting</span>
				</div>
			{:else if $websocketStatus.error}
				<div
					class="flex items-center space-x-1 rounded bg-error-500/20 px-2 py-1 text-error-600 dark:text-error-400"
					title={$websocketStatus.error}
				>
					<AlertCircle size={16} />
					<span class="text-xs font-medium">Offline</span>
				</div>
			{:else}
				<div
					class="flex items-center space-x-1 rounded bg-surface-400/20 px-2 py-1 text-surface-600 dark:text-surface-400"
				>
					<WifiOff size={16} />
					<span class="text-xs font-medium">Disconnected</span>
				</div>
			{/if}
		</div>
	</AppBar>

	<main class="container mx-auto flex-1 space-y-4 p-4">
		{@render children?.()}
	</main>

	{#if !page.route.id?.includes('operations')}
		<footer class="bg-surface-100-800-token p-4 text-center text-sm">
			<p>&copy; 2025 Liam Bowen</p>
		</footer>
	{/if}
</div>
