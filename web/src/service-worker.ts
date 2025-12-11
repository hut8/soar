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
