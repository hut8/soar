<script lang="ts">
	import { Clock } from '@lucide/svelte';
	import { timezonePreference, resolvedTimezone, LOCAL_TIMEZONE } from '$lib/stores/timezone';

	let showDropdown = $state(false);

	const COMMON_TIMEZONES = [
		{ value: LOCAL_TIMEZONE, label: 'Local' },
		{ value: 'UTC', label: 'UTC' },
		{ value: 'America/New_York', label: 'US Eastern' },
		{ value: 'America/Chicago', label: 'US Central' },
		{ value: 'America/Denver', label: 'US Mountain' },
		{ value: 'America/Los_Angeles', label: 'US Pacific' },
		{ value: 'Europe/London', label: 'London' },
		{ value: 'Europe/Paris', label: 'Paris' },
		{ value: 'Europe/Berlin', label: 'Berlin' },
		{ value: 'Australia/Sydney', label: 'Sydney' }
	];

	function selectTimezone(tz: string) {
		timezonePreference.setTimezone(tz);
		showDropdown = false;
	}

	function handleClickOutside(event: MouseEvent) {
		const target = event.target as HTMLElement;
		if (!target.closest('.tz-selector')) {
			showDropdown = false;
		}
	}

	function getShortLabel(pref: string, resolved: string): string {
		if (pref === LOCAL_TIMEZONE) {
			// Show abbreviated timezone from Intl
			try {
				return (
					new Intl.DateTimeFormat('en-US', {
						timeZone: resolved,
						timeZoneName: 'short'
					})
						.formatToParts(new Date())
						.find((p) => p.type === 'timeZoneName')?.value ?? resolved
				);
			} catch {
				return resolved;
			}
		}
		if (pref === 'UTC') return 'UTC';
		try {
			return (
				new Intl.DateTimeFormat('en-US', {
					timeZone: pref,
					timeZoneName: 'short'
				})
					.formatToParts(new Date())
					.find((p) => p.type === 'timeZoneName')?.value ?? pref
			);
		} catch {
			return pref;
		}
	}
</script>

<svelte:window onclick={handleClickOutside} />

<div class="tz-selector relative">
	<button
		class="preset-tonal-surface-500 btn btn-sm"
		onclick={() => (showDropdown = !showDropdown)}
		title="Change timezone"
	>
		<Clock size={18} />
		<span class="hidden sm:inline">{getShortLabel($timezonePreference, $resolvedTimezone)}</span>
	</button>

	{#if showDropdown}
		<div class="absolute top-12 right-0 z-50 w-52 card preset-filled-surface-50-950 p-2 shadow-xl">
			<div class="mb-2 px-3 py-1 text-xs font-semibold text-surface-600 dark:text-surface-400">
				Timezone
			</div>
			{#each COMMON_TIMEZONES as tz (tz.value)}
				<button
					class="btn w-full justify-start text-sm {$timezonePreference === tz.value
						? 'preset-filled-primary-500'
						: 'preset-tonal-surface-500'} btn-sm"
					onclick={() => selectTimezone(tz.value)}
				>
					{tz.label}
				</button>
			{/each}
		</div>
	{/if}
</div>
