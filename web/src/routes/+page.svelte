<script lang="ts">
	import { resolve } from '$app/paths';
	import { Users, Radar, Radio, Antenna, MapPin, Plane } from '@lucide/svelte';
	import { auth } from '$lib/stores/auth';

	const clubsPath = resolve('/clubs');
	const operationsPath = resolve('/operations');
	const aircraftPath = resolve('/aircraft');
	const receiversPath = resolve('/receivers');
	const airportsPath = resolve('/airports');
	const flightsPath = resolve('/flights');

	// Reactive club operations path
	let clubOpsPath = $derived(
		$auth.user?.clubId ? resolve(`/clubs/${$auth.user.clubId}/operations`) : ''
	);
	let hasClub = $derived($auth.isAuthenticated && !!$auth.user?.clubId);
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

<div
	class="relative z-10 flex min-h-screen flex-col md:fixed md:inset-0 md:top-16 md:overflow-hidden"
>
	<!-- Main Navigation -->
	<section class="flex flex-1 items-center justify-center p-12 md:h-full">
		<div class="flex h-full w-full max-w-7xl flex-col items-center justify-center gap-12">
			<!-- Club Ops Button (full width, only for logged-in users with a club) -->
			{#if hasClub}
				<a
					href={clubOpsPath}
					class="group flex w-full max-w-5xl items-center justify-center border border-white/30 bg-white/20 p-6 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl md:p-8"
				>
					<div class="flex items-center gap-6">
						<div
							class="rounded-full bg-primary-500/20 p-4 transition-colors group-hover:bg-primary-500/30"
						>
							<Radar size={48} class="text-white drop-shadow-lg" />
						</div>
						<h2 class="text-3xl font-bold text-white drop-shadow-lg md:text-4xl">
							My Club Operations
						</h2>
					</div>
				</a>
			{/if}

			<!-- Standard Navigation Grid -->
			<div
				class="grid w-full grid-cols-1 content-evenly justify-items-center gap-y-24 md:grid-cols-3 md:gap-x-12 md:gap-y-32"
			>
				<!-- Clubs Button -->
				<a
					href={clubsPath}
					class="group flex w-64 items-center justify-center border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl"
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
							<h2 class="text-2xl font-bold text-white drop-shadow-lg">Clubs</h2>
						</div>
					</div>
				</a>

				<!-- Operations Button -->
				<a
					href={operationsPath}
					class="group flex w-64 items-center justify-center border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl"
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
							<h2 class="text-2xl font-bold text-white drop-shadow-lg">Operations</h2>
						</div>
					</div>
				</a>

				<!-- Aircraft Button -->
				<a
					href={aircraftPath}
					class="group flex w-64 items-center justify-center border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl"
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
					class="group flex w-64 items-center justify-center border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl"
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
					class="group flex w-64 items-center justify-center border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl"
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

				<!-- Flights Button -->
				<a
					href={flightsPath}
					class="group flex w-64 items-center justify-center border border-white/30 bg-white/20 p-8 backdrop-blur-md transition-all duration-200 hover:bg-white/30 hover:shadow-xl"
				>
					<div class="space-y-6 text-center">
						<div class="flex justify-center">
							<div
								class="rounded-full bg-primary-500/20 p-4 transition-colors group-hover:bg-primary-500/30"
							>
								<Plane size={48} class="text-white drop-shadow-lg" />
							</div>
						</div>
						<div class="space-y-2">
							<h2 class="text-2xl font-bold text-white drop-shadow-lg">Flights</h2>
						</div>
					</div>
				</a>
			</div>
		</div>
	</section>
</div>
