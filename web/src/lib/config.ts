// Central configuration file for client-side environment variables
import { browser } from '$app/environment';

// Google Maps API Key
// Set via VITE_GOOGLE_MAPS_API_KEY environment variable
// Falls back to hardcoded key if not set in .env
export const GOOGLE_MAPS_API_KEY =
	import.meta.env.VITE_GOOGLE_MAPS_API_KEY || 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

// Environment detection
export function isStaging(): boolean {
	if (!browser) return false;
	return window.location.hostname === 'staging.glider.flights';
}

// Get the appropriate Grafana base URL for the current environment
export function getGrafanaUrl(): string {
	return isStaging() ? 'https://grafana.staging.glider.flights' : 'https://grafana.glider.flights';
}
