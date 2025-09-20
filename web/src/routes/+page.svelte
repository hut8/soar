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

<div class="space-y-12">
	<!-- Hero Section -->
	<section class="space-y-8 text-center">
		<!-- Hero Image Placeholder -->
		<div class="mx-auto max-w-4xl">
			<div
				class="border-surface-300-600-token flex aspect-video items-center justify-center rounded-2xl border-2 border-dashed bg-gradient-to-br from-primary-500/20 to-secondary-500/20"
			>
				<div class="space-y-4 text-center">
					<div class="text-6xl">🪂</div>
					<div class="text-surface-600-300-token text-lg font-medium">
						Glider Operations & Club Directory
					</div>
				</div>
			</div>
		</div>

		<!-- Hero Content -->
		<div class="space-y-6">
			<h1
				class="bg-gradient-to-br from-primary-500 to-secondary-500 bg-clip-text text-4xl font-bold text-transparent md:text-6xl"
			>
				Welcome to Glider.flights
			</h1>
			<p class="text-surface-600-300-token mx-auto max-w-3xl text-xl md:text-2xl">
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
				class="group card p-8 transition-transform duration-200 hover:scale-105 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-primary-500/10 p-4 transition-colors group-hover:bg-primary-500/20"
						>
							<Users size={48} class="text-primary-500" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold">Browse Clubs</h2>
						<p class="text-surface-600-300-token">
							Find soaring clubs and airports near you. Discover new places to fly and connect with
							local communities.
						</p>
					</div>
					<div
						class="variant-filled-primary group-hover:variant-filled-secondary btn w-full btn-lg transition-colors"
					>
						Explore Clubs
					</div>
				</div>
			</a>

			<!-- Operations Button -->
			<a
				href={operationsPath}
				class="group card p-8 transition-transform duration-200 hover:scale-105 hover:shadow-xl"
			>
				<div class="space-y-6 text-center">
					<div class="flex justify-center">
						<div
							class="rounded-full bg-secondary-500/10 p-4 transition-colors group-hover:bg-secondary-500/20"
						>
							<Radar size={48} class="text-secondary-500" />
						</div>
					</div>
					<div class="space-y-2">
						<h2 class="text-2xl font-bold">Live Operations</h2>
						<p class="text-surface-600-300-token">
							Track real-time glider operations, flight activity, and see what's happening in the
							sky right now.
						</p>
					</div>
					<div
						class="variant-filled-secondary group-hover:variant-filled-primary btn w-full btn-lg transition-colors"
					>
						View Operations
					</div>
				</div>
			</a>
		</div>
	</section>

	<!-- Features Overview -->
	<section class="mx-auto max-w-6xl">
		<div class="space-y-8 text-center">
			<h2 class="text-3xl font-bold">Everything You Need for Soaring</h2>
			<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
				<div class="space-y-4 card p-6 text-center">
					<div class="text-4xl">🗺️</div>
					<h3 class="text-xl font-semibold">Club Directory</h3>
					<p class="text-surface-600-300-token">
						Comprehensive database of soaring clubs and airports worldwide
					</p>
				</div>
				<div class="space-y-4 card p-6 text-center">
					<div class="text-4xl">📡</div>
					<h3 class="text-xl font-semibold">Live Tracking</h3>
					<p class="text-surface-600-300-token">
						Real-time flight tracking and operational status updates
					</p>
				</div>
				<div class="space-y-4 card p-6 text-center">
					<div class="text-4xl">🤝</div>
					<h3 class="text-xl font-semibold">Community</h3>
					<p class="text-surface-600-300-token">
						Connect with pilots and clubs in the soaring community
					</p>
				</div>
			</div>
		</div>
	</section>
</div>
