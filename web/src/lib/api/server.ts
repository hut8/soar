import { dev } from '$app/environment';
import { loading } from '$lib/stores/loading';

// Development-only flag to force using production backend
// Set to true to use https://glider.flights/data even in dev mode
export const FORCE_PRODUCTION_BACKEND = true;

// Detect development mode and set appropriate API base URL
export const API_BASE =
	dev && !FORCE_PRODUCTION_BACKEND
		? 'http://localhost:1337/data'
		: dev && FORCE_PRODUCTION_BACKEND
			? 'https://glider.flights/data'
			: '/data';

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
	let url = `${API_BASE}${endpoint}`;
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

	try {
		const response = await fetchFn(url, {
			...requestOptions,
			headers: {
				'Content-Type': 'application/json',
				...requestOptions.headers
			}
		});

		if (!response.ok) {
			// Try to parse error as JSON first (standard format: {"errors": "message"})
			const errorText = await response.text();
			try {
				const errorData = JSON.parse(errorText);
				const errorMessage = errorData.errors || errorData.message || errorText;
				throw new ServerError(errorMessage, response.status);
			} catch {
				// If JSON parsing fails, use the raw text
				throw new ServerError(errorText || 'Request failed', response.status);
			}
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
