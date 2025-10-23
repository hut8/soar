import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import type { Aircraft, Device, Fix } from '$lib/types';

// Event types for subscribers
export type DeviceRegistryEvent =
	| { type: 'device_updated'; device: Device }
	| { type: 'fix_added'; device: Device; fix: Fix }
	| { type: 'devices_changed'; devices: Device[] };

export type DeviceRegistrySubscriber = (event: DeviceRegistryEvent) => void;

// Internal type to store device with its fixes
interface DeviceWithFixesCache {
	device: Device;
	fixes: Fix[];
}

export class DeviceRegistry {
	private static instance: DeviceRegistry | null = null;
	private devices = new Map<string, DeviceWithFixesCache>();
	private subscribers = new Set<DeviceRegistrySubscriber>();
	private readonly storageKeyPrefix = 'device.';
	private readonly maxFixAge = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

	private constructor() {
		// Private constructor for singleton pattern
	}

	// Singleton instance getter
	public static getInstance(): DeviceRegistry {
		if (!DeviceRegistry.instance) {
			DeviceRegistry.instance = new DeviceRegistry();
		}
		return DeviceRegistry.instance;
	}

	// Subscription management
	public subscribe(subscriber: DeviceRegistrySubscriber): () => void {
		this.subscribers.add(subscriber);

		// Return unsubscribe function
		return () => {
			this.subscribers.delete(subscriber);
		};
	}

	private notifySubscribers(event: DeviceRegistryEvent): void {
		this.subscribers.forEach((subscriber) => {
			try {
				subscriber(event);
			} catch (error) {
				console.error('Error in DeviceRegistry subscriber:', error);
			}
		});
	}

	// Get a device by ID, loading from localStorage if needed
	public getDevice(deviceId: string): Device | null {
		// Check in-memory cache first
		if (this.devices.has(deviceId)) {
			const cached = this.devices.get(deviceId)!;
			// Return device with current fixes
			return { ...cached.device, fixes: cached.fixes };
		}

		// Try to load from localStorage
		const stored = this.loadDeviceFromStorage(deviceId);
		if (stored) {
			this.devices.set(deviceId, stored);
			return { ...stored.device, fixes: stored.fixes };
		}

		return null;
	}

	// Get a device by ID with automatic API fallback
	public async getDeviceWithFallback(deviceId: string): Promise<Device | null> {
		// Try to get from cache first
		let device = this.getDevice(deviceId);

		// If not found in cache, fetch from API
		if (!device) {
			console.log(`[REGISTRY] Device ${deviceId} not found in cache, fetching from API`);
			device = await this.updateDeviceFromAPI(deviceId);
		}

		return device;
	}

	// Add or update a device
	public setDevice(device: Device): void {
		// Get existing fixes if any, or use the ones from the device, or empty array
		const existingFixes = this.devices.get(device.id)?.fixes || device.fixes || [];

		this.devices.set(device.id, { device, fixes: existingFixes });
		this.saveDeviceToStorage({ device, fixes: existingFixes });

		// Notify subscribers
		this.notifySubscribers({
			type: 'device_updated',
			device: { ...device, fixes: existingFixes }
		});

		this.notifySubscribers({
			type: 'devices_changed',
			devices: this.getAllDevices()
		});
	}

	// Helper method to clean up old fixes
	private cleanupOldFixes(fixes: Fix[]): Fix[] {
		const cutoffTime = Date.now() - this.maxFixAge;
		return fixes.filter((fix) => {
			const fixTime = new Date(fix.timestamp).getTime();
			return fixTime > cutoffTime;
		});
	}

	// Add a new fix to device's fixes array
	private addFixToDeviceCache(deviceId: string, fix: Fix): void {
		const cached = this.devices.get(deviceId);
		if (!cached) return;

		// Add fix to the beginning (most recent first)
		cached.fixes.unshift(fix);

		// Clean up old fixes
		cached.fixes = this.cleanupOldFixes(cached.fixes);

		// Update the map
		this.devices.set(deviceId, cached);
	}

	// Create or update device from backend API data
	public async updateDeviceFromAPI(deviceId: string): Promise<Device | null> {
		try {
			const apiDevice = await serverCall<Device>(`/devices/${deviceId}`);
			if (!apiDevice) return null;

			this.setDevice(apiDevice);
			return this.getDevice(deviceId);
		} catch (error) {
			console.warn(`Failed to fetch device ${deviceId} from API:`, error);
			return null;
		}
	}

	// Update device from Aircraft data (from WebSocket or bbox search)
	public async updateDeviceFromAircraft(aircraft: Aircraft): Promise<Device | null> {
		try {
			// Extract the device portion
			const device: Device = {
				id: aircraft.id,
				device_address: aircraft.device_address,
				address_type: aircraft.address_type,
				address: aircraft.address,
				aircraft_model: aircraft.aircraft_model,
				registration: aircraft.registration,
				competition_number: aircraft.competition_number,
				tracked: aircraft.tracked,
				identified: aircraft.identified,
				club_id: aircraft.club_id,
				created_at: aircraft.created_at,
				updated_at: aircraft.updated_at,
				from_ddb: aircraft.from_ddb,
				frequency_mhz: aircraft.frequency_mhz,
				pilot_name: aircraft.pilot_name,
				home_base_airport_ident: aircraft.home_base_airport_ident,
				aircraft_type_ogn: aircraft.aircraft_type_ogn,
				last_fix_at: aircraft.last_fix_at,
				fixes: aircraft.fixes
			};

			this.setDevice(device);
			return this.getDevice(device.id);
		} catch (error) {
			console.warn(`Failed to update device from aircraft data:`, error);
			return null;
		}
	}

