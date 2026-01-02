// Shared formatting utility functions for aircraft data
import countryCodes from '$lib/data/countries.json';

/**
 * Convert TitleCase strings to Title Case with spaces
 * Example: "FixedWingSingleEngine" -> "Fixed Wing Single Engine"
 * Example: "UpTo12499" -> "Up To 12499"
 */
export function formatTitleCase(value: string | null | undefined): string {
	if (!value) return 'Unknown';
	return value
		.replace(/([a-z])([A-Z])/g, '$1 $2') // Space before capital after lowercase
		.replace(/(\d)([A-Z])/g, '$1 $2') // Space before capital after digit
		.replace(/([a-zA-Z])(\d)/g, '$1 $2') // Space before digit after letter
		.trim()
		.replace(/\s+/g, ' ');
}

/**
 * Get human-readable label for address type
 * O -> OGN, F -> FLARM, I -> ICAO
 */
export function getAddressTypeLabel(addressType: string | null | undefined): string {
	if (!addressType) return 'Unknown';

	switch (addressType.toUpperCase()) {
		case 'O':
			return 'OGN';
		case 'F':
			return 'FLARM';
		case 'I':
			return 'ICAO';
		default:
			return 'Unknown';
	}
}

/**
 * Format aircraft address with appropriate prefix based on type
 * O -> OGN-XXXXXX, F -> FLARM-XXXXXX, I -> ICAO-XXXXXX
 */
export function formatAircraftAddress(
	addressType: string | null | undefined,
	address: string
): string {
	if (!address) return 'Unknown';

	const hexAddress = address.toUpperCase();

	// Map address type to prefix (handle null/undefined addressType)
	if (!addressType) return hexAddress;

	switch (addressType.toUpperCase()) {
		case 'O':
			return `OGN-${hexAddress}`;
		case 'F':
			return `FLARM-${hexAddress}`;
		case 'I':
			return `ICAO-${hexAddress}`;
		default:
			return `UNKNOWN-${hexAddress}`;
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
		recip_engine: 'Piston',
		jet_turboprop: 'Jet/Turboprop',
		unknown: 'Unknown',
		balloon: 'Balloon',
		airship: 'Airship',
		uav: 'UAV',
		static_obstacle: 'Static Obstacle'
	};

	return typeMap[aircraftType] || aircraftType;
}

/**
 * Get color class for OGN aircraft type badges
 * Returns a Skeleton UI preset color class for consistent color-coding
 */
export function getAircraftTypeColor(aircraftType: string | undefined | null): string {
	if (!aircraftType) return 'preset-filled-surface-500';

	const colorMap: Record<string, string> = {
		glider: 'preset-filled-primary-500', // Blue for gliders
		tow_tug: 'preset-filled-error-500', // Red for tow tugs
		helicopter_gyro: 'preset-filled-secondary-500', // Purple for helicopters
		skydiver_parachute: 'preset-filled-success-500', // Green for skydivers
		drop_plane: 'preset-filled-warning-500', // Orange for drop planes
		hang_glider: 'preset-filled-tertiary-500', // Cyan for hang gliders
		paraglider: 'preset-filled-tertiary-500', // Cyan for paragliders
		recip_engine: 'preset-filled-warning-500', // Orange for reciprocating engines
		jet_turboprop: 'preset-filled-error-500', // Red for jets/turboprops
		balloon: 'preset-filled-secondary-500', // Purple for balloons
		airship: 'preset-filled-secondary-500', // Purple for airships
		uav: 'preset-filled-surface-500', // Gray for UAVs
		static_obstacle: 'preset-filled-surface-500', // Gray for obstacles
		unknown: 'preset-filled-surface-500', // Gray for unknown
		reserved: 'preset-filled-surface-500' // Gray for reserved
	};

	return colorMap[aircraftType] || 'preset-filled-surface-500';
}

/**
 * Format transponder code (Mode S code) from decimal to 6-digit uppercase hex
 * Example: 10853596 -> "A59CDC"
 */
export function formatTransponderCode(transponderCode: number | null | undefined): string {
	if (transponderCode === null || transponderCode === undefined) return 'N/A';

	// Convert to hex and pad to 6 digits
	return transponderCode.toString(16).toUpperCase().padStart(6, '0');
}

/**
 * Get the title/display name for an aircraft card
 * Priority:
 * 1. If both registration and aircraftModel: "Model - Registration" (e.g., "Piper Pawnee - N4606Y")
 * 2. If only registration: registration
 * 3. If OGN aircraft type is available: "Type (HexCode)" (e.g., "Hang Glider (012345)")
 * 4. Otherwise: formatted address (e.g., "FLARM-A0B380")
 */
export function getAircraftTitle(aircraft: {
	registration?: string | null;
	aircraftModel?: string | null;
	competitionNumber?: string | null;
	addressType: string;
	address: string;
	aircraftTypeOgn?: string | null;
}): string {
	const hasRegistration = aircraft.registration && aircraft.registration.trim() !== '';
	const hasModel = aircraft.aircraftModel && aircraft.aircraftModel.trim() !== '';

	// If both registration and model are available
	if (hasRegistration && hasModel) {
		return `${aircraft.aircraftModel} - ${aircraft.registration}`;
	}

	// If only registration
	if (hasRegistration) {
		return aircraft.registration!;
	}

	// If OGN aircraft type is available (but no registration/model), show type with hex code
	if (aircraft.aircraftTypeOgn && aircraft.aircraftTypeOgn.trim() !== '') {
		const typeName = getAircraftTypeOgnDescription(aircraft.aircraftTypeOgn);
		const hexCode = aircraft.address.toUpperCase();
		return `${typeName} (${hexCode})`;
	}

	// Default to formatted address
	return formatAircraftAddress(aircraft.address, aircraft.addressType);
}

/**
 * Get country name from ISO 3166-1 alpha-2 country code
 * Example: "US" -> "United States"
 */
export function getCountryName(countryCode: string | null | undefined): string | null {
	if (!countryCode) return null;

	const upperCode = countryCode.toUpperCase();
	return countryCodes[upperCode as keyof typeof countryCodes] || null;
}

/**
 * Get flag SVG path for a country code
 * Example: "US" -> "/flags/us.svg"
 */
export function getFlagPath(countryCode: string | null | undefined): string | null {
	if (!countryCode) return null;

	const lowerCode = countryCode.toLowerCase();
	return `/flags/${lowerCode}.svg`;
}
