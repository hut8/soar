/**
 * Centralized date/time formatting utilities.
 *
 * All functions accept an IANA timezone string (e.g. "America/New_York", "UTC")
 * and append the timezone abbreviation to the output so times are never ambiguous.
 */
import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';
import timezone from 'dayjs/plugin/timezone';
import relativeTime from 'dayjs/plugin/relativeTime';

dayjs.extend(utc);
dayjs.extend(timezone);
dayjs.extend(relativeTime);

/** Full date + time with timezone: "Mar 6, 2026 14:30:45 EST" */
export function formatDateTime(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('MMM D, YYYY HH:mm:ss z');
}

/** Short date + time with timezone: "Mar 6, 14:30 EST" */
export function formatShortDateTime(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('MMM D, HH:mm z');
}

/** Time-only with timezone: "14:30 EST" */
export function formatTime(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('HH:mm z');
}

/** Time-only with seconds and timezone: "14:30:45 EST" */
export function formatTimeWithSeconds(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('HH:mm:ss z');
}

/** 12-hour time with seconds and timezone: "2:30:45 PM EST" */
export function formatTime12h(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('h:mm:ss A z');
}

/** Date-only: "Mar 6, 2026" */
export function formatDate(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('MMM D, YYYY');
}

/** ISO-style date: "2026-03-06" */
export function formatISODate(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('YYYY-MM-DD');
}

/** Full date + time with seconds: "2026-03-06 14:30:45 EST" */
export function formatISODateTime(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('YYYY-MM-DD HH:mm:ss z');
}

/** Timestamp for table rows: "MMM D, HH:mm:ss EST" */
export function formatTimestamp(date: string | Date, tz: string): string {
	return dayjs(date).tz(tz).format('MMM D, HH:mm:ss z');
}

/** Relative time: "2 hours ago", "just now" */
export function formatRelative(date: string | Date): string {
	return dayjs(date).fromNow();
}
