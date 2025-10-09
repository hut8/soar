<script lang="ts">
	import { Combobox } from '@skeletonlabs/skeleton-svelte';
	import { onMount } from 'svelte';
	import { serverCall } from '$lib/api/server';
	import type { ClubWithSoaring, ComboboxData } from '$lib/types';

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
	let clubs: ClubWithSoaring[] = [];
	let comboboxData: ComboboxData[] = [];
	let error = '';

	// Convert clubs to combobox data format
	function transformClubsToComboboxData(clubList: ClubWithSoaring[]): ComboboxData[] {
		return clubList.map((club) => ({
			label: club.name,
			value: club.id,
			club: club
		}));
	}

	// Load clubs from API
	async function loadClubs(query?: string) {
		try {
			error = '';

			let endpoint = '/clubs?limit=100';
			if (query && query.trim()) {
				endpoint += `&q=${encodeURIComponent(query.trim())}`;
			}

			clubs = await serverCall<ClubWithSoaring[]>(endpoint);
			comboboxData = transformClubsToComboboxData(clubs);
		} catch (err) {
			console.error('Failed to load clubs:', err);
			error = err instanceof Error ? err.message : 'Failed to load clubs';
			clubs = [];
			comboboxData = [];
		} finally {
			// Loading complete
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
		<div class="preset-filled-error mb-2 rounded-lg p-2 text-sm">
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
			<div class="flex w-full items-center space-x-2">
				<span class="flex-1 text-left">{item.label}</span>
				{#if item.club.is_soaring}
					<span class="rounded-full bg-primary-500 px-2 py-1 text-xs text-white">Soaring</span>
				{/if}
			</div>
		{/snippet}
	</Combobox>
</div>

<style>
	.club-selector {
		width: 100%;
	}
</style>
