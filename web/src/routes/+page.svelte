<script lang="ts">
	import { resolve } from '$app/paths';

	const clubsPath = resolve('/clubs');
</script>

<svelte:head>
	<title></title>
</svelte:head>

<div class="space-y-8">
	<!-- Features Section -->
	<section class="grid grid-cols-1 gap-6 md:grid-cols-2">
		<div class="card space-y-4 p-6">
			<a href={clubsPath} class="variant-filled-primary btn">Browse Clubs</a>
		</div>
	</section>
</div>
