<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { 
		ArrowLeft, 
		Building, 
		MapPin, 
		Plane, 
		Mail, 
		Phone, 
		Globe, 
		Users, 
		Calendar,
		Navigation,
		Info
	} from '@lucide/svelte';
	import { ProgressRing } from '@skeletonlabs/skeleton-svelte';
	import { serverCall } from '$lib/api/server';

	interface Point {
		latitude: number;
		longitude: number;
	}

	interface Club {
		id: string;
		name: string;
		is_soaring?: boolean;
		home_base_airport_id?: number;
		location_id?: string;
		street1?: string;
		street2?: string;
		city?: string;
		state?: string;
		zip_code?: string;
		region_code?: string;
		county_mail_code?: string;
		country_mail_code?: string;
		base_location?: Point;
		created_at: string;
		updated_at: string;
	}

	let club: Club | null = null;
	let loading = true;
	let error = '';
	let clubId = '';

	$: clubId = $page.params.id || '';

	onMount(async () => {
		if (clubId) {
			await loadClub();
		}
	});

	async function loadClub() {
		loading = true;
		error = '';

		try {
			club = await serverCall<Club>(`/clubs/${clubId}`);
		} catch (err) {
			const errorMessage = err instanceof Error ? err.message : 'Unknown error';
			error = `Failed to load club: ${errorMessage}`;
			console.error('Error loading club:', err);
		} finally {
			loading = false;
		}
	}

	function formatAddress(club: Club): string {
		const parts = [];
		if (club.street1) parts.push(club.street1);
		if (club.street2) parts.push(club.street2);
		if (club.city) parts.push(club.city);
		if (club.state) parts.push(club.state);
		if (club.zip_code) parts.push(club.zip_code);
		return parts.join(', ') || 'Address not available';
	}

	function formatCoordinates(point: Point): string {
		return `${point.latitude.toFixed(6)}, ${point.longitude.toFixed(6)}`;
	}

	function formatDate(dateString: string): string {
		return new Date(dateString).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'long',
			day: 'numeric'
		});
	}

	function goBack() {
		goto('/clubs');
	}
</script>

<svelte:head>
	<title>{club?.name || 'Club Details'} - Soaring Clubs</title>
</svelte:head>

