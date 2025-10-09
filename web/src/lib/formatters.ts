// Shared formatting utility functions for aircraft data

/**
 * Convert TitleCase strings to Title Case with spaces
 * Example: "FixedWingSingleEngine" -> "Fixed Wing Single Engine"
 */
export function formatTitleCase(value: string | null | undefined): string {
	if (!value) return 'Unknown';
	return value
		.replace(/([A-Z])/g, ' $1')
		.trim()
		.replace(/\s+/g, ' ');
}

/**
 * Format device address with appropriate prefix based on type
 * O -> OGN-XXXXXX, F -> FLARM-XXXXXX, I -> ICAO-XXXXXX
 */
export function formatDeviceAddress(addressType: string, address: string): string {
	if (!address) return 'Unknown';

	const hexAddress = address.toUpperCase();

	// Map address type to prefix
	switch (addressType.toUpperCase()) {
		case 'O':
			return `OGN-${hexAddress}`;
		case 'F':
			return `FLARM-${hexAddress}`;
		case 'I':
			return `ICAO-${hexAddress}`;
		default:
			return `ICAO-${hexAddress}`;
	}
}

/**
 * Convert snake_case strings to Title Case
 * Example: "fixed_wing_single_engine" -> "Fixed Wing Single Engine"
 */
export function formatSnakeCase(value: string | null | undefined): string {
	if (!value) return 'Unknown';
	return value
		.split('_')
		.map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
		.join(' ');
}

/**
 * Get human-readable description for FAA aircraft registration status codes
 */
export function getStatusCodeDescription(statusCode: string | undefined): string {
	if (!statusCode) return 'Unknown';

	const statusMap: Record<string, string> = {
		A: 'Triennial form mailed, not returned',
		D: 'Expired Dealer',
		E: 'Revoked by enforcement',
		M: 'Valid - Manufacturer dealer',
		N: 'Non-citizen corp (no flight hours)',
		R: 'Registration pending',
		S: 'Second triennial form mailed',
		T: 'Valid - Trainee',
		V: 'Valid Registration',
		W: 'Invalid/Ineffective',
		X: 'Enforcement Letter',
		Z: 'Permanent Reserved',
		'1': 'Triennial form undeliverable',
		'2': 'N-Number assigned, not registered',
		'3': 'N-Number assigned (non-type cert)',
		'4': 'N-Number assigned (import)',
		'5': 'Reserved N-Number',
		'6': 'Administratively canceled',
		'7': 'Sale reported',
		'8': 'Second triennial attempt',
		'9': 'Registration revoked',
		'10': 'Pending cancellation',
		'11': 'Non-type cert pending cancel',
		'12': 'Import pending cancellation',
		'13': 'Registration Expired',
		'14': 'First notice renewal',
		'15': 'Second notice renewal',
		'16': 'Expired pending cancellation',
		'17': 'Sale reported pending cancel',
		'18': 'Sale reported - Canceled',
		'19': 'Registration pending cancel',
		'20': 'Registration pending - Canceled',
		'21': 'Revoked pending cancellation',
		'22': 'Revoked - Canceled',
		'23': 'Expired dealer pending cancel',
		'24': 'Third notice renewal',
		'25': 'First notice registration',
		'26': 'Second notice registration',
		'27': 'Registration Expired',
		'28': 'Third notice registration',
		'29': 'Expired pending cancellation'
	};

	return statusMap[statusCode] || statusCode;
}

/**
 * Get human-readable description for OGN aircraft type codes
 */
export function getAircraftTypeOgnDescription(aircraftType: string | undefined | null): string {
	if (!aircraftType) return 'Unknown';

	const typeMap: Record<string, string> = {
		reserved: 'Reserved',
		glider: 'Glider',
		tow_tug: 'Tow/Tug',
		helicopter_gyro: 'Helicopter/Gyro',
		skydiver_parachute: 'Skydiver/Parachute',
		drop_plane: 'Drop Plane',
		hang_glider: 'Hang Glider',
		paraglider: 'Paraglider',
		recip_engine: 'Reciprocating Engine',
		jet_turboprop: 'Jet/Turboprop',
		unknown: 'Unknown',
		balloon: 'Balloon',
		airship: 'Airship',
		uav: 'UAV',
		static_obstacle: 'Static Obstacle'
	};

	return typeMap[aircraftType] || aircraftType;
}
