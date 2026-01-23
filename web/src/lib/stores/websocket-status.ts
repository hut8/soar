import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { FixFeed } from '$lib/services/FixFeed';

interface ConnectionSources {
	ogn: boolean;
	adsb: boolean;
}

interface WebSocketStatus {
	connected: boolean;
	reconnecting: boolean;
	error: string | null;
	connectionSources: ConnectionSources;
}

interface DebugStatus {
	activeWatchlistEntries: string[];
	subscribedAircraft: string[];
	activeAreaSubscriptions: number;
	operationsPageActive: boolean;
}

const initialWebSocketStatus: WebSocketStatus = {
	connected: false,
	reconnecting: false,
	error: null,
	connectionSources: {
		ogn: false,
		adsb: false
	}
};

const initialDebugStatus: DebugStatus = {
	activeWatchlistEntries: [],
	subscribedAircraft: [],
	activeAreaSubscriptions: 0,
	operationsPageActive: false
};

export const websocketStatus = writable<WebSocketStatus>(initialWebSocketStatus);
export const debugStatus = writable<DebugStatus>(initialDebugStatus);

// Initialize WebSocket status tracking
if (browser) {
	const fixFeed = FixFeed.getInstance();

	// Subscribe to FixFeed events
	fixFeed.subscribe((event) => {
		switch (event.type) {
			case 'connection_opened':
				websocketStatus.update((status) => ({
					...status,
					connected: true,
					reconnecting: false,
					error: null
				}));
				updateDebugStatus();
				break;

			case 'connection_closed':
				websocketStatus.set({
					connected: false,
					reconnecting: false,
					error: null,
					connectionSources: { ogn: false, adsb: false }
				});
				break;

			case 'connection_status':
				websocketStatus.update((status) => ({
					...status,
					connectionSources: event.status
				}));
				break;

			case 'connection_error':
				websocketStatus.update((status) => ({
					...status,
					error: 'WebSocket connection error'
				}));
				break;

			case 'reconnecting':
				websocketStatus.update((status) => ({
					...status,
					reconnecting: true
				}));
				break;

			case 'subscription_added':
			case 'subscription_removed':
				updateDebugStatus();
				break;
		}
	});

	// Update debug status from FixFeed
	function updateDebugStatus() {
		const status = fixFeed.getConnectionStatus();
		debugStatus.update((current) => ({
			...current,
			subscribedAircraft: status.subscribedAircraft,
			operationsPageActive: status.operationsPageActive
		}));
	}

	// Periodically update debug status (for polling connection state)
	setInterval(updateDebugStatus, 1000);
}
