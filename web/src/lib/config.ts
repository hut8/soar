// Central configuration file for client-side environment variables

// Google Maps API Key
// Set via VITE_GOOGLE_MAPS_API_KEY environment variable
// Falls back to hardcoded key if not set in .env
export const GOOGLE_MAPS_API_KEY =
	import.meta.env.VITE_GOOGLE_MAPS_API_KEY || 'AIzaSyBaK8UU0l4z-k6b-UPlLzw3wv_Ti71XNy8';
