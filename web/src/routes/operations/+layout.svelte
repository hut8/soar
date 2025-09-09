<script lang="ts">
	import '../../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Operations - Glider Flights</title>
	<meta name="description" content="Real-time operations center for glider flights" />
</svelte:head>

<!-- Full screen layout without navigation for operations -->
<div class="h-screen w-full overflow-hidden">
	{@render children?.()}
</div>

<style>
	/* Ensure full screen coverage for operations */
	:global(html, body) {
		margin: 0;
		padding: 0;
		height: 100vh;
		width: 100vw;
		overflow: hidden;
	}
</style>
