import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import timezone from 'dayjs/plugin/timezone';

dayjs.extend(utc);
dayjs.extend(timezone);

const STORAGE_KEY = 'timezone';

/** Special value meaning "use the browser's local timezone" */
export const LOCAL_TIMEZONE = 'local';

function getBrowserTimezone(): string {
	return Intl.DateTimeFormat().resolvedOptions().timeZone;
}

function isValidTimezone(tz: string): boolean {
	if (tz === LOCAL_TIMEZONE || tz === 'UTC') return true;
	try {
		Intl.DateTimeFormat(undefined, { timeZone: tz });
		return true;
	} catch {
		return false;
	}
}

function createTimezoneStore() {
	const stored = browser ? localStorage.getItem(STORAGE_KEY) : null;
	const initial = stored && isValidTimezone(stored) ? stored : LOCAL_TIMEZONE;
	const { subscribe, set } = writable<string>(initial);

	return {
		subscribe,
		setTimezone: (tz: string) => {
			if (!isValidTimezone(tz)) {
				tz = LOCAL_TIMEZONE;
			}
			set(tz);
			if (browser) {
				localStorage.setItem(STORAGE_KEY, tz);
			}
		}
	};
}

export const timezonePreference = createTimezoneStore();

/** The resolved IANA timezone string (never "local") */
export const resolvedTimezone = derived(timezonePreference, ($pref) =>
	$pref === LOCAL_TIMEZONE ? getBrowserTimezone() : $pref
);

/**
 * Short timezone abbreviation for the current preference (e.g. "EST", "UTC", "PDT").
 * Uses a sample date to resolve the abbreviation.
 */
export const timezoneAbbreviation = derived(resolvedTimezone, ($tz) => {
	return dayjs().tz($tz).format('z');
});
