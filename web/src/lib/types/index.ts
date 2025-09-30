// Core data types for the application

export interface Point {
	latitude: number;
	longitude: number;
}

export interface Location {
	id: string;
	street1?: string;
	street2?: string;
	city?: string;
	state?: string;
	zip_code?: string;
	region_code?: string;
	county_mail_code?: string;
	country_mail_code?: string;
	geolocation?: Point;
	created_at: string;
	updated_at: string;
}

export interface Club {
	id: string;
	name: string;
	home_base_airport_id?: number;
	location?: Location;
	created_at: string;
	updated_at: string;
	similarity_score?: number;
	distance_meters?: number;
}

// For backward compatibility, extend Club with is_soaring for club selector
export interface ClubWithSoaring extends Club {
	is_soaring?: boolean;
}

export interface ComboboxData {
	label: string;
	value: string;
	club: ClubWithSoaring;
}

export interface RunwayEnd {
	ident: string | null;
	latitude_deg: number | null;
	longitude_deg: number | null;
	elevation_ft: number | null;
	heading_degt: number | null;
	displaced_threshold_ft: number | null;
}

export interface Runway {
	id: number;
	length_ft: number | null;
	width_ft: number | null;
	surface: string | null;
	lighted: boolean;
	closed: boolean;
	low: RunwayEnd;
	high: RunwayEnd;
}

export interface Airport {
	id: number;
	ident: string;
	airport_type: string;
	name: string;
	latitude_deg: string | null; // BigDecimal serialized as string
	longitude_deg: string | null; // BigDecimal serialized as string
	elevation_ft: number | null;
	continent: string | null;
	iso_country: string | null;
	iso_region: string | null;
	municipality: string | null;
	scheduled_service: boolean;
	icao_code: string | null;
	iata_code: string | null;
	gps_code: string | null;
	local_code: string | null;
	home_link: string | null;
	wikipedia_link: string | null;
	keywords: string | null;
	runways: Runway[];
}

// Aircraft registration information
export interface AircraftRegistration {
	n_number: string;
	serial_number: string;
	mfr_mdl_code: string;
	eng_mfr_mdl: string;
	year_mfr: number;
	type_registrant: number;
	name: string;
	registrant_name: string;
	street: string;
	street2: string;
	city: string;
	state: string;
	zip_code: string;
	region: string;
	county: string;
	country: string;
	last_action_date: string;
	cert_issue_date: string;
	certification: string;
	type_aircraft: number;
	type_engine: number;
	status_code: string;
	mode_s_code: string;
	fract_owner: string;
	air_worth_date: string;
	other_names_1: string;
	other_names_2: string;
	other_names_3: string;
	other_names_4: string;
	other_names_5: string;
	expiration_date: string;
	unique_id: string;
	kit_mfr: string;
	kit_model: string;
	mode_s_code_hex: string;
	created_at: string;
	updated_at: string;
}

// Aircraft model information
export interface AircraftModel {
	manufacturer_code: string;
	model_code: string;
	series_code: string;
	manufacturer_name: string;
	model_name: string;
	aircraft_type: string | null;
	engine_type: string | null;
	aircraft_category: string | null;
	builder_certification: string | null;
	number_of_engines: number | null;
	number_of_seats: number | null;
	weight_class: string | null;
	cruising_speed: number | null;
	type_certificate_data_sheet: string | null;
	type_certificate_data_holder: string | null;
}

// Device class with integrated caching functionality
export class Device {
	public id: string;
	public address_type: string; // F, O, I, or empty string
	public address: string; // Hex format like "ABCDEF"
	public aircraft_model: string;
	public registration: string;
	public cn: string; // Competition number
	public tracked: boolean;
	public identified: boolean;
	public aircraft: AircraftRegistration | null = null; // Lazy-loaded aircraft registration data
	public aircraftModel: AircraftModel | null = null; // Lazy-loaded aircraft model data
	public fixes: Fix[] = []; // Array with most recent fix first (index 0)
	private readonly maxFixAge = 24 * 60 * 60 * 1000; // 24 hours in milliseconds

