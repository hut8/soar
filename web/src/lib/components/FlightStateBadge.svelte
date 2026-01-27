<script lang="ts">
	import type { FlightState } from '$lib/types';

	interface Props {
		state: FlightState;
	}

	let { state }: Props = $props();

	// Determine badge classes and text based on state
	const stateConfig = $derived.by(() => {
		switch (state) {
			case 'active':
				return {
					classes: 'bg-green-500 text-white',
					text: 'Status: Active'
				};
			case 'stale':
				return {
					classes: 'bg-yellow-500 text-white',
					text: 'Status: Stale'
				};
			case 'complete':
				return {
					classes: 'bg-blue-500 text-white',
					text: 'Status: Complete'
				};
			case 'timed_out':
				return {
					classes: 'bg-orange-500 text-white',
					text: 'Status: Timed Out'
				};
			default:
				return {
					classes: 'bg-gray-500 text-white',
					text: 'Status: Unknown'
				};
		}
	});
</script>

<span class="badge {stateConfig.classes} text-xs">{stateConfig.text}</span>
