<script lang="ts">
	import '../app.css';
	import { AppShell, AppBar, Avatar } from '@skeletonlabs/skeleton';
	import favicon from '$lib/assets/favicon.svg';
	import { resolve } from '$app/paths';
	import { auth } from '$lib/stores/auth';
	import { onMount } from 'svelte';

	const base = resolve('/');
	const clubsPath = resolve('/clubs');
	const airportsPath = resolve('/airports');
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

<AppShell>
	<svelte:fragment slot="header">
		<AppBar>
			<svelte:fragment slot="lead">
				<a href={base} class="flex items-center space-x-2">
					<div class="text-xl font-bold text-primary-500">✈️ Glider.flights</div>
				</a>
			</svelte:fragment>
			<svelte:fragment slot="trail">
				<nav class="hidden space-x-4 md:flex">
					<a href={clubsPath} class="variant-ghost-surface btn btn-sm">Clubs</a>
					<a href={airportsPath} class="variant-ghost-surface btn btn-sm">Airports</a>
					<a href={operationsPath} class="variant-ghost-surface btn btn-sm">🗺️ Operations</a>
				</nav>

				{#if $auth.isAuthenticated && $auth.user}
					<div class="user-menu relative">
						<button
							class="variant-ghost-surface btn btn-sm flex items-center space-x-2"
							onclick={() => (showUserMenu = !showUserMenu)}
						>
							<Avatar
								initials={getInitials($auth.user.first_name, $auth.user.last_name)}
								width="w-8 h-8"
								background="bg-primary-500"
							/>
							<span class="hidden sm:inline">{$auth.user.first_name}</span>
						</button>

						{#if showUserMenu}
							<div class="card absolute right-0 top-12 z-10 w-48 p-2">
								<div class="space-y-1">
									<div class="px-3 py-2 text-sm">
										<div class="font-medium">{$auth.user.first_name} {$auth.user.last_name}</div>
										<div class="text-surface-600-300-token">{$auth.user.email}</div>
									</div>
									<hr class="!my-2" />
									<a
										href={profilePath}
										class="variant-ghost-surface btn btn-sm w-full justify-start"
									>
										👤 Profile
									</a>
									<button
										class="variant-ghost-error btn btn-sm w-full justify-start"
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
			</svelte:fragment>
		</AppBar>
	</svelte:fragment>

	<main class="container mx-auto space-y-4 p-4">
		{@render children?.()}
	</main>

	<svelte:fragment slot="pageFooter">
		<footer class="bg-surface-100-800-token p-4 text-center text-sm">
			<p>&copy; 2025 Liam Bowen</p>
		</footer>
	</svelte:fragment>
</AppShell>