	// Add a fix to the appropriate device
	public async addFixToDevice(fix: Fix, allowApiFallback: boolean = true): Promise<Device | null> {
		console.log('[REGISTRY] Adding fix to device:', {
			deviceId: fix.device_id,
			deviceAddressHex: fix.device_address_hex,
			timestamp: fix.timestamp,
			position: { lat: fix.latitude, lng: fix.longitude }
		});

		const deviceId = fix.device_id;
		if (!deviceId) {
			console.warn('[REGISTRY] No device_id in fix, cannot add');
			return null;
		}

		let device = this.getDevice(deviceId);
		if (!device) {
			console.log(
				'[REGISTRY] Device not found in cache for fix:',
				deviceId,
				allowApiFallback ? 'attempting to fetch from API' : 'will check if API fetch needed'
			);

			// Check if the fix has a registration number
			const hasRegistration = fix.registration && fix.registration.trim() !== '';

			// Always try to fetch from API if:
			// 1. allowApiFallback is true, OR
			// 2. The fix doesn't have a registration (try to get complete data from backend)
			if (allowApiFallback || !hasRegistration) {
				try {
					console.log(
						`[REGISTRY] Fetching device from API (allowApiFallback: ${allowApiFallback}, hasRegistration: ${hasRegistration})`
					);
					device = await this.updateDeviceFromAPI(deviceId);
				} catch (error) {
					console.warn('[REGISTRY] Failed to fetch device from API for:', deviceId, error);
				}
			}

			// If still no device, create a minimal one
			if (!device) {
				console.log('[REGISTRY] Creating minimal device for fix:', deviceId);
				device = {
					id: deviceId,
					device_address: fix.device_address_hex || '',
					address_type: '',
					address: fix.device_address_hex || '',
					aircraft_model: fix.model || '',
					registration: fix.registration || '',
					competition_number: '',
					tracked: false,
					identified: false,
					club_id: null,
					created_at: new Date().toISOString(),
					updated_at: new Date().toISOString(),
					from_ddb: false,
					fixes: []
				};
			}
		} else {
			console.log('[REGISTRY] Using existing device:', {
				deviceId,
				registration: device.registration,
				existingFixCount: device.fixes?.length || 0
			});
		}

		// Add the fix using the helper method
		this.addFixToDeviceCache(deviceId, fix);

		// Get the updated device
		device = this.getDevice(deviceId)!;

		// Always persist device (setDevice will normalize registration to "Unknown" if needed)
		this.setDevice(device);

		console.log('[REGISTRY] Fix added to device. New fix count:', device.fixes?.length || 0);

		// Notify subscribers about the fix
		this.notifySubscribers({
			type: 'fix_added',
			device,
			fix
		});

		console.log('[REGISTRY] Notified subscribers about fix_added');

		return device;
	}

	// Get all devices
	public getAllDevices(): Device[] {
		return Array.from(this.devices.values()).map((cached) => ({
			...cached.device,
			fixes: cached.fixes
		}));
	}

	// Get all devices with recent fixes (within last hour)
	public getActiveDevices(withinHours: number = 1): Device[] {
		const cutoffTime = Date.now() - withinHours * 60 * 60 * 1000;

		return this.getAllDevices().filter((device) => {
			const fixes = device.fixes || [];
			if (fixes.length === 0) return false;
			const latestFix = fixes[0]; // Most recent is first
			return new Date(latestFix.timestamp).getTime() > cutoffTime;
		});
	}

	// Get fixes for a specific device
	public getFixesForDevice(deviceId: string): Fix[] {
		const cached = this.devices.get(deviceId);
		return cached ? cached.fixes : [];
	}

	// Clear all devices (for cleanup)
	public clear(): void {
		this.devices.clear();

		// Clear localStorage
		if (browser) {
			const keys = [];
			for (let i = 0; i < localStorage.length; i++) {
				const key = localStorage.key(i);
				if (key && key.startsWith(this.storageKeyPrefix)) {
					keys.push(key);
				}
			}
			keys.forEach((key) => localStorage.removeItem(key));
		}

		this.notifySubscribers({
			type: 'devices_changed',
			devices: []
		});
	}

	// Private methods for localStorage management
	private saveDeviceToStorage(cached: DeviceWithFixesCache): void {
		if (!browser) return;

		try {
			const key = this.storageKeyPrefix + cached.device.id;
			localStorage.setItem(key, JSON.stringify(cached));
		} catch (error) {
			console.warn('Failed to save device to localStorage:', error);
		}
	}

	private loadDeviceFromStorage(deviceId: string): DeviceWithFixesCache | null {
		if (!browser) return null;

		const key = this.storageKeyPrefix + deviceId;
		const stored = localStorage.getItem(key);

		if (stored) {
			try {
				const data = JSON.parse(stored) as DeviceWithFixesCache;
				return data;
			} catch (e) {
				console.warn(`Failed to parse stored device ${deviceId}:`, e);
				// Remove corrupted data
				localStorage.removeItem(key);
			}
		}

		return null;
	}

	// Batch load recent fixes for a device from API
	public async loadRecentFixesFromAPI(deviceId: string, limit: number = 100): Promise<Fix[]> {
		try {
			const response = await serverCall<{ fixes: Fix[] }>(
				`/fixes?device_id=${deviceId}&limit=${limit}`
			);
			if (response.fixes) {
				// Add fixes to device
				for (const fix of response.fixes) {
					await this.addFixToDevice(fix);
				}
				return response.fixes;
			}
			return [];
		} catch (error) {
			console.warn(`Failed to load recent fixes for device ${deviceId}:`, error);
			return [];
		}
	}
}
