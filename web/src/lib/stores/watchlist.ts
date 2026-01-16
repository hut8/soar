import { writable, derived, get } from 'svelte/store';
import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type {
	WatchlistEntry,
	WatchlistEntryWithAircraft,
	Aircraft,
	DataListResponse,
	DataResponse
} from '$lib/types';
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
			const response = await serverCall<DataListResponse<WatchlistEntry>>('/watchlist', {
				method: 'GET'
			});
			const entries = response.data;

			// Fetch aircraft details for each entry, removing invalid entries
			const entriesWithAircraft: WatchlistEntryWithAircraft[] = [];
			const entriesToRemove: string[] = [];

			await Promise.all(
				entries.map(async (entry) => {
					try {
						const aircraftResponse = await serverCall<DataResponse<Aircraft>>(
							`/aircraft/${entry.aircraftId}`,
							{
								method: 'GET'
							}
						);
						entriesWithAircraft.push({ ...entry, aircraft: aircraftResponse.data });
					} catch {
						// Aircraft not found or invalid - mark for removal
						entriesToRemove.push(entry.aircraftId);
					}
				})
			);

			// Remove invalid entries from the database
			await Promise.all(
				entriesToRemove.map(async (aircraftId) => {
					try {
						await serverCall(`/watchlist/${aircraftId}`, {
							method: 'DELETE'
						});
					} catch {
						// Ignore errors when cleaning up - entry may already be gone
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
			await serverCall<DataResponse<WatchlistEntry>>('/watchlist', {
				method: 'POST',
				body: JSON.stringify({ aircraftId: aircraftId, sendEmail: sendEmail })
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
				const entries = state.entries.filter((e) => e.aircraftId !== aircraftId);
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
				body: JSON.stringify({ sendEmail: sendEmail })
			});

			// Update local state
			watchlistStore.update((state) => {
				const entries = state.entries.map((e) =>
					e.aircraftId === aircraftId ? { ...e, sendEmail: sendEmail } : e
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
		return state.entries.some((e) => e.aircraftId === aircraftId);
	},

	/**
	 * Get active aircraft IDs for WebSocket subscriptions
	 */
	getActiveAircraftIds(): string[] {
		const state = get(watchlistStore);
		return state.entries.map((e) => e.aircraftId);
	}
};

/**
 * Notify FixFeed about watchlist changes for WebSocket subscriptions
 */
function notifyWatchlistChange(entries: WatchlistEntryWithAircraft[]) {
	const aircraftIds = entries.map((e) => e.aircraftId);
	const fixFeedInstance = FixFeed.getInstance();
	fixFeedInstance.subscribeToWatchlist(aircraftIds);
}

/**
 * Derived store for active watchlist entries (all entries are active now)
 */
export const activeWatchlist = derived(watchlistStore, ($store) => $store.entries);
