<script lang="ts">
	import '../../../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Flight Map - Glider Flights</title>
	<meta name="description" content="Full-screen flight track map" />
</svelte:head>

<!-- Full screen layout without navigation for flight map -->
{@render children?.()}