<div class="container mx-auto p-4 space-y-6 max-w-4xl">
	<!-- Back Button -->
	<div class="flex items-center gap-4">
		<button class="btn btn-sm variant-soft" on:click={goBack}>
			<ArrowLeft class="w-4 h-4 mr-2" />
			Back to Clubs
		</button>
	</div>

	<!-- Loading State -->
	{#if loading}
		<div class="card p-8">
			<div class="flex items-center justify-center space-x-4">
				<ProgressRing size="w-8 h-8" />
				<span class="text-lg">Loading club details...</span>
			</div>
		</div>
	{/if}

	<!-- Error State -->
	{#if error}
		<div class="alert variant-filled-error">
			<div class="alert-message">
				<h3 class="h3">Error Loading Club</h3>
				<p>{error}</p>
				<div class="alert-actions">
					<button class="btn variant-filled" on:click={loadClub}>
						Try Again
					</button>
				</div>
			</div>
		</div>
	{/if}

	<!-- Club Details -->
	{#if !loading && !error && club}
		<div class="space-y-6">
			<!-- Header Card -->
			<div class="card p-6">
				<div class="flex items-start justify-between flex-wrap gap-4">
					<div class="flex-1">
						<div class="flex items-center gap-3 mb-2">
							<Building class="w-8 h-8 text-primary-500" />
							<h1 class="h1">{club.name}</h1>
						</div>
						{#if club.is_soaring}
							<div class="inline-flex items-center gap-2 bg-primary-500 text-white px-3 py-1 rounded-full text-sm">
								<Plane class="w-4 h-4" />
								Soaring Club
							</div>
						{/if}
					</div>
				</div>
			</div>

			<!-- Main Content Grid -->
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
				<!-- Location Information -->
				<div class="card p-6 space-y-4">
					<h2 class="h2 flex items-center gap-2">
						<MapPin class="w-6 h-6" />
						Location
					</h2>
					
					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Info class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Address</p>
								<p>{formatAddress(club)}</p>
							</div>
						</div>

						{#if club.base_location}
							<div class="flex items-start gap-3">
								<Navigation class="w-4 h-4 mt-1 text-surface-500" />
								<div>
									<p class="text-sm text-surface-600-300-token mb-1">Coordinates</p>
									<p class="font-mono text-sm">{formatCoordinates(club.base_location)}</p>
								</div>
							</div>
						{/if}

						{#if club.home_base_airport_id}
							<div class="flex items-start gap-3">
								<Plane class="w-4 h-4 mt-1 text-surface-500" />
								<div>
									<p class="text-sm text-surface-600-300-token mb-1">Home Base Airport</p>
									<p>Airport ID: {club.home_base_airport_id}</p>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Additional Information -->
				<div class="card p-6 space-y-4">
					<h2 class="h2 flex items-center gap-2">
						<Info class="w-6 h-6" />
						Details
					</h2>
					
					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Users class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Club ID</p>
								<p class="font-mono text-sm">{club.id}</p>
							</div>
						</div>

						{#if club.location_id}
							<div class="flex items-start gap-3">
								<MapPin class="w-4 h-4 mt-1 text-surface-500" />
								<div>
									<p class="text-sm text-surface-600-300-token mb-1">Location ID</p>
									<p class="font-mono text-sm">{club.location_id}</p>
								</div>
							</div>
						{/if}

						{#if club.region_code}
							<div class="flex items-start gap-3">
								<Globe class="w-4 h-4 mt-1 text-surface-500" />
								<div>
									<p class="text-sm text-surface-600-300-token mb-1">Region</p>
									<p>{club.region_code}</p>
								</div>
							</div>
						{/if}
					</div>
				</div>

				<!-- Administrative Information -->
				{#if club.county_mail_code || club.country_mail_code}
					<div class="card p-6 space-y-4">
						<h2 class="h2 flex items-center gap-2">
							<Mail class="w-6 h-6" />
							Administrative
						</h2>
						
						<div class="space-y-3">
							{#if club.county_mail_code}
								<div class="flex items-start gap-3">
									<Info class="w-4 h-4 mt-1 text-surface-500" />
									<div>
										<p class="text-sm text-surface-600-300-token mb-1">County Mail Code</p>
										<p>{club.county_mail_code}</p>
									</div>
								</div>
							{/if}

							{#if club.country_mail_code}
								<div class="flex items-start gap-3">
									<Globe class="w-4 h-4 mt-1 text-surface-500" />
									<div>
										<p class="text-sm text-surface-600-300-token mb-1">Country Mail Code</p>
										<p>{club.country_mail_code}</p>
									</div>
								</div>
							{/if}
						</div>
					</div>
				{/if}

				<!-- Timestamps -->
				<div class="card p-6 space-y-4">
					<h2 class="h2 flex items-center gap-2">
						<Calendar class="w-6 h-6" />
						Record Information
					</h2>
					
					<div class="space-y-3">
						<div class="flex items-start gap-3">
							<Calendar class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Created</p>
								<p>{formatDate(club.created_at)}</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<Calendar class="w-4 h-4 mt-1 text-surface-500" />
							<div>
								<p class="text-sm text-surface-600-300-token mb-1">Last Updated</p>
								<p>{formatDate(club.updated_at)}</p>
							</div>
						</div>
					</div>
				</div>
			</div>

			<!-- Map Section (placeholder for future implementation) -->
			{#if club.base_location}
				<div class="card p-6">
					<h2 class="h2 flex items-center gap-2 mb-4">
						<Navigation class="w-6 h-6" />
						Location Map
					</h2>
					<div class="bg-surface-100-800-token rounded-lg p-8 text-center">
						<MapPin class="w-12 h-12 mx-auto text-surface-500 mb-4" />
						<p class="text-surface-600-300-token">
							Map integration coming soon
						</p>
						<p class="text-sm text-surface-500 mt-2">
							{formatCoordinates(club.base_location)}
						</p>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>