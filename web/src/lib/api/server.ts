import { dev } from '$app/environment';
import { loading } from '$lib/stores/loading';

// Development-only flag to force using production backend
// Set to true to use https://glider.flights/data even in dev mode
const FORCE_PRODUCTION_BACKEND = true;

// Detect development mode and set appropriate API base URL
export const API_BASE = dev && !FORCE_PRODUCTION_BACKEND ? 'http://localhost:1337/data' : '/data';

export class ServerError extends Error {
	constructor(
		message: string,
		public status: number
	) {
		super(message);
		this.name = 'ServerError';
	}
}

type FromJSON<T> = { fromJSON(data: unknown): T };

export async function serverCall<T>(
	endpoint: string,
	options: RequestInit = {},
	cls?: FromJSON<T>,
	customFetch?: typeof fetch
): Promise<T> {
	loading.startRequest();

	// Use provided fetch (from SvelteKit load function) or fall back to global fetch
	const fetchFn = customFetch || fetch;

	try {
		// Properly join API_BASE and endpoint, avoiding duplicate slashes
		const base = API_BASE.endsWith('/') ? API_BASE.slice(0, -1) : API_BASE;
		const path = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
		const url = `${base}${path}`;

		const response = await fetchFn(url, {
			...options,
			headers: {
				'Content-Type': 'application/json',
				...options.headers
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

		if (!cls) return data as T;

		// handle arrays or single objects
		if (Array.isArray(data)) {
			return data.map((item) => cls.fromJSON(item)) as T;
		}

		return cls.fromJSON(data);
	} finally {
		loading.endRequest();
	}
}
