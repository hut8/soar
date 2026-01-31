// Shared formatting utility functions for aircraft data
import countryCodes from '$lib/data/countries.json';
import type { Aircraft } from '$lib/types';

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
 * Get human-readable description for aircraft category codes
 * Handles both PascalCase (from TypeScript) and snake_case (legacy) formats
 */
export function getAircraftCategoryDescription(category: string | undefined | null): string {
	if (!category) return 'Unknown';

	const categoryMap: Record<string, string> = {
		// PascalCase (from TypeScript/Rust)
		Landplane: 'Fixed Wing',
		Helicopter: 'Helicopter',
		Balloon: 'Balloon',
		Amphibian: 'Amphibian',
		Gyroplane: 'Gyroplane',
		Drone: 'Drone/UAV',
		PoweredParachute: 'Powered Parachute',
		Rotorcraft: 'Rotorcraft',
		Seaplane: 'Seaplane',
		Tiltrotor: 'Tiltrotor',
		Vtol: 'VTOL',
		Electric: 'Electric',
		Glider: 'Glider',
		TowTug: 'Tow/Tug',
		Paraglider: 'Paraglider',
		HangGlider: 'Hang Glider',
		Airship: 'Airship',
		SkydiverParachute: 'Skydiver/Parachute',
		StaticObstacle: 'Static Obstacle',
		Unknown: 'Unknown',
		// Legacy snake_case support
		landplane: 'Fixed Wing',
		helicopter: 'Helicopter',
		balloon: 'Balloon',
		amphibian: 'Amphibian',
		gyroplane: 'Gyroplane',
		drone: 'Drone/UAV',
		powered_parachute: 'Powered Parachute',
		rotorcraft: 'Rotorcraft',
		seaplane: 'Seaplane',
		tiltrotor: 'Tiltrotor',
		vtol: 'VTOL',
		electric: 'Electric',
		glider: 'Glider',
		tow_tug: 'Tow/Tug',
		paraglider: 'Paraglider',
		hang_glider: 'Hang Glider',
		airship: 'Airship',
		skydiver_parachute: 'Skydiver/Parachute',
		static_obstacle: 'Static Obstacle',
		unknown: 'Unknown'
	};

	return categoryMap[category] || category;
}

/**
 * @deprecated Use getAircraftCategoryDescription instead
 * Get human-readable description for OGN aircraft type codes (legacy alias)
 */
export function getAircraftTypeOgnDescription(aircraftType: string | undefined | null): string {
	return getAircraftCategoryDescription(aircraftType);
}

/**
 * Get color class for aircraft category badges
 * Returns a Skeleton UI preset color class for consistent color-coding
 * Handles both PascalCase (from TypeScript) and snake_case (legacy) formats
 */
export function getAircraftCategoryColor(category: string | undefined | null): string {
	if (!category) return 'preset-filled-surface-500';

	const colorMap: Record<string, string> = {
		// PascalCase
		Glider: 'preset-filled-primary-500', // Blue for gliders
		TowTug: 'preset-filled-error-500', // Red for tow tugs
		Helicopter: 'preset-filled-secondary-500', // Purple for helicopters
		Gyroplane: 'preset-filled-secondary-500', // Purple for gyroplanes
		SkydiverParachute: 'preset-filled-success-500', // Green for skydivers
		HangGlider: 'preset-filled-tertiary-500', // Cyan for hang gliders
		Paraglider: 'preset-filled-tertiary-500', // Cyan for paragliders
		Landplane: 'preset-filled-warning-500', // Orange for fixed-wing
		Balloon: 'preset-filled-secondary-500', // Purple for balloons
		Airship: 'preset-filled-secondary-500', // Purple for airships
		Drone: 'preset-filled-surface-500', // Gray for drones
		StaticObstacle: 'preset-filled-surface-500', // Gray for obstacles
		Unknown: 'preset-filled-surface-500', // Gray for unknown
		Seaplane: 'preset-filled-primary-500', // Blue for seaplanes
		Amphibian: 'preset-filled-primary-500', // Blue for amphibians
		Rotorcraft: 'preset-filled-secondary-500', // Purple for rotorcraft
		Tiltrotor: 'preset-filled-secondary-500', // Purple for tiltrotor
		Vtol: 'preset-filled-secondary-500', // Purple for VTOL
		Electric: 'preset-filled-success-500', // Green for electric
		PoweredParachute: 'preset-filled-tertiary-500', // Cyan for powered parachutes
		// Legacy snake_case
		glider: 'preset-filled-primary-500',
		tow_tug: 'preset-filled-error-500',
		helicopter: 'preset-filled-secondary-500',
		gyroplane: 'preset-filled-secondary-500',
		skydiver_parachute: 'preset-filled-success-500',
		hang_glider: 'preset-filled-tertiary-500',
		paraglider: 'preset-filled-tertiary-500',
		landplane: 'preset-filled-warning-500',
		balloon: 'preset-filled-secondary-500',
		airship: 'preset-filled-secondary-500',
		drone: 'preset-filled-surface-500',
		static_obstacle: 'preset-filled-surface-500',
		unknown: 'preset-filled-surface-500',
		seaplane: 'preset-filled-primary-500',
		amphibian: 'preset-filled-primary-500',
		rotorcraft: 'preset-filled-secondary-500',
		tiltrotor: 'preset-filled-secondary-500',
		vtol: 'preset-filled-secondary-500',
		electric: 'preset-filled-success-500',
		powered_parachute: 'preset-filled-tertiary-500'
	};

	return colorMap[category] || 'preset-filled-surface-500';
}

