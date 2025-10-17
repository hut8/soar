import { dev } from '$app/environment';
import { loading } from '$lib/stores/loading';

// Detect development mode and set appropriate API base URL
export const API_BASE = dev ? 'http://localhost:1337/data' : '/data';

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
		const response = await fetchFn(`${API_BASE}${endpoint}`, {
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
