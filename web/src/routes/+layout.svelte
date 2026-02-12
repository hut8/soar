<script lang="ts">
	import '../app.css';
	import { AppBar, Toast } from '@skeletonlabs/skeleton-svelte';
	import { toaster } from '$lib/toaster';
	import favicon from '$lib/assets/favicon.svg';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { auth } from '$lib/stores/auth';
	import { theme } from '$lib/stores/theme';
	import {
		backendMode,
		BACKEND_LABELS,
		BACKEND_SHORT_LABELS,
		type BackendMode
	} from '$lib/stores/backend';
	import { websocketStatus, debugStatus } from '$lib/stores/websocket-status';
	import { onMount, onDestroy } from 'svelte';
	import { startTracking, stopTracking } from '$lib/services/locationTracker';
	import { dev, browser } from '$app/environment';
	import { isStaging } from '$lib/config';
	import RadarLoader from '$lib/components/RadarLoader.svelte';
	import LoadingBar from '$lib/components/LoadingBar.svelte';
	import BottomLoadingBar from '$lib/components/BottomLoadingBar.svelte';
	import {
		Radar,
		Users,
		Plane,
		PlaneTakeoff,
		Antenna,
		MapPin,
		Globe,
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
		Moon,
		Info,
		Eye
	} from '@lucide/svelte';

	const base = resolve('/');
	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');
	const livePath = resolve('/live');
	const aircraftPath = resolve('/aircraft');
	const receiversPath = resolve('/receivers');
	const airportsPath = resolve('/airports');
	const flightsPath = resolve('/flights');
	const infoPath = resolve('/info');
	const loginPath = resolve('/login');
	const registerPath = resolve('/register');
	const profilePath = resolve('/profile');
	const watchlistPath = resolve('/watchlist');
	const arPath = resolve('/ar');

	let { children } = $props();
	let showUserMenu = $state(false);
	let showMobileMenu = $state(false);
	let showDesktopMenu = $state(false);
	let showBackendDropdown = $state(false);

	// Reactive club operations path
	let clubOpsPath = $derived(
		$auth.user?.clubId ? resolve(`/clubs/${$auth.user.clubId}/operations`) : ''
	);
	let hasClub = $derived($auth.isAuthenticated && !!$auth.user?.clubId);

	// Invert favicon colors for staging environment
	function invertFavicon() {
		if (!isStaging()) return;

		// Find all favicon link elements and apply inversion via canvas
		const faviconLinks = document.querySelectorAll<HTMLLinkElement>(
			'link[rel="icon"], link[rel="apple-touch-icon"]'
		);

		faviconLinks.forEach((link) => {
			const img = new Image();
			img.crossOrigin = 'anonymous';
			img.onload = () => {
				const canvas = document.createElement('canvas');
				canvas.width = img.width || 32;
				canvas.height = img.height || 32;
				const ctx = canvas.getContext('2d');
				if (ctx) {
					ctx.filter = 'invert(1)';
					ctx.drawImage(img, 0, 0);
					link.href = canvas.toDataURL('image/png');
				}
			};
			img.src = link.href;
		});
	}

	// Initialize auth, theme, and backend mode from localStorage on mount
	onMount(() => {
		auth.initFromStorage();
		theme.init();
		backendMode.init();

		// Invert favicon on staging for visual distinction
		invertFavicon();

		// Load Google Analytics only on production domain
		if (browser && window.location.hostname === 'glider.flights') {
			const script = document.createElement('script');
			script.async = true;
			script.src = 'https://www.googletagmanager.com/gtag/js?id=G-DW6KXT6VG1';
			document.head.appendChild(script);

			window.dataLayer = window.dataLayer || [];
			function gtag(...args: unknown[]) {
				window.dataLayer.push(args);
			}
			gtag('js', new Date());
			gtag('config', 'G-DW6KXT6VG1');
		}

		// Add click outside listener
		document.addEventListener('click', handleClickOutside);
		return () => {
			document.removeEventListener('click', handleClickOutside);
		};
	});

	// Start/stop location tracking based on auth state
	$effect(() => {
		if ($auth.isAuthenticated) {
			startTracking();
		} else {
			stopTracking();
		}
	});

	// Clean up location tracking on component destroy
	onDestroy(() => {
		stopTracking();
	});

	function handleLogout() {
		auth.logout();
		showUserMenu = false;
		goto(loginPath);
	}

	function formatDelay(ms: number): string {
		return ms >= 1000 ? `${(ms / 1000).toFixed(1)}s` : `${ms}ms`;
	}

	function delayColorClass(ms: number): string {
		if (ms < 2000) return 'text-success-700 dark:text-success-400';
		if (ms <= 10000) return 'text-warning-700 dark:text-warning-400';
		return 'text-error-700 dark:text-error-400';
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
		if (!target.closest('.desktop-menu') && !target.closest('.desktop-menu-button')) {
			showDesktopMenu = false;
		}
	}
