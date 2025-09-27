import { writable } from 'svelte/store';
import { browser, dev } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { WatchlistEntry, Fix, Device } from '$lib/types';

// Store interfaces
export interface WatchlistStore {
	entries: WatchlistEntry[];
}

export interface FixesStore {
	fixes: Fix[];
}

// Initial states
const initialWatchlistState: WatchlistStore = {
	entries: []
};

const initialFixesState: FixesStore = {
	fixes: []
};

// WebSocket connection state
let websocket: WebSocket | null = null;
let websocketUrl = '';
const currentlySubscribedDevices = new Set<string>();
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;
const reconnectDelay = 1000; // Start with 1 second

// WebSocket status store
export const websocketStatus = writable<{
	connected: boolean;
	reconnecting: boolean;
	error: string | null;
}>({
	connected: false,
	reconnecting: false,
	error: null
});

// Create watchlist store
function createWatchlistStore() {
	const { subscribe, set, update } = writable<WatchlistStore>(initialWatchlistState);

	return {
		subscribe,

		// Add device to watchlist
		add: (device: Device) => {
			update((state) => {
				const newEntry: WatchlistEntry = {
					id: Date.now().toString(),
					device,
					active: true
				};
				const newEntries = [...state.entries, newEntry];
				saveWatchlistToStorage(newEntries);
				handleWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Remove device from watchlist
		remove: (id: string) => {
			update((state) => {
				const newEntries = state.entries.filter((entry) => entry.id !== id);
				saveWatchlistToStorage(newEntries);
				handleWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Toggle device active state
		toggleActive: (id: string) => {
			update((state) => {
				const newEntries = state.entries.map((entry) =>
					entry.id === id ? { ...entry, active: !entry.active } : entry
				);
				saveWatchlistToStorage(newEntries);
				handleWatchlistChange(newEntries);
				return { entries: newEntries };
			});
		},

		// Load from localStorage
		loadFromStorage: () => {
			if (!browser) return;

			const saved = localStorage.getItem('watchlist');
			if (saved) {
				try {
					const entries = (JSON.parse(saved) as WatchlistEntry[]).filter(
						(entry) => entry.device?.id && entry.id
					);
					set({ entries });
					handleWatchlistChange(entries);
				} catch (e) {
					console.warn('Failed to load watchlist from localStorage:', e);
				}
			}
		},

		// Clear all entries
		clear: () => {
			const newEntries: WatchlistEntry[] = [];
			set({ entries: newEntries });
			saveWatchlistToStorage(newEntries);
			handleWatchlistChange(newEntries);
		}
	};
}

// Create fixes store
function createFixesStore() {
	const { subscribe, set, update } = writable<FixesStore>(initialFixesState);

	return {
		subscribe,

		// Add fixes to store
		addFixes: (newFixes: Fix[]) => {
			update((state) => {
				// Log each fix to console
				newFixes.forEach((fix) => {
					console.log('Received fix:', fix);
				});

				// Add to existing fixes (you might want to implement deduplication here)
				return { fixes: [...state.fixes, ...newFixes] };
			});
		},

		// Clear all fixes
		clear: () => {
			set(initialFixesState);
		},

		// Get fixes for a specific device
		getFixesForDevice: (deviceId: string) => {
			let currentFixes: Fix[] = [];
			subscribe((state) => {
				currentFixes = state.fixes.filter((fix) => fix.device_id === deviceId);
			})();
			return currentFixes;
		}
	};
}

// WebSocket management functions
function initializeWebSocket() {
	if (!browser) return;

	// Determine WebSocket URL based on current environment
	const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const host = dev ? 'localhost:1337' : window.location.host;
	websocketUrl = `${protocol}//${host}/data/fixes/live`;
}

function connectWebSocket() {
	if (!browser || websocket?.readyState === WebSocket.OPEN) return;

	// Update status to show connection attempt
	websocketStatus.update((status) => ({
		...status,
		reconnecting: reconnectAttempts > 0,
		error: null
	}));

	try {
		websocket = new WebSocket(websocketUrl);

		websocket.onopen = () => {
			console.log('WebSocket connected to live fixes feed');

			// Reset reconnection state
			reconnectAttempts = 0;
			websocketStatus.update((status) => ({
				connected: true,
				reconnecting: false,
				error: null
			}));

            // Subscribe to all devices in the watchlist after a small delay to ensure connection is fully ready
            setTimeout(() => {
                if (websocket?.readyState === WebSocket.OPEN) {
                    watchlist.subscribe((state) => {
                        state.entries.forEach((entry) => {
                            if (entry.active && entry.device.id) {
                                console.log('Subscribing to device after connection:', entry.device.id);
                                websocket?.send(
                                    JSON.stringify({
                                        action: 'subscribe',
                                        device_id: entry.device.id
                                    })
                                );
                                currentlySubscribedDevices.add(entry.device.id);
                            }
                        });
                    })();
                }
            }, 50); // Small delay to ensure WebSocket is fully ready
		};

		websocket.onmessage = (event) => {
			try {
				const fix = JSON.parse(event.data) as Fix;
				fixes.addFixes([fix]);
			} catch (e) {
				console.warn('Failed to parse WebSocket message:', e);
			}
		};

		websocket.onclose = (event) => {
			console.log('WebSocket disconnected', event.code, event.reason);
			websocket = null;

			// Update status
			websocketStatus.update((status) => ({
				connected: false,
				reconnecting: false,
				error: event.code !== 1000 ? `Connection lost (${event.code})` : null
			}));

			// Clear subscription tracking when connection is lost
			currentlySubscribedDevices.clear();

			// Attempt to reconnect if it wasn't a clean close
			if (event.code !== 1000 && reconnectAttempts < maxReconnectAttempts) {
				attemptReconnect();
			}
		};

		websocket.onerror = (error) => {
			console.error('WebSocket error:', error);
			websocketStatus.update((status) => ({
				...status,
				error: 'Connection failed'
			}));
		};
	} catch (e) {
		console.error('Failed to create WebSocket connection:', e);
		websocketStatus.update((status) => ({
			...status,
			error: 'Failed to create connection'
		}));
	}
}

function attemptReconnect() {
	if (reconnectAttempts >= maxReconnectAttempts) {
		console.log('Max reconnection attempts reached');
		websocketStatus.update((status) => ({
			...status,
			reconnecting: false,
			error: 'Max reconnection attempts reached'
		}));
		return;
	}

	reconnectAttempts++;
	const delay = reconnectDelay * Math.pow(2, reconnectAttempts - 1); // Exponential backoff

	console.log(`Attempting to reconnect in ${delay}ms (attempt ${reconnectAttempts}/${maxReconnectAttempts})`);

	websocketStatus.update((status) => ({
		...status,
		reconnecting: true,
		error: `Reconnecting... (${reconnectAttempts}/${maxReconnectAttempts})`
	}));

	setTimeout(() => {
		if (!websocket || websocket.readyState === WebSocket.CLOSED) {
			connectWebSocket();
		}
	}, delay);
}

function disconnectWebSocket() {
	if (websocket) {
		websocket.close(1000, 'User requested disconnection'); // Clean close
		websocket = null;
	}
	// Clear subscription tracking when disconnecting
	currentlySubscribedDevices.clear();

	// Reset reconnection attempts
	reconnectAttempts = 0;

	// Update status
	websocketStatus.update(() => ({
		connected: false,
		reconnecting: false,
		error: null
	}));
}

async function subscribeToDevice(deviceId: string) {
	if (currentlySubscribedDevices.has(deviceId)) {
		return; // Already subscribed
	}

	if (!websocket || websocket.readyState !== WebSocket.OPEN) {
		connectWebSocket();
		// Wait a bit for connection to establish
		await new Promise((resolve) => setTimeout(resolve, 100));
	}

	if (websocket?.readyState === WebSocket.OPEN) {
		websocket.send(
			JSON.stringify({
				action: 'subscribe',
				device_id: deviceId
			})
		);

		// Track that we're now subscribed to this device
		currentlySubscribedDevices.add(deviceId);

		// Also fetch recent fixes from REST endpoint
		try {
			const response = await serverCall<{ fixes: Fix[] }>(`/fixes?device_id=${deviceId}&limit=100`);
			if (response.fixes) {
				fixes.addFixes(response.fixes);
			}
		} catch (error) {
			console.warn(`Failed to fetch fixes for device ${deviceId}:`, error);
		}
	}
}

async function unsubscribeFromDevice(deviceId: string) {
	if (!currentlySubscribedDevices.has(deviceId)) {
		return; // Not subscribed
	}

	if (websocket?.readyState === WebSocket.OPEN) {
		websocket.send(
			JSON.stringify({
				action: 'unsubscribe',
				device_id: deviceId
			})
		);
	}

	// Track that we're no longer subscribed to this device
	currentlySubscribedDevices.delete(deviceId);
}

// Handle watchlist changes - subscribe/unsubscribe as needed
async function handleWatchlistChange(entries: WatchlistEntry[]) {
	if (!browser) return;

	const desiredActiveDevices = new Set(
		entries.filter((entry) => entry.active && entry.device.id).map((entry) => entry.device.id!)
	);

	// Find devices to unsubscribe from (currently subscribed but not in desired set)
	const devicesToUnsubscribe = Array.from(currentlySubscribedDevices).filter(
		(deviceId) => !desiredActiveDevices.has(deviceId)
	);

	// Find devices to subscribe to (in desired set but not currently subscribed)
	const devicesToSubscribe = Array.from(desiredActiveDevices).filter(
		(deviceId) => !currentlySubscribedDevices.has(deviceId)
	);

	// Unsubscribe from devices no longer needed
	for (const deviceId of devicesToUnsubscribe) {
		await unsubscribeFromDevice(deviceId);
	}

	// Subscribe to new devices
	if (devicesToSubscribe.length > 0) {
		// Initialize WebSocket if needed
		initializeWebSocket();
		connectWebSocket();

		for (const deviceId of devicesToSubscribe) {
			await subscribeToDevice(deviceId);
		}
	}

	// If no devices are active, disconnect WebSocket
	if (desiredActiveDevices.size === 0) {
		disconnectWebSocket();
		currentlySubscribedDevices.clear();
	}
}

// Save watchlist to localStorage
function saveWatchlistToStorage(entries: WatchlistEntry[]) {
	if (!browser) return;
	localStorage.setItem('watchlist', JSON.stringify(entries));
}

// Create store instances
export const watchlist = createWatchlistStore();
export const fixes = createFixesStore();

// Initialize WebSocket URL when module loads
if (browser) {
	initializeWebSocket();
}
