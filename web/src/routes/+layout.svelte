<script lang="ts">
	import '../app.css';
	import { AppBar, Toaster } from '@skeletonlabs/skeleton-svelte';
	import { toaster } from '$lib/toaster';
	import favicon from '$lib/assets/favicon.svg';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth';
	import { theme } from '$lib/stores/theme';
	import { websocketStatus, debugStatus } from '$lib/stores/watchlist';
	import { onMount } from 'svelte';
	import LoadingBar from '$lib/components/LoadingBar.svelte';
	import {
		Radar,
		Users,
		Plane,
		Radio,
		Antenna,
		MapPin,
		Wifi,
		WifiOff,
		RotateCcw,
		AlertCircle,
		Menu,
		X,
		LogIn,
		UserPlus as SignUp,
		User,
		Sun,
		Moon
	} from '@lucide/svelte';

	const base = resolve('/');
	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');
	const devicesPath = resolve('/devices');
	const receiversPath = resolve('/receivers');
	const airportsPath = resolve('/airports');
	const flightsPath = resolve('/flights');
	const loginPath = resolve('/login');
	const registerPath = resolve('/register');
	const profilePath = resolve('/profile');

	let { children } = $props();
	let showUserMenu = $state(false);
	let showMobileMenu = $state(false);

	// Initialize auth and theme from localStorage on mount
	onMount(() => {
		auth.initFromStorage();
		theme.init();

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

	// Close menus when clicking outside
	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.user-menu')) {
			showUserMenu = false;
		}
		if (!target.closest('.mobile-menu') && !target.closest('.mobile-menu-button')) {
			showMobileMenu = false;
		}
	}
</script>

<svelte:head>
	<title>Glider Flights - Soaring Club Directory</title>
	<meta name="description" content="Find soaring clubs and airports near you" />
	<link rel="icon" href={favicon} />
</svelte:head>

