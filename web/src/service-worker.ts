/// <reference types="@sveltejs/kit" />
import { build, files, version } from '$service-worker';

// Create a unique cache name for this deployment
const CACHE = `cache-${version}`;

// Assets to cache on install
const ASSETS = [
	...build, // SvelteKit build files
	...files // Static files in /static
];

// Install event - cache all assets
self.addEventListener('install', (event) => {
	async function addFilesToCache() {
		const cache = await caches.open(CACHE);
		await cache.addAll(ASSETS);
	}

	event.waitUntil(addFilesToCache());
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
	async function deleteOldCaches() {
		for (const key of await caches.keys()) {
			if (key !== CACHE) await caches.delete(key);
		}
	}

	event.waitUntil(deleteOldCaches());
});

// Fetch event - use network-first for HTML, cache-first for hashed assets
self.addEventListener('fetch', (event) => {
	// Ignore non-GET requests
	if (event.request.method !== 'GET') return;

	// Never intercept API calls - let them go directly to the server
	const url = new URL(event.request.url);
	if (url.pathname.startsWith('/data/')) return;

	async function respond() {
		const url = new URL(event.request.url);
		const cache = await caches.open(CACHE);

		// Determine if this is an HTML document or hashed asset
		const isHtml =
			url.pathname === '/' || url.pathname.endsWith('.html') || !url.pathname.includes('.');

		const isHashedAsset =
			url.pathname.includes('/_app/immutable/') ||
			url.pathname.includes('/assets/') ||
			/_[a-f0-9]{8,}\.(js|css)/.test(url.pathname);

		// Network-first for HTML (including index.html and SPA routes)
		// This ensures users get updates when server is reachable
		if (isHtml) {
			try {
				const response = await fetch(event.request);
				if (response.status === 200) {
					cache.put(event.request, response.clone());
				}
				return response;
			} catch {
				const cachedResponse = await cache.match(event.request);
				if (cachedResponse) return cachedResponse;
				return new Response('Offline', {
					status: 503,
					statusText: 'Service Unavailable',
					headers: new Headers({
						'Content-Type': 'text/plain'
					})
				});
			}
		}

		// Cache-first for hashed/immutable assets
		// These assets have content hashes in their names, so they never change
		if (isHashedAsset || ASSETS.includes(url.pathname)) {
			const cachedResponse = await cache.match(url.pathname);
			if (cachedResponse) return cachedResponse;
		}

		// Network-first for everything else
		try {
			const response = await fetch(event.request);

			// Cache successful responses for static assets
			if (response.status === 200 && url.origin === location.origin) {
				cache.put(event.request, response.clone());
			}

			return response;
		} catch {
			// Fall back to cache on network failure
			const cachedResponse = await cache.match(event.request);

			if (cachedResponse) {
				return cachedResponse;
			}

			// Return offline response if no cache available
			return new Response('Offline', {
				status: 503,
				statusText: 'Service Unavailable',
				headers: new Headers({
					'Content-Type': 'text/plain'
				})
			});
		}
	}

	event.respondWith(respond());
});

// Push notification event - show notification when received from server
self.addEventListener('push', (event: PushEvent) => {
	if (!event.data) return;

	let payload;
	try {
		payload = event.data.json();
	} catch {
		return;
	}

	if (!payload.eventType || !payload.flightId) return;

	const aircraftName = payload.aircraftRegistration || payload.aircraftModel || 'Aircraft';
	const title =
		payload.eventType === 'takeoff' ? `${aircraftName} took off` : `${aircraftName} landed`;

	const bodyParts: string[] = [];
	if (payload.airportIdent) bodyParts.push(`at ${payload.airportIdent}`);
	if (payload.clubName) bodyParts.push(payload.clubName);

	const ts = payload.timestamp ? new Date(payload.timestamp).getTime() : Date.now();

	const options: NotificationOptions = {
		body: bodyParts.join(' — ') || undefined,
		icon: '/favicon.png',
		badge: '/favicon.png',
		tag: `flight-${payload.flightId}`,
		data: { url: `/flights/${payload.flightId}` },
		timestamp: Number.isNaN(ts) ? Date.now() : ts
	};

	event.waitUntil(
		(self as unknown as ServiceWorkerGlobalScope).registration.showNotification(title, options)
	);
});

// Handle notification click - open the flight page
self.addEventListener('notificationclick', (event: NotificationEvent) => {
	event.notification.close();
	const url = event.notification.data?.url || '/';

	event.waitUntil(
		(self as unknown as ServiceWorkerGlobalScope).clients
			.matchAll({ type: 'window', includeUncontrolled: true })
			.then((windowClients: readonly WindowClient[]) => {
				for (const client of windowClients) {
					if (client.url.includes(url) && 'focus' in client) {
						return client.focus();
					}
				}
				return (self as unknown as ServiceWorkerGlobalScope).clients.openWindow(url);
			})
	);
});
