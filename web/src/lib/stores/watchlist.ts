import { writable } from 'svelte/store';
import { browser, dev } from '$app/environment';
import { serverCall } from '$lib/api/server';
import { Device } from '$lib/types';
import type { WatchlistEntry, Fix } from '$lib/types';
import { DeviceRegistry } from '$lib/services/DeviceRegistry';

// Store interfaces
export interface WatchlistStore {
	entries: WatchlistEntry[];
}

export interface DeviceRegistryStore {
	registry: DeviceRegistry;
	lastUpdate: number; // Timestamp of last update for reactivity
}

// Initial states
const initialWatchlistState: WatchlistStore = {
	entries: []
};

const initialDeviceRegistryState: DeviceRegistryStore = {
	registry: DeviceRegistry.getInstance(),
	lastUpdate: Date.now()
};

// WebSocket connection state
let websocket: WebSocket | null = null;
let websocketUrl = '';
const currentlySubscribedDevices = new Set<string>();
let reconnectAttempts = 0;
const reconnectDelay = 1000; // Start with 1 second
let operationsPageActive = false; // Track if operations page is using live fixes

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
		add: (deviceId: string) => {
			update((state) => {
				// Check if device is already in watchlist
				const existingEntry = state.entries.find(entry => entry.deviceId === deviceId);
				if (existingEntry) {
					console.log('Device already in watchlist:', deviceId);
					return state;
				}

				const newEntry: WatchlistEntry = {
					id: Date.now().toString(),
					deviceId,
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
					const rawEntries = JSON.parse(saved);
					const entries: WatchlistEntry[] = [];

					// Handle both old format (with device objects) and new format (with deviceId)
					for (const entry of rawEntries) {
						if (entry.deviceId && entry.id) {
							// New format - just validate and use
							entries.push({
								id: entry.id,
								deviceId: entry.deviceId,
								active: entry.active
							});
						} else if (entry.device?.id && entry.id) {
							// Old format - convert to new format
							entries.push({
								id: entry.id,
								deviceId: entry.device.id,
								active: entry.active
							});
						}
					}

					console.log(`Loaded ${entries.length} valid watchlist entries from storage`);
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

// Create device registry store
function createDeviceRegistryStore() {
	const { subscribe, set, update } = writable<DeviceRegistryStore>(initialDeviceRegistryState);

	return {
		subscribe,

		// Add fixes to appropriate devices
		addFixes: (newFixes: Fix[]) => {
			update((state) => {
				// Log each fix to console
				newFixes.forEach(async (fix) => {
					console.log('Received fix:', fix);
					// Add fix to the device registry
					try {
						await state.registry.addFixToDevice(fix);
					} catch (error) {
						console.warn('Failed to add fix to device registry:', error);
					}
				});

				// Update timestamp to trigger reactivity
				return {
					registry: state.registry,
					lastUpdate: Date.now()
				};
			});
		},

		// Update device information from API
		updateDeviceFromAPI: async (deviceId: string) => {
			update((state) => {
				state.registry.updateDeviceFromAPI(deviceId);
				return {
					registry: state.registry,
					lastUpdate: Date.now()
				};
			});
		},

		// Get a specific device
		getDevice: (deviceId: string): Device | null => {
			let currentDevice: Device | null = null;
			subscribe((state) => {
				currentDevice = state.registry.getDevice(deviceId);
			})();
			return currentDevice;
		},

		// Get all devices with recent fixes
		getActiveDevices: (): Device[] => {
			let devices: Device[] = [];
			subscribe((state) => {
				devices = state.registry.getAllDevices().filter((device) => {
					const latestFix = device.getLatestFix();
					if (!latestFix) return false;
					// Consider active if last fix was within the last hour
					const oneHourAgo = Date.now() - 60 * 60 * 1000;
					return new Date(latestFix.timestamp).getTime() > oneHourAgo;
				});
			})();
			return devices;
		},

		// Get fixes for a specific device
		getFixesForDevice: (deviceId: string): Fix[] => {
			let deviceFixes: Fix[] = [];
			subscribe((state) => {
				const device = state.registry.getDevice(deviceId);
				deviceFixes = device ? device.fixes : [];
			})();
			return deviceFixes;
		},

		// Clear all devices
		clear: () => {
			DeviceRegistry.getInstance().clear();
			set({
				registry: DeviceRegistry.getInstance(),
				lastUpdate: Date.now()
			});
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
	if (!browser) return;

	// Prevent multiple connections
	if (websocket?.readyState === WebSocket.OPEN || websocket?.readyState === WebSocket.CONNECTING) {
		console.log('WebSocket already connected or connecting, skipping connection attempt');
		return;
	}

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
			websocketStatus.update(() => ({
				connected: true,
				reconnecting: false,
				error: null
			}));

			// Subscribe to all active devices in the watchlist after connection is ready
			setTimeout(() => {
				if (websocket?.readyState === WebSocket.OPEN) {
					// Get current watchlist state without creating a subscription
					let currentState: WatchlistStore = { entries: [] };
					const unsubscribe = watchlist.subscribe((state) => {
						currentState = state;
					});
					unsubscribe(); // Immediately unsubscribe to avoid creating permanent subscription

					currentState.entries.forEach((entry) => {
						if (
							entry.active &&
							entry.deviceId &&
							!currentlySubscribedDevices.has(entry.deviceId)
						) {
							console.log('Subscribing to device after connection:', entry.deviceId);
							websocket?.send(
								JSON.stringify({
									action: 'subscribe',
									device_id: entry.deviceId
								})
							);
							currentlySubscribedDevices.add(entry.deviceId);
						}
					});
				}
			}, 50); // Small delay to ensure WebSocket is fully ready
		};

		websocket.onmessage = async (event) => {
			try {
				const fix = JSON.parse(event.data) as Fix;
				await DeviceRegistry.getInstance().addFixToDevice(fix);
			} catch (e) {
				console.warn('Failed to parse WebSocket message:', e);
			}
		};

		websocket.onclose = (event) => {
			console.log('WebSocket disconnected', event.code, event.reason);
			websocket = null;

			// Update status
			websocketStatus.update(() => ({
				connected: false,
				reconnecting: false,
				error: event.code !== 1000 ? `Connection lost (${event.code})` : null
			}));

			// Clear subscription tracking when connection is lost
			currentlySubscribedDevices.clear();

			// Attempt to reconnect if it wasn't a clean close
			if (event.code !== 1000) {
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
	reconnectAttempts++;
	const delay = reconnectDelay * Math.pow(2, reconnectAttempts - 1); // Exponential backoff

	console.log(`Attempting to reconnect in ${delay}ms (attempt ${reconnectAttempts})`);

	websocketStatus.update((status) => ({
		...status,
		reconnecting: true,
		error: `Reconnecting... (${reconnectAttempts})`
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

		// Fetch device information and recent fixes from REST endpoints
		try {
			// Fetch device information
			await deviceRegistry.updateDeviceFromAPI(deviceId);

			// Fetch recent fixes
			const fixesResponse = await serverCall<{ fixes: Fix[] }>(
				`/fixes?device_id=${deviceId}&limit=100`
			);
			if (fixesResponse.fixes) {
				// Add fixes one by one asynchronously
				for (const fix of fixesResponse.fixes) {
					try {
						await DeviceRegistry.getInstance().addFixToDevice(fix);
					} catch (error) {
						console.warn('Failed to add historical fix:', error);
					}
				}
			}
		} catch (error) {
			console.warn(`Failed to fetch data for device ${deviceId}:`, error);
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
		entries.filter((entry) => entry.active && entry.deviceId).map((entry) => entry.deviceId)
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

	// Only disconnect WebSocket if no devices are active AND operations page is not active
	if (desiredActiveDevices.size === 0 && !operationsPageActive) {
		disconnectWebSocket();
		currentlySubscribedDevices.clear();
	}
}

// Save watchlist to localStorage
function saveWatchlistToStorage(entries: WatchlistEntry[]) {
	if (!browser) return;
	localStorage.setItem('watchlist', JSON.stringify(entries));
}

// Function to connect and listen for all live fixes (for operations page)
export function startLiveFixesFeed() {
	if (!browser) return;

	console.log('Starting live fixes feed for operations page');
	operationsPageActive = true;
	initializeWebSocket();
	connectWebSocket();
}

// Function to stop live fixes feed
export function stopLiveFixesFeed() {
	if (!browser) return;

	console.log('Stopping live fixes feed for operations page');
	operationsPageActive = false;

	// Only disconnect if there are no active watchlist devices
	const hasActiveDevices = currentlySubscribedDevices.size > 0;
	if (!hasActiveDevices) {
		disconnectWebSocket();
	}
}

// Create store instances
export const watchlist = createWatchlistStore();
export const deviceRegistry = createDeviceRegistryStore();

// Backward compatibility - provide a way to get all recent fixes
export const fixes = {
	subscribe: deviceRegistry.subscribe,
	addFixes: deviceRegistry.addFixes,
	clear: deviceRegistry.clear,
	// Get all recent fixes from all devices
	getAllRecentFixes: (): Fix[] => {
		const devices = deviceRegistry.getActiveDevices();
		const allFixes: Fix[] = [];
		devices.forEach((device) => {
			allFixes.push(...device.fixes);
		});
		// Sort by timestamp, most recent first
		return allFixes.sort(
			(a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
		);
	}
};

// Initialize WebSocket URL when module loads
if (browser) {
	initializeWebSocket();
}
