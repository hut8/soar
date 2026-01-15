/**
 * FAA Engine Type Codes
 * Numeric codes (0-11) used in FAA aircraft registration database
 * to classify aircraft engine types.
 */

/**
 * Type representing valid FAA engine type codes
 */
export type FaaEngineTypeCode = 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11;

/**
 * FAA engine type labels
 * Maps numeric codes to human-readable labels
 */
export const FAA_ENGINE_TYPE_LABELS: Record<FaaEngineTypeCode, string> = {
	0: 'None',
	1: 'Reciprocating',
	2: 'Turbo-Prop',
	3: 'Turbo-Shaft',
	4: 'Turbo-Jet',
	5: 'Turbo-Fan',
	6: 'Ramjet',
	7: '2 Cycle',
	8: '4 Cycle',
	9: 'Unknown',
	10: 'Electric',
	11: 'Rotary'
};

/**
 * Get the label for an FAA engine type code
 * @param code - Numeric engine type code (0-11)
 * @returns Human-readable label of the engine type, or 'Unknown' if invalid
 */
export function getEngineTypeLabel(code: number | null | undefined): string {
	if (code === null || code === undefined) {
		return 'Unknown';
	}

	if (code in FAA_ENGINE_TYPE_LABELS) {
		return FAA_ENGINE_TYPE_LABELS[code as FaaEngineTypeCode];
	}

	return 'Unknown';
}

/**
 * Check if a code is a valid FAA engine type
 * @param code - Code to validate
 * @returns True if the code is a valid engine type
 */
export function isValidEngineTypeCode(code: number): code is FaaEngineTypeCode {
	return code >= 0 && code <= 11;
}
