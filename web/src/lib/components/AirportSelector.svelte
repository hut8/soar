<script lang="ts">
	import { Combobox } from '@skeletonlabs/skeleton-svelte';
	import { onMount, onDestroy } from 'svelte';
	import { serverCall } from '$lib/api/server';
	import type { Airport, DataListResponse } from '$lib/types';
	import { getLogger } from '$lib/logging';

	const logger = getLogger(['soar', 'AirportSelector']);

	interface Props {
		value?: string[];
		placeholder?: string;
		label?: string;
		disabled?: boolean;
		required?: boolean;
		initialAirportId?: number;
		onSelect?: (airport: Airport | null) => void;
	}

	let {
		value = $bindable([]),
		placeholder = 'Search airports by name or identifier...',
		label = 'Airport',
		disabled = false,
		required = false,
		initialAirportId,
		onSelect
	}: Props = $props();

	let airports: Airport[] = $state([]);
	let error = $state('');
	let searchTimeout: ReturnType<typeof setTimeout>;

	interface AirportComboboxItem {
		label: string;
		value: string;
		airport: Airport;
	}

	let comboboxData: AirportComboboxItem[] = $derived(
		airports.map((airport) => ({
			label: formatAirportLabel(airport),
			value: String(airport.id),
			airport
		}))
	);

	function formatAirportLabel(airport: Airport): string {
		const code = airport.icaoCode || airport.ident;
		const municipality = airport.municipality ? ` (${airport.municipality})` : '';
		return `${code} - ${airport.name}${municipality}`;
	}

	async function loadAirports(query: string) {
		if (!query.trim()) {
			airports = [];
			return;
		}
		try {
			error = '';
			const endpoint = `/airports?limit=20&q=${encodeURIComponent(query.trim())}`;
			const response = await serverCall<DataListResponse<Airport>>(endpoint);
			airports = response.data;
		} catch (err) {
			logger.error('Failed to load airports: {error}', { error: err });
			error = err instanceof Error ? err.message : 'Failed to load airports';
			airports = [];
		}
	}

	async function loadAirportById(id: number) {
		try {
			const response = await serverCall<{ data: Airport }>(`/airports/${id}`);
			airports = [response.data];
			value = [String(id)];
			if (onSelect) onSelect(response.data);
		} catch (err) {
			logger.error('Failed to load airport {id}: {error}', { id, error: err });
		}
	}

	function handleInputValueChange(details: { inputValue: string }) {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			loadAirports(details.inputValue);
		}, 300);
	}

	function handleValueChange(details: { value: string[] }) {
		value = details.value;
		if (onSelect) {
			if (details.value.length > 0) {
				const selected = airports.find((a) => String(a.id) === details.value[0]);
				onSelect(selected || null);
			} else {
				onSelect(null);
			}
		}
	}

	onMount(() => {
		if (initialAirportId) {
			loadAirportById(initialAirportId);
		}
	});

	onDestroy(() => {
		clearTimeout(searchTimeout);
	});
</script>

<div class="airport-selector">
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
			<Combobox.Content
				class="border border-surface-300 bg-surface-50 shadow-lg dark:border-surface-600 dark:bg-surface-800"
			>
				{#each comboboxData as item (item.value)}
					<Combobox.Item item={{ label: item.label, value: item.value }}>
						<div class="flex w-full items-center gap-2 bg-surface-50 dark:bg-surface-800">
							<span class="flex-1 text-left text-sm">{item.label}</span>
						</div>
					</Combobox.Item>
				{/each}
				{#if comboboxData.length === 0}
					<div class="p-3 text-center text-sm text-surface-500">No airports found</div>
				{/if}
			</Combobox.Content>
		</Combobox.Positioner>
	</Combobox>
</div>

<style>
	.airport-selector {
		width: 100%;
	}

	:global(.airport-selector [data-popover-content]) {
		background-color: var(--color-surface-50);
		color: var(--color-surface-900);
	}

	:global(.dark .airport-selector [data-popover-content]) {
		background-color: var(--color-surface-800);
		color: var(--color-surface-50);
	}

	:global(.airport-selector [data-combobox-item]) {
		color: var(--color-surface-900);
	}

	:global(.dark .airport-selector [data-combobox-item]) {
		color: var(--color-surface-100);
	}

	:global(.airport-selector [data-combobox-item][data-selected]) {
		background-color: var(--color-primary-500);
		color: white;
	}

	:global(.airport-selector [data-combobox-item]:hover) {
		background-color: var(--color-surface-200);
	}

	:global(.dark .airport-selector [data-combobox-item]:hover) {
		background-color: var(--color-surface-700);
	}

	:global(.dark .airport-selector [data-combobox-item][data-selected]:hover) {
		background-color: var(--color-primary-600);
	}
</style>
