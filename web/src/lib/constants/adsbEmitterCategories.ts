/**
 * ADS-B Emitter Category Codes
 * Two-digit codes (A0-A7, B0-B7, C0-C7) used to classify aircraft and other emitters
 * as defined by FAA regulations for ADS-B transponders.
 */

/**
 * Type representing valid ADS-B emitter category codes
 */
export type AdsbEmitterCategory =
	| 'A0'
	| 'A1'
	| 'A2'
	| 'A3'
	| 'A4'
	| 'A5'
	| 'A6'
	| 'A7'
	| 'B0'
	| 'B1'
	| 'B2'
	| 'B3'
	| 'B4'
	| 'B5'
	| 'B6'
	| 'B7'
	| 'C0'
	| 'C1'
	| 'C2'
	| 'C3'
	| 'C4'
	| 'C5'
	| 'C6'
	| 'C7';

/**
 * ADS-B emitter category descriptions
 * Maps two-digit category codes to their full descriptions
 */
export const ADSB_EMITTER_CATEGORIES: Record<AdsbEmitterCategory, string> = {
	A0: 'No ADS-B emitter category information. Do not use this emitter category. If no emitter category fits your installation, seek guidance from the FAA as appropriate.',
	A1: 'Light (< 15500 lbs) – Any airplane with a maximum takeoff weight less than 15,500 pounds. This includes very light aircraft (light sport aircraft) that do not meet the requirements of 14 CFR § 103.1.',
	A2: 'Small (15500 to 75000 lbs) – Any airplane with a maximum takeoff weight greater than or equal to 15,500 pounds but less than 75,000 pounds.',
	A3: 'Large (75000 to 300000 lbs) – Any airplane with a maximum takeoff weight greater than or equal to 75,000 pounds but less than 300,000 pounds that does not qualify for the high vortex category.',
	A4: 'High vortex large (aircraft such as B-757) – Any airplane with a maximum takeoff weight greater than or equal to 75,000 pounds but less than 300,000 pounds that has been determined to generate a high wake vortex. Currently, the Boeing 757 is the only example.',
	A5: 'Heavy (> 300000 lbs) – Any airplane with a maximum takeoff weight equal to or above 300,000 pounds.',
	A6: 'High performance (> 5g acceleration and 400 kts) – Any airplane, regardless of weight, which can maneuver in excess of 5 Gs and maintain true airspeed above 400 knots.',
	A7: 'Rotorcraft – Any rotorcraft regardless of weight.',
	B0: 'No ADS-B emitter category information',
	B1: 'Glider / sailplane – Any glider or sailplane regardless of weight.',
	B2: 'Lighter-than-air – Any lighter than air (airship or balloon) regardless of weight.',
	B3: 'Parachutist / skydiver',
	B4: 'Ultralight / hang-glider / paraglider – A vehicle that meets the requirements of 14 CFR § 103.1. Light sport aircraft should not use the ultralight emitter category unless they meet 14 CFR § 103.1.',
	B5: 'Reserved',
	B6: 'Unmanned aerial vehicle – Any unmanned aerial vehicle or unmanned aircraft system regardless of weight.',
	B7: 'Space / trans-atmospheric vehicle',
	C0: 'No ADS-B emitter category information',
	C1: 'Surface vehicle – emergency vehicle',
	C2: 'Surface vehicle – service vehicle',
	C3: 'Point obstacle (includes tethered balloons)',
	C4: 'Cluster obstacle',
	C5: 'Line obstacle',
	C6: 'Reserved',
	C7: 'Reserved'
};

/**
 * Short, human-friendly labels for each emitter category
 * Useful for UI displays where the full description is too verbose
 */
export const ADSB_EMITTER_LABELS: Record<AdsbEmitterCategory, string> = {
	A0: 'Unknown',
	A1: 'Light Aircraft',
	A2: 'Small Aircraft',
	A3: 'Large Aircraft',
	A4: 'High Vortex Large',
	A5: 'Heavy Aircraft',
	A6: 'High Performance',
	A7: 'Rotorcraft',
	B0: 'Unknown',
	B1: 'Glider',
	B2: 'Lighter-than-air',
	B3: 'Parachutist',
	B4: 'Ultralight',
	B5: 'Reserved',
	B6: 'UAV/Drone',
	B7: 'Space Vehicle',
	C0: 'Unknown',
	C1: 'Emergency Vehicle',
	C2: 'Service Vehicle',
	C3: 'Point Obstacle',
	C4: 'Cluster Obstacle',
	C5: 'Line Obstacle',
	C6: 'Reserved',
	C7: 'Reserved'
};

/**
 * Get the description for an ADS-B emitter category code
 * @param code - Two-digit emitter category code (e.g., 'A1', 'B6', 'C2')
 * @returns Full description of the emitter category, or 'Unknown category' if invalid
 */
export function getEmitterCategoryDescription(code: string): string {
	const upperCode = code?.toUpperCase();
	if (upperCode in ADSB_EMITTER_CATEGORIES) {
		return ADSB_EMITTER_CATEGORIES[upperCode as AdsbEmitterCategory];
	}
	return 'Unknown category';
}

/**
 * Get the short label for an ADS-B emitter category code
 * @param code - Two-digit emitter category code (e.g., 'A1', 'B6', 'C2')
 * @returns Short label of the emitter category, or 'Unknown' if invalid
 */
export function getEmitterCategoryLabel(code: string): string {
	const upperCode = code?.toUpperCase();
	if (upperCode in ADSB_EMITTER_LABELS) {
		return ADSB_EMITTER_LABELS[upperCode as AdsbEmitterCategory];
	}
	return 'Unknown';
}

/**
 * Check if a code is a valid ADS-B emitter category
 * @param code - Code to validate
 * @returns True if the code is a valid emitter category
 */
export function isValidEmitterCategory(code: string): code is AdsbEmitterCategory {
	return code?.toUpperCase() in ADSB_EMITTER_CATEGORIES;
}