	constructor(data: {
		id?: string;
		address_type: string;
		address: string;
		aircraft_model: string;
		registration: string;
		cn: string;
		tracked: boolean;
		identified: boolean;
		aircraft?: AircraftRegistration | null;
		aircraftModel?: AircraftModel | null;
	}) {
		this.id = data.id || '';
		this.address_type = data.address_type;
		this.address = data.address;
		this.aircraft_model = data.aircraft_model;
		this.registration = data.registration;
		this.cn = data.cn;
		this.tracked = data.tracked;
		this.identified = data.identified;
		this.aircraft = data.aircraft || null;
		this.aircraftModel = data.aircraftModel || null;
	}

	// Add a new fix, maintaining the most-recent-first order
	addFix(fix: Fix): void {
		console.log('[DEVICE] Adding fix to device:', {
			deviceId: this.id,
			registration: this.registration,
			fixTimestamp: fix.timestamp,
			currentFixCount: this.fixes.length
		});

		// Add fix to the beginning of the array (most recent first)
		this.fixes.unshift(fix);

		// Remove fixes older than 24 hours
		this.cleanupOldFixes();

		console.log('[DEVICE] Fix added. New fix count:', this.fixes.length);
	}

	// Get the most recent fix
	getLatestFix(): Fix | null {
		return this.fixes.length > 0 ? this.fixes[0] : null;
	}

	// Get all fixes within the last N hours
	getRecentFixes(hours: number = 24): Fix[] {
		const cutoffTime = Date.now() - hours * 60 * 60 * 1000;
		return this.fixes.filter((fix) => {
			const fixTime = new Date(fix.timestamp).getTime();
			return fixTime > cutoffTime;
		});
	}

	// Lazy-load aircraft registration data
	async getAircraftRegistration(): Promise<AircraftRegistration | null> {
		// Return cached data if available
		if (this.aircraft) {
			console.log('[DEVICE] Returning cached aircraft registration for:', this.id);
			return this.aircraft;
		}

		// Fetch from API if not cached
		try {
			console.log('[DEVICE] Fetching aircraft registration for device:', this.id);
			const { serverCall } = await import('$lib/api/server');
			const aircraft = await serverCall<AircraftRegistration>(
				`/devices/${this.id}/aircraft-registration`
			);

			if (aircraft) {
				console.log('[DEVICE] Fetched aircraft registration:', aircraft.n_number);
				this.aircraft = aircraft;

				// Save updated device to localStorage via DeviceRegistry
				const { DeviceRegistry } = await import('$lib/services/DeviceRegistry');
				DeviceRegistry.getInstance().setDevice(this);
			} else {
				console.log('[DEVICE] No aircraft registration found for device:', this.id);
			}

			return aircraft;
		} catch (error) {
			console.warn('[DEVICE] Failed to fetch aircraft registration:', error);
			return null;
		}
	}

	// Lazy-load aircraft model data
	async getAircraftModel(): Promise<AircraftModel | null> {
		// Return cached data if available
		if (this.aircraftModel) {
			console.log('[DEVICE] Returning cached aircraft model for:', this.id);
			return this.aircraftModel;
		}

		// Fetch from API if not cached
		try {
			console.log('[DEVICE] Fetching aircraft model for device:', this.id);
			const { serverCall } = await import('$lib/api/server');
			const aircraftModel = await serverCall<AircraftModel>(`/devices/${this.id}/aircraft/model`);

			if (aircraftModel) {
				console.log(
					'[DEVICE] Fetched aircraft model:',
					aircraftModel.manufacturer_name,
					aircraftModel.model_name
				);
				this.aircraftModel = aircraftModel;

				// Save updated device to localStorage via DeviceRegistry
				const { DeviceRegistry } = await import('$lib/services/DeviceRegistry');
				DeviceRegistry.getInstance().setDevice(this);
			} else {
				console.log('[DEVICE] No aircraft model found for device:', this.id);
			}

			return aircraftModel;
		} catch (error) {
			console.warn('[DEVICE] Failed to fetch aircraft model:', error);
			return null;
		}
	}

