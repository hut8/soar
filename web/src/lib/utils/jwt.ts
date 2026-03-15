/**
 * Decode a JWT payload without verification and check if the token is expired.
 * Returns true if the token is definitely expired, false if it is not expired
 * or if expiration cannot be determined (e.g. malformed or undecodable token).
 */
export function isTokenExpired(token: string): boolean {
	try {
		const parts = token.split('.');
		// If the token does not have three parts, we cannot determine expiry
		if (parts.length !== 3) return false;

		// JWT payloads are base64url-encoded; normalize to standard base64
		let base64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
		// Add padding if necessary
		const padding = base64.length % 4;
		if (padding === 2) {
			base64 += '==';
		} else if (padding === 3) {
			base64 += '=';
		} else if (padding !== 0) {
			// Invalid base64 length; treat as unknown rather than expired
			return false;
		}

		const payload = JSON.parse(atob(base64));

		if (typeof payload.exp !== 'number') {
			// No usable exp claim; treat as unknown
			return false;
		}

		// Compare with current time (exp is in seconds).
		// Use <= so tokens expiring at the current second are treated as expired.
		return payload.exp <= Date.now() / 1000;
	} catch {
		// On any decode/parse failure, treat expiry as unknown (not definitely expired)
		return false;
	}
}
