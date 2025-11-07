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
	export let onValueChange: ((details: { value: string[] }) => void) | undefined = undefined;
	export let onInputValueChange: ((details: { inputValue: string }) => void) | undefined =
		undefined;

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
	function handleInputValueChange(details: { inputValue: string }) {
		searchQuery = details.inputValue;

		// Call external handler if provided
		if (onInputValueChange) {
			onInputValueChange(details);
		}

		// Debounce the search
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			loadClubs(searchQuery);
		}, 300);
	}

	// Handle value changes
	function handleValueChange(details: { value: string[] }) {
		value = details.value;

		// Call external handler if provided
		if (onValueChange) {
			onValueChange(details);
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
		<div class="mb-2 rounded-lg preset-filled-error-500 p-2 text-sm">
			{error}
		</div>
	{/if}

	<Combobox
		{value}
		onValueChange={handleValueChange}
		onInputValueChange={handleInputValueChange}
		{disabled}
	>
		<Combobox.Label>{label}</Combobox.Label>
		<Combobox.Control>
			<Combobox.Input {placeholder} {required} />
		</Combobox.Control>
		<Combobox.Positioner>
			<Combobox.Content>
				{#each comboboxData as clubItem (clubItem.value)}
					<Combobox.Item item={{ label: clubItem.label, value: clubItem.value }}>
						<div class="flex w-full items-center space-x-2">
							<span class="flex-1 text-left">{clubItem.label}</span>
							{#if clubItem.club.is_soaring}
								<span class="rounded-full bg-primary-500 px-2 py-1 text-xs text-white">Soaring</span
								>
							{/if}
						</div>
					</Combobox.Item>
				{/each}
			</Combobox.Content>
		</Combobox.Positioner>
	</Combobox>
</div>

<style>
	.club-selector {
		width: 100%;
	}

	/* Fix dark mode colors for combobox dropdown */
	:global(.club-selector [data-popover-content]) {
		background-color: var(--color-surface-50);
		color: var(--color-surface-900);
	}

	:global(.dark .club-selector [data-popover-content]) {
		background-color: var(--color-surface-800);
		color: var(--color-surface-50);
	}

	:global(.club-selector [data-combobox-item]) {
		color: var(--color-surface-900);
	}

	:global(.dark .club-selector [data-combobox-item]) {
		color: var(--color-surface-100);
	}

	:global(.club-selector [data-combobox-item][data-selected]) {
		background-color: var(--color-primary-500);
		color: white;
	}

	:global(.club-selector [data-combobox-item]:hover) {
		background-color: var(--color-surface-200);
	}

	:global(.dark .club-selector [data-combobox-item]:hover) {
		background-color: var(--color-surface-700);
	}

	:global(.dark .club-selector [data-combobox-item][data-selected]:hover) {
		background-color: var(--color-primary-600);
	}
</style>
