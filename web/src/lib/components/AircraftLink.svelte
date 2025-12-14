<script lang="ts">
	import { Plane, ExternalLink } from '@lucide/svelte';
	import { getAircraftTitle } from '$lib/formatters';
	import type { Aircraft, Flight } from '$lib/types';

	let {
		aircraft,
		aircraftId,
		size = 'md',
		openInNewTab = false,
		showIcon = false
	}: {
		aircraft?: Aircraft | Flight;
		aircraftId?: string;
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

	// Helper to format device address for Flight objects
	function formatDeviceAddress(address: string, addressType?: string): string {
		if (!addressType) return address;
		const typePrefix = addressType === 'Flarm' ? 'FLARM' : addressType === 'Ogn' ? 'OGN' : 'ICAO';
		return `${typePrefix}-${address}`;
	}

	// Get aircraft ID and title
	const id = $derived(
		aircraftId ||
			('aircraft_id' in (aircraft || {}) ? (aircraft as Flight)?.aircraft_id : aircraft!.id) ||
			''
	);

	const title = $derived(() => {
		if (!aircraft) return '';

		// Check if this is a Flight object (has aircraft_id instead of id)
		if ('aircraft_id' in aircraft) {
			const flight = aircraft as Flight;
			return getAircraftTitle({
				registration: flight.registration,
				aircraft_model: flight.aircraft_model,
				competition_number: null,
				device_address: formatDeviceAddress(flight.device_address, flight.device_address_type)
			});
		}

		// It's an Aircraft object
		return getAircraftTitle(aircraft as Aircraft);
	});

	// Get country code (only available on Aircraft objects, not Flight)
	const countryCode = $derived(() => {
		if (!aircraft) return null;
		// Only Aircraft objects have country_code
		if ('country_code' in aircraft) {
			const code = (aircraft as Aircraft).country_code;
			return code && code.trim() !== '' ? code.toUpperCase() : null;
		}
		return null;
	});

	// Flag SVG URL from hampusborgos/country-flags repository
	const flagUrl = $derived(() => {
		const code = countryCode();
		return code
			? `https://cdn.jsdelivr.net/gh/hampusborgos/country-flags@main/svg/${code}.svg`
			: null;
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
