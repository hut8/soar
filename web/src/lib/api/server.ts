import { dev } from '$app/environment';

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
	cls?: FromJSON<T>
): Promise<T> {
	const response = await fetch(`${API_BASE}${endpoint}`, {
		...options,
		headers: {
			'Content-Type': 'application/json',
			...options.headers
		}
	});

	if (!response.ok) {
		const errorText = await response.text();
		throw new ServerError(errorText || 'Request failed', response.status);
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
}