<div class="flex h-full min-h-screen flex-col">
	<AppBar classes="preset-glass-neutral relative z-[70]">
		{#snippet lead()}
			<a href={base} class="flex items-center space-x-2">
				<div class="flex items-center gap-3 text-xl font-bold text-primary-500">
					<Plane />
					Glider.flights
				</div>
			</a>
		{/snippet}
		{#snippet trail()}
			<!-- Desktop Navigation -->
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
				<a href={receiversPath} class="btn preset-filled-primary-500 btn-sm">
					<Antenna /> Receivers
				</a>
				<a href={airportsPath} class="btn preset-filled-primary-500 btn-sm">
					<MapPin /> Airports
				</a>
				<a href={flightsPath} class="btn preset-filled-primary-500 btn-sm">
					<Plane /> Flights
				</a>
			</nav>

			<!-- Theme Toggle -->
			<button
				class="preset-tonal-surface-500 btn btn-sm"
				onclick={() => theme.toggle()}
				title={$theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
			>
				{#if $theme === 'dark'}
					<Sun size={18} />
				{:else}
					<Moon size={18} />
				{/if}
			</button>

			<!-- Desktop Auth -->
			<div class="hidden md:flex">
				{#if $auth.isAuthenticated && $auth.user}
					<div class="user-menu relative">
						<button
							class="btn preset-filled-primary-500 btn-sm"
							onclick={() => (showUserMenu = !showUserMenu)}
						>
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
										<User size={16} /> Profile
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
						<a href={loginPath} class="btn preset-filled-primary-500 btn-sm"><LogIn /> Login</a>
						<a href={registerPath} class="btn preset-filled-primary-500 btn-sm"
							><SignUp /> Sign Up</a
						>
					</div>
				{/if}
			</div>

			<!-- Mobile Hamburger Menu -->
			<div class="md:hidden">
				<button
					class="mobile-menu-button preset-tonal-surface-500 btn p-2 btn-sm"
					onclick={(e) => {
						e.stopPropagation();
						showMobileMenu = !showMobileMenu;
					}}
				>
					{#if showMobileMenu}
						<X size={20} />
					{:else}
						<Menu size={20} />
					{/if}
				</button>
			</div>
		{/snippet}
		<!-- WebSocket Status Indicator for larger screens -->
		<div class="hidden w-full items-center justify-center lg:flex">
			{#if $websocketStatus.connected}
				<div
					class="flex items-center space-x-1 rounded bg-success-500/20 px-2 py-1 text-success-600 dark:text-success-400"
					title="Connected - Tracking {$debugStatus.activeWatchlistEntries
						.length} from watchlist, {$debugStatus.subscribedDevices
						.length} device subscriptions, {$debugStatus.activeAreaSubscriptions} area subscriptions{$debugStatus.operationsPageActive
						? ', Operations page active'
						: ''}"
				>
					<Wifi size={16} />
					<span class="text-xs font-medium">Live</span>
					{#if $debugStatus.activeWatchlistEntries.length > 0 || $debugStatus.activeAreaSubscriptions > 0}
						<span class="text-xs font-medium">
							({#if $debugStatus.activeWatchlistEntries.length > 0}{$debugStatus
									.activeWatchlistEntries
									.length}{/if}{#if $debugStatus.activeWatchlistEntries.length > 0 && $debugStatus.activeAreaSubscriptions > 0}+{/if}{#if $debugStatus.activeAreaSubscriptions > 0}{$debugStatus.activeAreaSubscriptions}
								area{/if})
						</span>
					{/if}
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
		<LoadingBar />
	</AppBar>

	<!-- Mobile Menu Overlay -->
	{#if showMobileMenu}
		<div
			class="mobile-menu bg-surface-50-900-token border-surface-200-700-token bg-opacity-95 dark:bg-opacity-95 fixed inset-x-0 top-0 z-[60] min-h-screen border-b pt-16 shadow-lg backdrop-blur-sm md:hidden"
		>
			<nav class="flex flex-col space-y-4 p-6">
				<a
					href={clubsPath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<Users size={16} /> Clubs
				</a>
				<a
					href={operationsPath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<Radar size={16} /> Operations
				</a>
				<a
					href={devicesPath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<Radio size={16} /> Devices
				</a>
				<a
					href={receiversPath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<Antenna size={16} /> Receivers
				</a>
				<a
					href={airportsPath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<MapPin size={16} /> Airports
				</a>
				<a
					href={flightsPath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<Plane size={16} /> Flights
				</a>

				{#if $auth.isAuthenticated && $auth.user}
					<div class="space-y-4">
						<a
							href={profilePath}
							class="btn w-full justify-start preset-filled-primary-500"
							onclick={() => (showMobileMenu = false)}
						>
							<User size={16} /> Profile
						</a>
						<button
							class="btn w-full justify-start preset-filled-primary-500"
							onclick={() => {
								handleLogout();
								showMobileMenu = false;
							}}
						>
							Sign out
						</button>
					</div>
				{:else}
					<div class="space-y-4">
						<a
							href={loginPath}
							class="btn w-full justify-start preset-filled-primary-500"
							onclick={() => (showMobileMenu = false)}
						>
							<LogIn size={16} /> Login
						</a>
						<a
							href={registerPath}
							class="btn w-full justify-start preset-filled-primary-500"
							onclick={() => (showMobileMenu = false)}
						>
							<SignUp size={16} /> Sign Up
						</a>
					</div>
				{/if}
				<!-- Mobile Theme Toggle -->
				<button
					class="btn w-full justify-start preset-filled-surface-500"
					onclick={() => theme.toggle()}
				>
					{#if $theme === 'dark'}
						<Sun size={16} /> Light Mode
					{:else}
						<Moon size={16} /> Dark Mode
					{/if}
				</button>
			</nav>
		</div>
	{/if}

	<main class="container mx-auto flex-1 space-y-4">
		{@render children?.()}
	</main>

	{#if !page.route.id?.includes('operations')}
		<footer class="bg-surface-100-800-token p-4 text-center text-sm">
			<p>&copy; 2025 Liam Bowen</p>
		</footer>
	{/if}
</div>

<Toaster {toaster}></Toaster>
