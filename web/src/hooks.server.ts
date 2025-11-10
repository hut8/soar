// Server-side hooks for static adapter with API proxying
// Sentry error tracking is handled client-side only
import type { Handle, HandleServerError } from '@sveltejs/kit';

// Proxy /data/* requests to the Rust backend
// This is needed for E2E tests and preview mode (vite proxy only works in dev)
export const handle: Handle = async ({ event, resolve }) => {
	// Proxy /data/* requests to Rust backend
	if (event.url.pathname.startsWith('/data/')) {
		const backendUrl = process.env.BACKEND_URL || 'http://localhost:61226';
		const targetUrl = `${backendUrl}${event.url.pathname}${event.url.search}`;

		try {
			const response = await fetch(targetUrl, {
				method: event.request.method,
				headers: {
					...Object.fromEntries(event.request.headers),
					// Forward the original host for proper routing
					'x-forwarded-host': event.request.headers.get('host') || '',
					'x-forwarded-proto': event.url.protocol.replace(':', '')
				},
				body:
					event.request.method !== 'GET' && event.request.method !== 'HEAD'
						? await event.request.text()
						: undefined
			});

			// Create a new Response with the backend's response
			return new Response(response.body, {
				status: response.status,
				statusText: response.statusText,
				headers: response.headers
			});
		} catch (error) {
			console.error(`Failed to proxy request to ${targetUrl}:`, error);
			return new Response(JSON.stringify({ error: 'Backend unavailable' }), {
				status: 503,
				headers: { 'Content-Type': 'application/json' }
			});
		}
	}

	// For all other requests, use the default SvelteKit handling
	return resolve(event);
};

export const handleError: HandleServerError = ({ error }) => {
	console.error('Server error:', error);
	return {
		message: 'Internal error'
	};
};
