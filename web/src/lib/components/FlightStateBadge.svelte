<script lang="ts">
	interface Props {
		state: 'active' | 'complete' | 'timed_out';
	}

	let { state }: Props = $props();

	// Determine badge classes and text based on state
	const stateConfig = $derived.by(() => {
		switch (state) {
			case 'active':
				return {
					classes: 'bg-green-500 text-white',
					text: 'Active'
				};
			case 'complete':
				return {
					classes: 'bg-blue-500 text-white',
					text: 'Complete'
				};
			case 'timed_out':
				return {
					classes: 'bg-orange-500 text-white',
					text: 'Timed Out'
				};
			default:
				return {
					classes: 'bg-gray-500 text-white',
					text: 'Unknown'
				};
		}
	});
</script>

<span class="badge {stateConfig.classes} text-xs">{stateConfig.text}</span>
