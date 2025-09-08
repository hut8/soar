<script lang="ts">
	import '../app.css';
	import { AppShell, AppBar } from '@skeletonlabs/skeleton';
	import favicon from '$lib/assets/favicon.svg';
	import { resolve } from '$app/paths';

	const base = resolve('/');
	const clubsPath = resolve('/clubs');
	const airportsPath = resolve('/airports');

	let { children } = $props();
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
				</nav>
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