</script>

<svelte:head>
	<title>Glider Flights - Soaring Club Directory</title>
	<meta name="description" content="Find soaring clubs and airports near you" />

	<!-- Favicons -->
	<link rel="icon" href={favicon} />
	<link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png" />
	<link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png" />
	<link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png" />

	<!-- PWA Manifest -->
	<link rel="manifest" href="/manifest.json" />

	<!-- PWA Meta Tags -->
	<meta name="mobile-web-app-capable" content="yes" />
	<meta name="apple-mobile-web-app-capable" content="yes" />
	<meta name="apple-mobile-web-app-status-bar-style" content="default" />
	<meta name="apple-mobile-web-app-title" content="SOAR" />
	<meta name="theme-color" content="#0ea5e9" />

	<!-- Google Analytics is loaded dynamically in onMount for production only -->
</svelte:head>

<!-- AR page gets full-screen treatment without app bar -->
{#if page.route.id === '/ar'}
	{@render children?.()}
{:else}
	<div class="flex h-full min-h-screen flex-col">
		<AppBar
			class="relative z-[70] bg-gradient-to-br from-orange-300 to-orange-500 p-1 dark:bg-gradient-to-br dark:from-red-950 dark:to-red-800"
		>
			<LoadingBar />
			<AppBar.Toolbar class="grid grid-cols-[auto_1fr_auto] gap-3 p-0">
				<AppBar.Lead>
					<a href={base} class="relative z-10 btn preset-filled-primary-500 btn-sm">
						<Plane />
						Glider.flights
					</a>
				</AppBar.Lead>
				<AppBar.Headline class="flex items-center justify-center">
					<!-- WebSocket Status Indicator -->
					<div class="hidden lg:flex">
						{#if $websocketStatus.connected}
							{@const sources = $websocketStatus.connectionSources}
							{@const label =
								sources.ogn && sources.adsb
									? 'Live - ADSB+OGN'
									: sources.adsb
										? 'Live - ADSB'
										: sources.ogn
											? 'Live - OGN'
											: 'Live'}
							{@const delay = $websocketStatus.delayMs}
							<div
								class="flex items-center space-x-1 rounded bg-white/90 px-2 py-1 text-success-700 shadow-sm dark:bg-success-500/20 dark:text-success-400"
								title="{label} - WebSocket connected{delay !== null
									? `, feed delay: ${formatDelay(delay)}`
									: ''}{$debugStatus.operationsPageActive ? ', Operations page active' : ''}"
							>
								<Wifi size={16} />
								<span class="text-xs font-medium">{label}</span>
								{#if delay !== null}
									<span class="text-xs opacity-60">·</span>
									<span class="text-xs font-medium {delayColorClass(delay)}"
										>{formatDelay(delay)}</span
									>
								{/if}
								<RadarLoader />
							</div>
						{:else if $websocketStatus.reconnecting}
							<div
								class="flex items-center space-x-1 rounded bg-white/90 px-2 py-1 text-warning-700 shadow-sm dark:bg-warning-500/20 dark:text-warning-400"
							>
								<RotateCcw size={16} class="animate-spin" />
								<span class="text-xs font-medium">Reconnecting</span>
							</div>
						{:else if $websocketStatus.error}
							<div
								class="flex items-center space-x-1 rounded bg-white/90 px-2 py-1 text-error-700 shadow-sm dark:bg-error-500/20 dark:text-error-400"
								title={$websocketStatus.error}
							>
								<AlertCircle size={16} />
								<span class="text-xs font-medium">Offline</span>
							</div>
						{:else}
							<div
								class="flex items-center space-x-1 rounded bg-white/90 px-2 py-1 text-surface-700 shadow-sm dark:bg-surface-400/20 dark:text-surface-400"
							>
								<WifiOff size={16} />
								<span class="text-xs font-medium">Disconnected</span>
							</div>
						{/if}
					</div>
				</AppBar.Headline>
				<AppBar.Trail class="justify-end">
					<div class="relative z-10 flex items-center gap-4">
						<!-- Desktop Navigation -->
						<nav class="hidden space-x-4 md:flex">
							{#if hasClub}
								<a href={clubOpsPath} class="btn preset-filled-success-500 btn-sm">
									<Radar /> Club Ops
								</a>
							{/if}

							<!-- Desktop Hamburger Menu -->
							<div class="desktop-menu relative">
								<button
									class="desktop-menu-button preset-tonal-surface-500 btn btn-sm"
									onclick={(e) => {
										e.stopPropagation();
										showDesktopMenu = !showDesktopMenu;
									}}
								>
									<Menu size={18} />
								</button>

								{#if showDesktopMenu}
									<div
										class="absolute top-12 left-0 z-10 w-48 card preset-filled-primary-50-950 p-2"
									>
										<div class="space-y-1">
											<a
												href={livePath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<Globe size={16} /> Live Map
											</a>
											<a
												href={clubsPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<Users size={16} /> Clubs
											</a>
											<a
												href={aircraftPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<Plane size={16} /> Aircraft
											</a>
											<a
												href={receiversPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<Antenna size={16} /> Receivers
											</a>
											<a
												href={airportsPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<MapPin size={16} /> Airports
											</a>
											<a
												href={flightsPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<PlaneTakeoff size={16} /> Flights
											</a>
											{#if $auth.isAuthenticated}
												<a
													href={watchlistPath}
													class="btn w-full justify-start preset-filled-primary-500 btn-sm"
													onclick={() => (showDesktopMenu = false)}
												>
													<Eye size={16} /> Watchlist
												</a>
											{/if}
											<a
												href={arPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<Globe size={16} /> AR
											</a>
											<a
												href={infoPath}
												class="btn w-full justify-start preset-filled-primary-500 btn-sm"
												onclick={() => (showDesktopMenu = false)}
											>
												<Info size={16} /> Info
											</a>
										</div>
									</div>
								{/if}
							</div>

							<a href={operationsPath} class="btn preset-filled-primary-500 btn-sm">
								<Radar /> Operations
							</a>
							<!-- 3D Globe link temporarily disabled
						<a href={globePath} class="btn preset-filled-primary-500 btn-sm">
							<Globe /> 3D Globe
						</a>
						-->
						</nav>

						<!-- Backend Selector (Dev Only) -->
						{#if dev}
							<div class="relative">
								<button
									class="preset-tonal-surface-500 btn btn-sm font-mono font-bold"
									onclick={() => (showBackendDropdown = !showBackendDropdown)}
									title="Select backend environment"
								>
									{BACKEND_SHORT_LABELS[$backendMode]}
								</button>
								{#if showBackendDropdown}
									<div
										class="absolute top-10 right-0 z-50 w-40 card preset-filled-surface-100-900 p-1 shadow-lg"
									>
										{#each ['dev', 'staging', 'prod'] as BackendMode[] as mode (mode)}
											<button
												class="btn w-full justify-start text-sm {$backendMode === mode
													? 'preset-filled-primary-500'
													: 'preset-tonal-surface-500'}"
												onclick={() => {
													showBackendDropdown = false;
													if (mode !== $backendMode) {
														backendMode.setMode(mode);
													}
												}}
											>
												<span class="font-mono font-bold">{BACKEND_SHORT_LABELS[mode]}</span>
												{BACKEND_LABELS[mode]}
											</button>
										{/each}
									</div>
								{/if}
							</div>
						{/if}

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
										class="btn hidden preset-filled-primary-500 btn-sm sm:inline-flex"
										onclick={() => (showUserMenu = !showUserMenu)}
									>
										<User size={16} />
										{$auth.user.firstName}
									</button>

									{#if showUserMenu}
										<div
											class="absolute top-12 right-0 z-10 w-48 card preset-filled-primary-50-950 p-2"
										>
											<div class="space-y-1">
												<div class="px-3 py-2 text-sm">
													<div class="font-medium">
														{$auth.user.firstName}
														{$auth.user.lastName}
													</div>
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
									<a href={loginPath} class="btn preset-filled-primary-500 btn-sm"
										><LogIn /> Login</a
									>
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
					</div>
				</AppBar.Trail>
			</AppBar.Toolbar>
		</AppBar>

		<!-- Mobile Menu Overlay -->
		{#if showMobileMenu}
			<div
				class="mobile-menu bg-surface-50-900-token border-surface-200-700-token bg-opacity-95 dark:bg-opacity-95 fixed inset-x-0 top-0 z-[60] min-h-screen border-b pt-16 shadow-lg backdrop-blur-sm md:hidden"
			>
				<nav class="flex flex-col space-y-4 p-6">
					{#if hasClub}
						<a
							href={clubOpsPath}
							class="btn w-full justify-start preset-filled-success-500"
							onclick={() => (showMobileMenu = false)}
						>
							<Radar size={16} /> Club Ops
						</a>
					{/if}
					<a
						href={livePath}
						class="btn w-full justify-start preset-filled-primary-500"
						onclick={() => (showMobileMenu = false)}
					>
						<Globe size={16} /> Live Map
					</a>
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
						href={arPath}
						class="btn w-full justify-start preset-filled-primary-500"
						onclick={() => (showMobileMenu = false)}
					>
						<Globe size={16} /> AR
					</a>
					<!-- 3D Globe link temporarily disabled
				<a
					href={globePath}
					class="btn w-full justify-start preset-filled-primary-500"
					onclick={() => (showMobileMenu = false)}
				>
					<Globe size={16} /> 3D Globe
				</a>
				-->
					<a
						href={aircraftPath}
						class="btn w-full justify-start preset-filled-primary-500"
						onclick={() => (showMobileMenu = false)}
					>
						<Plane size={16} /> Aircraft
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
						<PlaneTakeoff size={16} /> Flights
					</a>
					<a
						href={infoPath}
						class="btn w-full justify-start preset-filled-primary-500"
						onclick={() => (showMobileMenu = false)}
					>
						<Info size={16} /> System Info
					</a>

					{#if $auth.isAuthenticated}
						<a
							href={watchlistPath}
							class="btn w-full justify-start preset-filled-primary-500"
							onclick={() => (showMobileMenu = false)}
						>
							<Eye size={16} /> Watchlist
						</a>
					{/if}

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

					<!-- Mobile Backend Selector (Dev Only) -->
					{#if dev}
						<div class="space-y-1">
							<div class="px-3 py-1 text-xs text-surface-500">Backend</div>
							{#each ['dev', 'staging', 'prod'] as BackendMode[] as mode (mode)}
								<button
									class="btn w-full justify-start {$backendMode === mode
										? 'preset-filled-primary-500'
										: 'preset-filled-surface-500'}"
									onclick={() => {
										showMobileMenu = false;
										if (mode !== $backendMode) {
											backendMode.setMode(mode);
										}
									}}
								>
									<span class="font-mono font-bold">{BACKEND_SHORT_LABELS[mode]}</span>
									{BACKEND_LABELS[mode]}
								</button>
							{/each}
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

		<main class="container mx-auto flex-1">
			{@render children?.()}
		</main>

		{#if !page.route.id?.includes('operations') && !page.route.id?.includes('/flights/[id]/map') && !page.route.id?.includes('receivers/coverage')}
			<footer class="bg-surface-100-800-token p-4 text-center text-sm">
				<p>&copy; 2025 Liam Bowen</p>
			</footer>
		{/if}
	</div>
{/if}

<Toast.Group {toaster}></Toast.Group>
<BottomLoadingBar />
