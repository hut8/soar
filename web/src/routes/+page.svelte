<script lang="ts">
	import { onMount } from 'svelte';

	let searchQuery = '';
	let searchType = 'clubs';
	let locationSearch = false;
	let latitude = '';
	let longitude = '';
	let radius = '50';

	function handleSearch() {
		if (locationSearch) {
			window.location.href = `/${searchType}?latitude=${latitude}&longitude=${longitude}&radius=${radius}`;
		} else {
			window.location.href = `/${searchType}?q=${encodeURIComponent(searchQuery)}`;
		}
	}

	function getCurrentLocation() {
		if (navigator.geolocation) {
			navigator.geolocation.getCurrentPosition(
				(position) => {
					latitude = position.coords.latitude.toString();
					longitude = position.coords.longitude.toString();
				},
				(error) => {
					console.error('Error getting location:', error);
				}
			);
		}
	}
</script>

<svelte:head>
	<title>Glider Flights - Find Soaring Clubs and Airports</title>
</svelte:head>

<div class="space-y-8">
	<!-- Hero Section -->
	<section class="space-y-4 text-center">
		<h1 class="gradient-heading h1">Welcome to Glider Flights</h1>
		<p class="text-surface-600-300-token text-xl">
			Discover soaring clubs and airports near you. Connect with the gliding community worldwide.
		</p>
	</section>

	<!-- Search Section -->
	<section class="card space-y-4 p-6">
		<h2 class="h2">Search</h2>

		<!-- Search Type Toggle -->
		<div class="flex justify-center space-x-2">
			<button
				class="btn {searchType === 'clubs' ? 'variant-filled-primary' : 'variant-ghost-surface'}"
				on:click={() => (searchType = 'clubs')}
			>
				Soaring Clubs
			</button>
			<button
				class="btn {searchType === 'airports' ? 'variant-filled-primary' : 'variant-ghost-surface'}"
				on:click={() => (searchType = 'airports')}
			>
				Airports
			</button>
		</div>

		<!-- Search Method Toggle -->
		<div class="flex justify-center space-x-2">
			<button
				class="btn btn-sm {!locationSearch ? 'variant-filled-secondary' : 'variant-ghost-surface'}"
				on:click={() => (locationSearch = false)}
			>
				🔍 Name Search
			</button>
			<button
				class="btn btn-sm {locationSearch ? 'variant-filled-secondary' : 'variant-ghost-surface'}"
				on:click={() => (locationSearch = true)}
			>
				📍 Location Search
			</button>
		</div>

		<!-- Search Forms -->
		{#if !locationSearch}
			<div class="space-y-4">
				<input
					bind:value={searchQuery}
					class="input"
					type="text"
					placeholder="Search for {searchType} by name..."
					on:keydown={(e) => e.key === 'Enter' && handleSearch()}
				/>
				<div class="flex justify-center">
					<button class="variant-filled-primary btn" on:click={handleSearch}>
						Search {searchType}
					</button>
				</div>
			</div>
		{:else}
			<div class="space-y-4">
				<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
					<input
						bind:value={latitude}
						class="input"
						type="number"
						step="any"
						placeholder="Latitude"
					/>
					<input
						bind:value={longitude}
						class="input"
						type="number"
						step="any"
						placeholder="Longitude"
					/>
					<input
						bind:value={radius}
						class="input"
						type="number"
						min="1"
						max="1000"
						placeholder="Radius (km)"
					/>
				</div>
				<div class="flex justify-center space-x-2">
					<button class="variant-ghost-surface btn" on:click={getCurrentLocation}>
						📱 Use My Location
					</button>
					<button class="variant-filled-primary btn" on:click={handleSearch}>
						Search Nearby {searchType}
					</button>
				</div>
			</div>
		{/if}
	</section>

	<!-- Features Section -->
	<section class="grid grid-cols-1 gap-6 md:grid-cols-2">
		<div class="card space-y-4 p-6">
			<h3 class="h3">🏛️ Find Soaring Clubs</h3>
			<p>
				Discover active soaring clubs in your area. Connect with local gliding communities, find
				instruction opportunities, and join fellow aviation enthusiasts.
			</p>
			<a href="/clubs" class="variant-filled-primary btn">Browse Clubs</a>
		</div>

		<div class="card space-y-4 p-6">
			<h3 class="h3">✈️ Locate Airports</h3>
			<p>
				Search for airports and airfields suitable for gliding operations. Find runway information,
				contact details, and facilities available for soaring activities.
			</p>
			<a href="/airports" class="variant-filled-primary btn">Browse Airports</a>
		</div>
	</section>

	<!-- About Section -->
	<section class="card space-y-4 p-6">
		<h3 class="h3">About Glider Flights</h3>
		<p class="text-surface-600-300-token">
			Glider Flights is your comprehensive directory for the soaring community. We help pilots,
			students, and enthusiasts discover clubs, airports, and connect with the worldwide gliding
			network. Whether you're looking for instruction, club membership, or just exploring the world
			of soaring, we're here to help you take flight.
		</p>
	</section>
</div>

<style>
	.gradient-heading {
		background: linear-gradient(
			45deg,
			rgb(var(--color-primary-500)),
			rgb(var(--color-secondary-500))
		);
		-webkit-background-clip: text;
		-webkit-text-fill-color: transparent;
		background-clip: text;
	}
</style>