	// Remove fixes older than 24 hours
	private cleanupOldFixes(): void {
		const cutoffTime = Date.now() - this.maxFixAge;
		this.fixes = this.fixes.filter((fix) => {
			const fixTime = new Date(fix.timestamp).getTime();
			return fixTime > cutoffTime;
		});
	}

	// Convert to plain object for localStorage serialization
	toJSON(): {
		id: string;
		address_type: string;
		address: string;
		aircraft_model: string;
		registration: string;
		cn: string;
		tracked: boolean;
		identified: boolean;
		aircraft: AircraftRegistration | null;
		aircraftModel: AircraftModel | null;
	} {
		return {
			id: this.id,
			address_type: this.address_type,
			address: this.address,
			aircraft_model: this.aircraft_model,
			registration: this.registration,
			cn: this.cn,
			tracked: this.tracked,
			identified: this.identified,
			aircraft: this.aircraft,
			aircraftModel: this.aircraftModel
		};
	}

	// Validate device data structure
	static isValidDeviceData(data: unknown): data is {
		id?: string;
		address_type: string;
		address: string;
		aircraft_model: string;
		registration: string;
		cn: string;
		tracked: boolean;
		identified: boolean;
		aircraft?: AircraftRegistration | null;
		aircraftModel?: AircraftModel | null;
	} {
		if (!data || typeof data !== 'object') {
			return false;
		}

		const obj = data as Record<string, unknown>;
		return (
			typeof obj.address_type === 'string' &&
			typeof obj.address === 'string' &&
			typeof obj.aircraft_model === 'string' &&
			typeof obj.registration === 'string' &&
			typeof obj.cn === 'string' &&
			typeof obj.tracked === 'boolean' &&
			typeof obj.identified === 'boolean' &&
			(obj.aircraft === null || obj.aircraft === undefined || typeof obj.aircraft === 'object') &&
			(obj.aircraftModel === null ||
				obj.aircraftModel === undefined ||
				typeof obj.aircraftModel === 'object')
		);
	}

	// Create from plain object (localStorage deserialization or API response)
	static fromJSON(data: {
		id?: string;
		address_type: string;
		address: string;
		aircraft_model: string;
		registration: string;
		cn: string;
		tracked: boolean;
		identified: boolean;
		aircraft?: AircraftRegistration | null;
		aircraftModel?: AircraftModel | null;
	}): Device {
		return new Device({
			id: data.id,
			address_type: data.address_type,
			address: data.address,
			aircraft_model: data.aircraft_model,
			registration: data.registration,
			cn: data.cn,
			tracked: data.tracked,
			identified: data.identified,
			aircraft: data.aircraft || null,
			aircraftModel: data.aircraftModel || null
		});
	}
}

export interface Fix {
	id: string;
	device_id?: string;
	device_address_hex?: string;
	timestamp: string;
	latitude: number;
	longitude: number;
	altitude_feet?: number;
	track_degrees?: number;
	ground_speed_knots?: number;
	climb_fpm?: number;
	registration?: string;
	model?: string;
	flight_id?: string;
}

export interface Flight {
	id: string;
	device_id?: string; // UUID foreign key to devices table
	device_address: string; // Hex format like "39D304" - kept for compatibility
	device_address_type: string; // F, O, I, or empty string - kept for compatibility
	takeoff_time?: string; // ISO datetime string - null for flights first seen airborne
	landing_time?: string; // ISO datetime string - null for flights in progress
	departure_airport?: string; // Airport identifier
	arrival_airport?: string; // Airport identifier
	tow_aircraft_id?: string; // Registration number of tow aircraft
	tow_release_height_msl?: number; // Tow release height in meters MSL
	club_id?: string; // UUID of club that owns the aircraft
	created_at: string; // ISO datetime string
	updated_at: string; // ISO datetime string
}

export interface WatchlistEntry {
	id: string;
	deviceId: string; // Only store device ID, not full device object
	active: boolean;
}
