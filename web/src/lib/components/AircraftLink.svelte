<script lang="ts">
	import { Plane, ExternalLink } from '@lucide/svelte';
	import { getAircraftTitle } from '$lib/formatters';
	import type { Aircraft } from '$lib/types';

	let {
		aircraft,
		size = 'md',
		openInNewTab = false,
		showIcon = false
	}: {
		aircraft: Aircraft;
		size?: 'sm' | 'md' | 'lg';
		openInNewTab?: boolean;
		showIcon?: boolean;
	} = $props();

	// Size classes for text
	const sizeClasses = {
		sm: 'text-xs',
		md: 'text-sm',
		lg: 'text-base'
	};

	// Icon sizes
	const iconSizes = {
		sm: 'h-3 w-3',
		md: 'h-4 w-4',
		lg: 'h-5 w-5'
	};

	// Flag sizes (match text height)
	const flagSizes = {
		sm: 'h-3',
		md: 'h-3.5',
		lg: 'h-4'
	};

	// Get aircraft ID and title
	const id = $derived(aircraft.id);
	const title = $derived(() => getAircraftTitle(aircraft));

	// Get country code
	const countryCode = $derived(() => {
		const code = aircraft.countryCode;
		return code && code.trim() !== '' ? code.toUpperCase() : null;
	});

	// Flag SVG URL from local flags directory
	const flagUrl = $derived(() => {
		const code = countryCode();
		return code ? `/flags/${code.toLowerCase()}.svg` : null;
	});
</script>

<a
	href="/aircraft/{id}"
	target={openInNewTab ? '_blank' : undefined}
	rel={openInNewTab ? 'noopener noreferrer' : undefined}
	class="inline-flex items-center gap-1 anchor {sizeClasses[size]}"
	title={title()}
>
	{#if showIcon}
		<Plane class={iconSizes[size]} />
	{/if}
	{#if flagUrl()}
		<img src={flagUrl()} alt="" class="{flagSizes[size]} inline-block rounded-sm" />
	{/if}
	<span>{title()}</span>
	{#if openInNewTab}
		<ExternalLink class={iconSizes[size]} />
	{/if}
</a>
