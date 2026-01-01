import { browser } from '$app/environment';
import { dev } from '$app/environment';
import { loading } from '$lib/stores/loading';
import { backendMode } from '$lib/stores/backend';
import { auth } from '$lib/stores/auth';
import { toaster } from '$lib/toaster';
import { get } from 'svelte/store';

// Get the API base URL based on environment and backend mode
export function getApiBase(): string {
	if (!dev) {
		// Production build always uses relative path
		return '/data';
	}

	// In development, check the backend mode setting
	const mode = get(backendMode);
	return mode === 'dev' ? 'http://localhost:1337/data' : 'https://staging.glider.flights/data';
}

// Legacy export for compatibility - but prefer using getApiBase()
export const API_BASE = getApiBase();

export class ServerError extends Error {
	constructor(
		message: string,
		public status: number
	) {
		super(message);
		this.name = 'ServerError';
	}
}

export interface ServerCallOptions extends RequestInit {
	params?: Record<string, string | number | boolean | null | undefined>;
	fetch?: typeof fetch;
}

export async function serverCall<T>(endpoint: string, options?: ServerCallOptions): Promise<T> {
	loading.startRequest();

	// Extract custom options
	const { params, fetch: customFetch, ...requestOptions } = options || {};

	// Build query string from params
	let url = `${getApiBase()}${endpoint}`;
	if (params) {
		const searchParams = new URLSearchParams();
		Object.entries(params).forEach(([key, value]) => {
			if (value !== null && value !== undefined) {
				searchParams.append(key, String(value));
			}
		});
		const queryString = searchParams.toString();
		if (queryString) {
			url += (endpoint.includes('?') ? '&' : '?') + queryString;
		}
	}

	// Use provided fetch (from SvelteKit load function) or fall back to global fetch
	const fetchFn = customFetch || fetch;

	// Get auth token from localStorage and add to headers if available
	const headers: Record<string, string> = {
		'Content-Type': 'application/json',
		...(requestOptions.headers as Record<string, string>)
	};

	// Add Authorization header if token is available in browser environment
	if (browser) {
		const token = localStorage.getItem('auth_token');
		if (token) {
			// Only add Authorization header if one wasn't explicitly provided
			if (!headers['Authorization'] && !headers['authorization']) {
				headers['Authorization'] = `Bearer ${token}`;
			}
		}
	}

	try {
		const response = await fetchFn(url, {
			...requestOptions,
			headers
		});

		if (!response.ok) {
			// Handle 401 Unauthorized - invalid/expired token
			if (response.status === 401 && browser) {
				// Clear the token from localStorage
				localStorage.removeItem('auth_token');

				// Log the user out
				auth.logout();

				// Show a toast notification
				toaster.error({
					title: 'Session Expired',
					description: 'Your session has expired. Please log in again.'
				});

				// Throw error with a user-friendly message
				throw new ServerError('Session expired', response.status);
			}

			// Try to parse error as JSON first (standard format: {"errors": "message"})
			const errorText = await response.text();
			let errorMessage = errorText || 'Request failed';

			try {
				const errorData = JSON.parse(errorText);
				// Extract the actual error message from the JSON response
				errorMessage = errorData.errors || errorData.message || errorText;
			} catch {
				// If JSON parsing fails, use the raw text (already set above)
			}

			throw new ServerError(errorMessage, response.status);
		}

		if (response.status === 204) {
			return {} as T;
		}

		const data = await response.json();
		return data as T;
	} finally {
		loading.endRequest();
	}
}
