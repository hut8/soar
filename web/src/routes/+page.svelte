<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';

	const clubsPath = resolve('/clubs');

	// Watch for auth changes and redirect when user becomes authenticated
	$: if ($auth.isAuthenticated && $auth.user) {
		goto(clubsPath);
	}
</script>

<svelte:head>
	<title></title>
</svelte:head>

<div class="space-y-8">
	<!-- Features Section -->
	<section class="grid grid-cols-1 gap-6 md:grid-cols-2">
		<div class="space-y-4 card p-6">
			<a href={clubsPath} class="variant-filled-primary btn">Browse Clubs</a>
		</div>
	</section>
</div>
