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

export interface Device {
	id?: string; // UUID from backend (optional for devices from external sources)
	address_type: string; // F, O, I, or empty string
	address: string; // Hex format like "ABCDEF"
	aircraft_model: string;
	registration: string;
	cn: string; // Competition number
	tracked: boolean;
	identified: boolean;
}

export interface Fix {
	id: string;
	device_id: string;
	timestamp: string;
	latitude: number;
	longitude: number;
	altitude: number;
	track: number;
	ground_speed: number;
	climb_rate: number;
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
