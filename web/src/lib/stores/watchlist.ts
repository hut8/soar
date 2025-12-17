import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { WatchlistEntry, WatchlistEntryWithAircraft, Aircraft } from '$lib/types';
import { FixFeed } from '$lib/services/FixFeed';

interface WatchlistState {
	entries: WatchlistEntryWithAircraft[];
	loading: boolean;
	error: string | null;
}

const initialState: WatchlistState = {
	entries: [],
	loading: false,
	error: null
};

const watchlistStore = writable<WatchlistState>(initialState);

export const watchlist = {
	subscribe: watchlistStore.subscribe,

	/**
	 * Load watchlist from API
	 */
	async load(): Promise<void> {
		if (!browser) return;

		watchlistStore.update((state) => ({ ...state, loading: true, error: null }));

		try {
			const entries = await serverCall<WatchlistEntry[]>('/watchlist', {
				method: 'GET'
			});

			// Fetch aircraft details for each entry
			const entriesWithAircraft = await Promise.all(
				entries.map(async (entry) => {
					try {
						const aircraft = await serverCall<Aircraft>(`/aircraft/${entry.aircraft_id}`, {
							method: 'GET'
						});
						return { ...entry, aircraft };
					} catch {
						return { ...entry, aircraft: undefined };
					}
				})
			);

			watchlistStore.set({
				entries: entriesWithAircraft,
				loading: false,
				error: null
			});

			// Update WebSocket subscriptions
			notifyWatchlistChange(entriesWithAircraft);
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to load watchlist';
			watchlistStore.update((state) => ({
				...state,
				loading: false,
				error: message
			}));
		}
	},

	/**
	 * Add aircraft to watchlist
	 */
	async add(aircraftId: string, sendEmail: boolean = false): Promise<void> {
		if (!browser) return;

		try {
			await serverCall<WatchlistEntry>('/watchlist', {
				method: 'POST',
				body: JSON.stringify({ aircraft_id: aircraftId, send_email: sendEmail })
			});

			// Reload watchlist
			await this.load();
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to add to watchlist';
			watchlistStore.update((state) => ({ ...state, error: message }));
			throw err;
		}
	},

	/**
	 * Remove aircraft from watchlist
	 */
	async remove(aircraftId: string): Promise<void> {
		if (!browser) return;

		try {
			await serverCall(`/watchlist/${aircraftId}`, {
				method: 'DELETE'
			});

			// Remove from local state immediately
			watchlistStore.update((state) => {
				const entries = state.entries.filter((e) => e.aircraft_id !== aircraftId);
				notifyWatchlistChange(entries);
				return { ...state, entries };
			});
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to remove from watchlist';
			watchlistStore.update((state) => ({ ...state, error: message }));
			throw err;
		}
	},

	/**
	 * Update email preference for aircraft
	 */
	async updateEmailPreference(aircraftId: string, sendEmail: boolean): Promise<void> {
		if (!browser) return;

		try {
			await serverCall(`/watchlist/${aircraftId}`, {
				method: 'PUT',
				body: JSON.stringify({ send_email: sendEmail })
			});

			// Update local state
			watchlistStore.update((state) => {
				const entries = state.entries.map((e) =>
					e.aircraft_id === aircraftId ? { ...e, send_email: sendEmail } : e
				);
				return { ...state, entries };
			});
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to update email preference';
			watchlistStore.update((state) => ({ ...state, error: message }));
			throw err;
		}
	},

	/**
	 * Clear entire watchlist
	 */
	async clear(): Promise<void> {
		if (!browser) return;

		try {
			await serverCall('/watchlist/clear', {
				method: 'DELETE'
			});

			watchlistStore.set({
				entries: [],
				loading: false,
				error: null
			});

			notifyWatchlistChange([]);
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to clear watchlist';
			watchlistStore.update((state) => ({ ...state, error: message }));
			throw err;
		}
	},

	/**
	 * Check if aircraft is in watchlist
	 */
	has(aircraftId: string): boolean {
		const state = get(watchlistStore);
		return state.entries.some((e) => e.aircraft_id === aircraftId);
	},

	/**
	 * Get active aircraft IDs for WebSocket subscriptions
	 */
	getActiveAircraftIds(): string[] {
		const state = get(watchlistStore);
		return state.entries.map((e) => e.aircraft_id);
	}
};

/**
 * Notify FixFeed about watchlist changes for WebSocket subscriptions
 */
function notifyWatchlistChange(entries: WatchlistEntryWithAircraft[]) {
	const aircraftIds = entries.map((e) => e.aircraft_id);
	const fixFeedInstance = FixFeed.getInstance();
	fixFeedInstance.subscribeToWatchlist(aircraftIds);
}

/**
 * Derived store for active watchlist entries (all entries are active now)
 */
export const activeWatchlist = derived(watchlistStore, ($store) => $store.entries);
