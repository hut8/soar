<script lang="ts">
	import { Combobox } from '@skeletonlabs/skeleton-svelte';
	import { onMount } from 'svelte';
	import { serverCall } from '$lib/api/server';

	interface Club {
		id: string;
		name: string;
		is_soaring?: boolean;
	}

	interface ComboboxData {
		label: string;
		value: string;
		club: Club;
	}

	// Props
	export let value: string[] = [];
	export let placeholder: string = 'Select a club...';
	export let label: string = 'Club';
	export let disabled: boolean = false;
	export let required: boolean = false;
	export let searchQuery: string = '';
	export let onValueChange: ((e: { value: string[] }) => void) | undefined = undefined;
	export let onInputValueChange: ((e: { inputValue: string }) => void) | undefined = undefined;

	// Internal state
	let clubs: Club[] = [];
	let comboboxData: ComboboxData[] = [];
	let loading = true;
	let error = '';

	// Convert clubs to combobox data format
	function transformClubsToComboboxData(clubList: Club[]): ComboboxData[] {
		return clubList.map((club) => ({
			label: club.name,
			value: club.id,
			club: club
		}));
	}

	// Load clubs from API
	async function loadClubs(query?: string) {
		try {
			loading = true;
			error = '';

			let endpoint = '/clubs?limit=100';
			if (query && query.trim()) {
				endpoint += `&q=${encodeURIComponent(query.trim())}`;
			}

			clubs = await serverCall<Club[]>(endpoint);
			comboboxData = transformClubsToComboboxData(clubs);
		} catch (err) {
			console.error('Failed to load clubs:', err);
			error = err instanceof Error ? err.message : 'Failed to load clubs';
			clubs = [];
			comboboxData = [];
		} finally {
			loading = false;
		}
	}

	// Custom search function
	function handleInputValueChange(e: { inputValue: string }) {
		searchQuery = e.inputValue;

		// Call external handler if provided
		if (onInputValueChange) {
			onInputValueChange(e);
		}

		// Debounce the search
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			loadClubs(searchQuery);
		}, 300);
	}

	// Handle value changes
	function handleValueChange(e: { value: string[] }) {
		value = e.value;

		// Call external handler if provided
		if (onValueChange) {
			onValueChange(e);
		}
	}

	// Search timeout for debouncing
	let searchTimeout: ReturnType<typeof setTimeout>;

	// Load initial clubs on mount
	onMount(() => {
		loadClubs();
	});

	// Cleanup timeout on destroy
	import { onDestroy } from 'svelte';
	onDestroy(() => {
		if (searchTimeout) {
			clearTimeout(searchTimeout);
		}
	});
</script>

<div class="club-selector">
	{#if error}
		<div class="variant-filled-error mb-2 rounded-lg p-2 text-sm">
			{error}
		</div>
	{/if}

	<Combobox
		data={comboboxData}
		{value}
		onValueChange={handleValueChange}
		onInputValueChange={handleInputValueChange}
		{label}
		{placeholder}
		{disabled}
		{required}
	>
		{#snippet item(item)}
			<div class="flex w-full justify-between items-center space-x-2">
				<span class="flex-1">{item.label}</span>
				{#if item.club.is_soaring}
					<span class="text-xs bg-primary-500 text-white px-2 py-1 rounded-full">Soaring</span>
				{/if}
			</div>
		{/snippet}
	</Combobox>

	{#if loading}
		<div class="text-surface-600-300-token text-xs mt-1">Loading clubs...</div>
	{/if}
</div>

<style>
	.club-selector {
		width: 100%;
	}
</style>
