import * as Sentry from '@sentry/sveltekit';
import { replayIntegration } from '@sentry/sveltekit';
import type { HandleClientError } from '@sveltejs/kit';

Sentry.init({
	dsn: 'https://5d2b053d9c52b539568f9bb038cfae06@o4510021799706624.ingest.us.sentry.io/4510173675520000',
	environment: import.meta.env.MODE || 'production',

	// Adjust trace sample rate for production
	tracesSampleRate: import.meta.env.MODE === 'development' ? 1.0 : 0.1,

	// Session replay settings
	replaysSessionSampleRate: 0.1,
	replaysOnErrorSampleRate: 1.0,

	integrations: [replayIntegration()]
});

export const handleError: HandleClientError = ({ error, event }) => {
	Sentry.captureException(error, { contexts: { sveltekit: { event } } });

	return {
		message: 'An error occurred'
	};
};

// Register service worker for PWA support
if ('serviceWorker' in navigator && import.meta.env.PROD) {
	navigator.serviceWorker
		.register('/service-worker.js')
		.then((registration) => {
			console.log('Service Worker registered with scope:', registration.scope);
		})
		.catch((error) => {
			console.error('Service Worker registration failed:', error);
		});
}
