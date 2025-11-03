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
	transponder_code: number | null; // Mode S code as decimal number
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

// Device interface matching backend DeviceView exactly
export interface Device {
	id: string;
	device_address: string; // Formatted like "FLARM-A0B380"
	address_type: string; // F, O, I, or empty string
	address: string; // Hex format like "ABCDEF"
	aircraft_model: string; // Short string from device database (e.g., "Cessna 172")
	registration: string;
	competition_number: string;
	tracked: boolean;
	identified: boolean;
	club_id?: string | null;
	created_at: string;
	updated_at: string;
	from_ddb: boolean;
	frequency_mhz?: number | null;
	pilot_name?: string | null;
	home_base_airport_ident?: string | null;
	aircraft_type_ogn?: string | null;
	last_fix_at?: string | null;
	tracker_device_type?: string | null;
	icao_model_code?: string | null;
	country_code?: string | null;
	fixes?: Fix[];
}

// Aircraft extends Device with optional aircraft registration and detailed model information
// This matches the backend Aircraft struct
export interface Aircraft extends Device {
	aircraft_registration?: AircraftRegistration;
	// Detailed aircraft model information from FAA database
	aircraft_model_details?: AircraftModel;
}

export interface Fix {
	id: string;
	device_id?: string;
	device_address_hex?: string;
	timestamp: string;
	latitude: number;
	longitude: number;
	altitude_msl_feet?: number;
	altitude_agl_feet?: number;
	track_degrees?: number;
	ground_speed_knots?: number;
	climb_fpm?: number;
	registration?: string;
	model?: string;
	flight_id?: string;
	active: boolean;
	raw_packet?: string; // Raw APRS packet data (joined from aprs_messages table)
	flight?: Flight; // Full flight information if part of an active flight (from websocket)
}

// User authentication and profile
export interface User {
	id: string;
	first_name: string;
	last_name: string;
	email: string;
	access_level: 'standard' | 'admin';
	club_id?: string;
	email_verified: boolean;
}

// Flight interface matching backend FlightView
export interface Flight {
	id: string;
	device_id?: string; // UUID foreign key to devices table
	device_address: string; // Hex format like "39D304" - kept for compatibility
	device_address_type: string; // F, O, I, or empty string - kept for compatibility
	takeoff_time?: string; // ISO datetime string - null for flights first seen airborne
	landing_time?: string; // ISO datetime string - null for flights in progress
	timed_out_at?: string; // ISO datetime string when flight timed out
	state: 'active' | 'complete' | 'timed_out'; // Flight state
	duration_seconds?: number; // Duration in seconds (null if takeoff_time or landing_time is null)
	departure_airport?: string; // Airport identifier
	departure_airport_id?: number; // Airport ID in database
	departure_airport_country?: string; // Country code
	arrival_airport?: string; // Airport identifier
	arrival_airport_id?: number; // Airport ID in database
	arrival_airport_country?: string; // Country code
	towed_by_device_id?: string; // UUID of towplane device that towed this glider
	towed_by_flight_id?: string; // UUID of towplane flight that towed this glider
	club_id?: string; // UUID of club that owns the aircraft
	takeoff_altitude_offset_ft?: number; // Altitude offset at takeoff
	landing_altitude_offset_ft?: number; // Altitude offset at landing
	takeoff_runway_ident?: string; // Takeoff runway identifier
	landing_runway_ident?: string; // Landing runway identifier
	total_distance_meters?: number; // Total distance flown in meters
	maximum_displacement_meters?: number; // Maximum displacement from takeoff point
	runways_inferred?: boolean; // Whether runways were inferred from heading vs matched to airport data
	created_at: string; // ISO datetime string
	updated_at: string; // ISO datetime string
	// Device information (merged into FlightView from DeviceInfo)
	aircraft_model?: string;
	registration?: string;
	aircraft_type_ogn?: string;
	// Latest fix information (for active flights)
	latest_altitude_msl_feet: number | null;
	latest_altitude_agl_feet: number | null;
	latest_fix_timestamp: string | null;
	// Navigation to previous/next flights for the same device (chronologically by takeoff time)
	previous_flight_id?: string;
	next_flight_id?: string;
	// Flight callsign (from APRS packets)
	callsign?: string;
}

export interface WatchlistEntry {
	id: string;
	deviceId: string; // Only store device ID, not full device object
	active: boolean;
}

export interface Pilot {
	id: string;
	first_name: string;
	last_name: string;
	is_licensed: boolean;
	is_instructor: boolean;
	is_tow_pilot: boolean;
	is_examiner: boolean;
	club_id?: string;
	created_at: string;
	updated_at: string;
}
