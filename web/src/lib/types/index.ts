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