/**
 * @deprecated Use getAircraftCategoryColor instead
 * Get color class for OGN aircraft type badges (legacy alias)
 */
export function getAircraftTypeColor(aircraftType: string | undefined | null): string {
	return getAircraftCategoryColor(aircraftType);
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
 * Get the primary address from an aircraft with typed address fields.
 * Returns the first non-null address with its type label, preferring ICAO > Flarm > OGN > Other.
 */
export function getPrimaryAddress(
	aircraft: Pick<Aircraft, 'icaoAddress' | 'flarmAddress' | 'ognAddress' | 'otherAddress'>
): { label: string; hex: string } | null {
	if (aircraft.icaoAddress) return { label: 'ICAO', hex: aircraft.icaoAddress };
	if (aircraft.flarmAddress) return { label: 'FLARM', hex: aircraft.flarmAddress };
	if (aircraft.ognAddress) return { label: 'OGN', hex: aircraft.ognAddress };
	if (aircraft.otherAddress) return { label: 'OTHER', hex: aircraft.otherAddress };
	return null;
}

/**
 * Format all known addresses for an aircraft (for display in detail views).
 * Returns an array of { label, hex } for each non-null address.
 */
export function getAllAddresses(
	aircraft: Pick<Aircraft, 'icaoAddress' | 'flarmAddress' | 'ognAddress' | 'otherAddress'>
): { label: string; hex: string }[] {
	const addresses: { label: string; hex: string }[] = [];
	if (aircraft.icaoAddress) addresses.push({ label: 'ICAO', hex: aircraft.icaoAddress });
	if (aircraft.flarmAddress) addresses.push({ label: 'FLARM', hex: aircraft.flarmAddress });
	if (aircraft.ognAddress) addresses.push({ label: 'OGN', hex: aircraft.ognAddress });
	if (aircraft.otherAddress) addresses.push({ label: 'OTHER', hex: aircraft.otherAddress });
	return addresses;
}

/**
 * Format an aircraft's primary address as "LABEL-HEXCODE" (e.g., "ICAO-A59CDC")
 */
export function formatPrimaryAddress(
	aircraft: Pick<Aircraft, 'icaoAddress' | 'flarmAddress' | 'ognAddress' | 'otherAddress'>
): string {
	const primary = getPrimaryAddress(aircraft);
	if (!primary) return 'Unknown';
	return `${primary.label}-${primary.hex}`;
}

/**
 * Get the title/display name for an aircraft card
 * Priority:
 * 1. If both registration and aircraftModel: "Model - Registration" (e.g., "Piper Pawnee - N4606Y")
 * 2. If only registration: registration
 * 3. If aircraft category is available: "Type (HexCode)" (e.g., "Hang Glider (012345)")
 * 4. Otherwise: formatted primary address (e.g., "ICAO-A0B380", "FLARM-A0B380", "OGN-A0B380", or "OTHER-A0B380", depending on which address is available)
 */
export function getAircraftTitle(
	aircraft: Pick<
		Aircraft,
		| 'registration'
		| 'aircraftModel'
		| 'icaoAddress'
		| 'flarmAddress'
		| 'ognAddress'
		| 'otherAddress'
		| 'aircraftCategory'
	>
): string {
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

	// If aircraft category is available (but no registration/model), show type with hex code
	if (
		aircraft.aircraftCategory &&
		aircraft.aircraftCategory.trim() !== '' &&
		aircraft.aircraftCategory !== 'Unknown'
	) {
		const typeName = getAircraftCategoryDescription(aircraft.aircraftCategory);
		const primary = getPrimaryAddress(aircraft);
		const hexCode = primary ? primary.hex : 'Unknown';
		return `${typeName} (${hexCode})`;
	}

	// Default to formatted address
	return formatPrimaryAddress(aircraft);
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
