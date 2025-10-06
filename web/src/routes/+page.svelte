<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { Users, Radar, Radio, Antenna, MapPin } from '@lucide/svelte';

	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');
	const devicesPath = resolve('/devices');
	const receiversPath = resolve('/receivers');
	const airportsPath = resolve('/airports');

	// Watch for auth changes and redirect when user becomes authenticated
	$: if ($auth.isAuthenticated && $auth.user) {
		goto(clubsPath);
	}
</script>

<svelte:head>
	<title>Glider Flights - Soaring Club Directory</title>
	<meta name="description" content="Discover soaring clubs and track glider operations near you" />
</svelte:head>

<!-- Background Video -->
<video class="fixed top-0 left-0 z-[-1] h-full w-full object-cover" autoplay muted loop playsinline>
	<source src="/glider.mp4" type="video/mp4" />
</video>

<!-- Video Overlay for Text Legibility -->
<div class="fixed top-0 left-0 z-[-1] h-full w-full bg-black/40"></div>

<div class="relative z-10 flex min-h-screen flex-col justify-center space-y-12">
	<!-- Hero Section -->
	<section class="space-y-8 text-center">
		<!-- Hero Content -->
		<div class="space-y-6">
			<h1 class="text-4xl font-bold text-white drop-shadow-lg md:text-6xl">
				Welcome to Glider.flights
			</h1>
			<p class="mx-auto max-w-3xl text-xl text-white/90 drop-shadow-lg md:text-2xl">
				Discover soaring clubs, track glider operations, and connect with the soaring community
			</p>
		</div>
	</section>

	<!-- Main Navigation -->
	<section class="mx-auto max-w-6xl">
		<div class="grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-3">
			<!-- Clubs Button -->
			<a
				href={clubsPath}
				class="group block rounded-lg border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:scale-105 hover:bg-white/30 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-primary-500/20 p-4 transition-colors group-hover:bg-primary-500/30"
						>
							<Users size={48} class="text-white drop-shadow-lg" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold text-white drop-shadow-lg">Browse Clubs</h2>
					</div>
				</div>
			</a>

			<!-- Operations Button -->
			<a
				href={operationsPath}
				class="group block rounded-lg border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:scale-105 hover:bg-white/30 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-secondary-500/20 p-4 transition-colors group-hover:bg-secondary-500/30"
						>
							<Radar size={48} class="text-white drop-shadow-lg" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold text-white drop-shadow-lg">Live Operations</h2>
					</div>
				</div>
			</a>

			<!-- Aircraft Button -->
			<a
				href={devicesPath}
				class="group block rounded-lg border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:scale-105 hover:bg-white/30 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-tertiary-500/20 p-4 transition-colors group-hover:bg-tertiary-500/30"
						>
							<Radio size={48} class="text-white drop-shadow-lg" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold text-white drop-shadow-lg">Aircraft</h2>
					</div>
				</div>
			</a>

			<!-- Receivers Button -->
			<a
				href={receiversPath}
				class="group block rounded-lg border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:scale-105 hover:bg-white/30 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-success-500/20 p-4 transition-colors group-hover:bg-success-500/30"
						>
							<Antenna size={48} class="text-white drop-shadow-lg" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold text-white drop-shadow-lg">Receivers</h2>
					</div>
				</div>
			</a>

			<!-- Airports Button -->
			<a
				href={airportsPath}
				class="group block rounded-lg border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:scale-105 hover:bg-white/30 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-warning-500/20 p-4 transition-colors group-hover:bg-warning-500/30"
						>
							<MapPin size={48} class="text-white drop-shadow-lg" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold text-white drop-shadow-lg">Airports</h2>
					</div>
				</div>
			</a>
		</div>
	</section>
</div>
