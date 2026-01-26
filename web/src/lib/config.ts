// Central configuration file for client-side environment variables
import { browser } from '$app/environment';

// Google Maps API Key
// Set via VITE_GOOGLE_MAPS_API_KEY environment variable
// Falls back to hardcoded key if not set in .env
export const GOOGLE_MAPS_API_KEY =
	import.meta.env.VITE_GOOGLE_MAPS_API_KEY || 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';

// Cesium Ion Access Token
// Set via VITE_CESIUM_ION_TOKEN environment variable
// Get a free token from https://cesium.com/ion
// Free tier: 50,000 requests/month for terrain and imagery
export const CESIUM_ION_TOKEN =
	import.meta.env.VITE_CESIUM_ION_TOKEN ||
	'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJqdGkiOiI3NWVkZmM1Yi1kMWIyLTRmY2QtOWZjNi05M2IyODFhNjBiNzkiLCJpZCI6MzY5OTY1LCJpYXQiOjE3NjU4MjM5Mjh9.W6K21mkvgIjRdfWqc8CQRdrz8Kajb5AVtFc3SWJa06I';

// MapTiler API Key
// Set via VITE_MAPTILER_API_KEY environment variable
// Get a free key from https://www.maptiler.com/
// Free tier: 100,000 requests/month
export const MAPTILER_API_KEY = import.meta.env.VITE_MAPTILER_API_KEY || '';

// Environment detection
export function isStaging(): boolean {
	if (!browser) return false;
	return window.location.hostname === 'staging.glider.flights';
}

// Get the appropriate Grafana base URL for the current environment
export function getGrafanaUrl(): string {
	return isStaging() ? 'https://grafana.staging.glider.flights' : 'https://grafana.glider.flights';
}
