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

export interface Runway {
	id: number;
	length_ft: number | null;
	width_ft: number | null;
	surface: string | null;
	lighted: boolean;
	closed: boolean;

	// Low-numbered end
	le_ident: string | null;
	le_latitude_deg: number | null;
	le_longitude_deg: number | null;
	le_elevation_ft: number | null;
	le_heading_degt: number | null;
	le_displaced_threshold_ft: number | null;

	// High-numbered end
	he_ident: string | null;
	he_latitude_deg: number | null;
	he_longitude_deg: number | null;
	he_elevation_ft: number | null;
	he_heading_degt: number | null;
	he_displaced_threshold_ft: number | null;
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
	}) {
		this.id = data.id || '';
		this.address_type = data.address_type;
		this.address = data.address;
		this.aircraft_model = data.aircraft_model;
		this.registration = data.registration;
		this.cn = data.cn;
		this.tracked = data.tracked;
		this.identified = data.identified;
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
	} {
		return {
			id: this.id,
			address_type: this.address_type,
			address: this.address,
			aircraft_model: this.aircraft_model,
			registration: this.registration,
			cn: this.cn,
			tracked: this.tracked,
			identified: this.identified
		};
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
	}): Device {
		return new Device({
			id: data.id,
			address_type: data.address_type,
			address: data.address,
			aircraft_model: data.aircraft_model,
			registration: data.registration,
			cn: data.cn,
			tracked: data.tracked,
			identified: data.identified
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
	device: Device;
	active: boolean;
}
