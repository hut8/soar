<script lang="ts">
	import '../../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Operations - Glider Flights</title>
	<meta name="description" content="Real-time operations center for glider flights" />
</svelte:head>

<!-- Full screen layout without navigation for operations -->
{@render children?.()}
