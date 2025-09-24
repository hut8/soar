<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import { Users, Radar } from '@lucide/svelte';

	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');

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
<video
	class="fixed top-0 left-0 w-full h-full object-cover z-[-1]"
	autoplay
	muted
	loop
	playsinline
>
	<source src="/glider.mp4" type="video/mp4">
</video>

<!-- Video Overlay for Text Legibility -->
<div class="fixed top-0 left-0 w-full h-full bg-black/40 z-[-1]"></div>

<div class="relative z-10 min-h-screen flex flex-col justify-center space-y-12">
	<!-- Hero Section -->
	<section class="space-y-8 text-center">
		<!-- Hero Content -->
		<div class="space-y-6">
			<h1
				class="text-white text-4xl font-bold md:text-6xl drop-shadow-lg"
			>
				Welcome to Glider.flights
			</h1>
			<p class="text-white/90 mx-auto max-w-3xl text-xl md:text-2xl drop-shadow-lg">
				Discover soaring clubs, track glider operations, and connect with the soaring community
			</p>
		</div>
	</section>

	<!-- Main Navigation -->
	<section class="mx-auto max-w-4xl">
		<div class="grid grid-cols-1 gap-8 md:grid-cols-2">
			<!-- Clubs Button -->
			<a
				href={clubsPath}
				class="group block p-8 transition-all duration-200 hover:scale-105 hover:shadow-xl bg-white/20 backdrop-blur-md border border-white/30 rounded-lg hover:bg-white/30"
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
				class="group block p-8 transition-all duration-200 hover:scale-105 hover:shadow-xl bg-white/20 backdrop-blur-md border border-white/30 rounded-lg hover:bg-white/30"
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
		</div>
	</section>
</div>
