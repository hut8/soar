import { browser } from '$app/environment';
import { serverCall } from '$lib/api/server';
import { Device } from '$lib/types';
import type { Fix } from '$lib/types';

// Event types for subscribers
export type DeviceRegistryEvent =
	| { type: 'device_updated'; device: Device }
	| { type: 'fix_added'; device: Device; fix: Fix }
	| { type: 'devices_changed'; devices: Device[] };

export type DeviceRegistrySubscriber = (event: DeviceRegistryEvent) => void;

export class DeviceRegistry {
	private static instance: DeviceRegistry | null = null;
	private devices = new Map<string, Device>();
	private subscribers = new Set<DeviceRegistrySubscriber>();
	private readonly storageKeyPrefix = 'device.';

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
			return this.devices.get(deviceId)!;
		}

		// Try to load from localStorage
		const stored = this.loadDeviceFromStorage(deviceId);
		if (stored) {
			this.devices.set(deviceId, stored);
			return stored;
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
		this.devices.set(device.id, device);
		this.saveDeviceToStorage(device);

		// Notify subscribers
		this.notifySubscribers({
			type: 'device_updated',
			device
		});

		this.notifySubscribers({
			type: 'devices_changed',
			devices: this.getAllDevices()
		});
	}

	// Create or update device from backend API data
	public async updateDeviceFromAPI(deviceId: string): Promise<Device | null> {
		try {
			// API response structure
			type APIDevice = {
				id?: string;
				address_type: string;
				address: string;
				aircraft_model: string;
				registration: string;
				cn: string;
				tracked: string; // API returns "Y" for true, "" for false
				identified: string; // API returns "Y" for true, "" for false
			};

			const apiDevice = await serverCall<APIDevice>(`/devices/${deviceId}`);
			if (!apiDevice) return null;

			let cachedDevice = this.getDevice(deviceId);

			if (cachedDevice) {
				// Update existing device info
				cachedDevice.address_type = apiDevice.address_type;
				cachedDevice.address = apiDevice.address;
				cachedDevice.aircraft_model = apiDevice.aircraft_model;
				cachedDevice.registration = apiDevice.registration;
				cachedDevice.cn = apiDevice.cn;
				cachedDevice.tracked = apiDevice.tracked.toUpperCase() === 'Y';
				cachedDevice.identified = apiDevice.identified.toUpperCase() === 'Y';
			} else {
				// Create new device
				cachedDevice = Device.fromJSON({
					id: deviceId,
					address_type: apiDevice.address_type,
					address: apiDevice.address,
					aircraft_model: apiDevice.aircraft_model,
					registration: apiDevice.registration,
					cn: apiDevice.cn,
					tracked: apiDevice.tracked.toUpperCase() === 'Y',
					identified: apiDevice.identified.toUpperCase() === 'Y'
				});
			}

			this.setDevice(cachedDevice);
			return cachedDevice;
		} catch (error) {
			console.warn(`Failed to fetch device ${deviceId} from API:`, error);
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
				allowApiFallback ? 'attempting to fetch from API' : 'creating minimal device'
			);

			// Only try to fetch from API if allowed (for individual subscriptions)
			if (allowApiFallback) {
				try {
					device = await this.updateDeviceFromAPI(deviceId);
				} catch (error) {
					console.warn('[REGISTRY] Failed to fetch device from API for:', deviceId, error);
				}
			}

			// If still no device, create a minimal one
			if (!device) {
				console.log('[REGISTRY] Creating minimal device for fix:', deviceId);
				device = new Device({
					id: deviceId,
					address_type: '',
					address: fix.device_address_hex || '',
					aircraft_model: fix.model || '',
					registration: fix.registration || '',
					cn: '',
					tracked: false,
					identified: false
				});
			}
		} else {
			console.log('[REGISTRY] Using existing device:', {
				deviceId,
				registration: device.registration,
				existingFixCount: device.fixes.length
			});
		}

		device.addFix(fix);
		this.setDevice(device);

		console.log('[REGISTRY] Fix added to device. New fix count:', device.fixes.length);

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
		return Array.from(this.devices.values());
	}

	// Get all devices with recent fixes (within last hour)
	public getActiveDevices(withinHours: number = 1): Device[] {
		const cutoffTime = Date.now() - withinHours * 60 * 60 * 1000;

		return this.getAllDevices().filter((device) => {
			const latestFix = device.getLatestFix();
			if (!latestFix) return false;
			return new Date(latestFix.timestamp).getTime() > cutoffTime;
		});
	}

	// Get fixes for a specific device
	public getFixesForDevice(deviceId: string): Fix[] {
		const device = this.getDevice(deviceId);
		return device ? device.fixes : [];
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
	private saveDeviceToStorage(device: Device): void {
		if (!browser) return;

		try {
			const key = this.storageKeyPrefix + device.id;
			localStorage.setItem(key, JSON.stringify(device.toJSON()));
		} catch (error) {
			console.warn('Failed to save device to localStorage:', error);
		}
	}

	private loadDeviceFromStorage(deviceId: string): Device | null {
		if (!browser) return null;

		const key = this.storageKeyPrefix + deviceId;
		const stored = localStorage.getItem(key);

		if (stored) {
			try {
				const data = JSON.parse(stored);

				// Validate the data structure
				if (!Device.isValidDeviceData(data)) {
					console.warn(`Invalid device data structure for ${deviceId}, removing and will re-fetch`);
					localStorage.removeItem(key);
					return null;
				}

				return Device.fromJSON(data);
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

	// Update device info from complete DeviceWithFixes data
	public async updateDeviceInfo(
		deviceId: string,
		deviceData: {
			id: string;
			registration: string;
			device_address_hex: string;
			created_at: string;
			updated_at: string;
		},
		aircraftRegistration?: {
			id: string;
			device_id: string;
			tail_number: string;
			manufacturer_code: string;
			model_code: string;
			series_code: string;
			created_at: string;
			updated_at: string;
		},
		aircraftModel?: {
			manufacturer_code: string;
			model_code: string;
			series_code: string;
			manufacturer_name: string;
			model_name: string;
			aircraft_type?: string;
			engine_type?: string;
			aircraft_category?: string;
			builder_certification?: string;
			number_of_engines?: number;
			number_of_seats?: number;
			weight_class?: string;
			cruising_speed?: number;
			type_certificate_data_sheet?: string;
			type_certificate_data_holder?: string;
		}
	): Promise<Device | null> {
		try {
			let device = this.getDevice(deviceId);

			if (device) {
				// Update existing device
				device.registration = deviceData.registration || device.registration;
				// We could update other fields if the backend DeviceModel had more fields
			} else {
				// Create new device from the complete data
				device = Device.fromJSON({
					id: deviceId,
					address_type: '', // Not in DeviceModel from backend, would need to be added
					address: deviceData.device_address_hex || '',
					aircraft_model: aircraftModel?.model_name || '',
					registration: deviceData.registration || '',
					cn: '', // Not in DeviceModel from backend
					tracked: true, // Assume tracked if we received it
					identified: !!aircraftRegistration // Identified if we have registration
				});
			}

			this.setDevice(device);
			return device;
		} catch (error) {
			console.warn(`Failed to update device info for ${deviceId}:`, error);
			return null;
		}
	}
}
