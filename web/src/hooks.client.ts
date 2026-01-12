import * as Sentry from '@sentry/sveltekit';
import { replayIntegration } from '@sentry/sveltekit';
import type { HandleClientError } from '@sveltejs/kit';
import { configureLogging, getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'hooks']);

// Initialize logging
configureLogging();

// Determine if we should enable Sentry
// Only enable on staging (staging.glider.flights) and production (glider.flights)
// Exclude localhost, 127.0.0.1, and test environments
const shouldEnableSentry = () => {
	if (typeof window === 'undefined') return false;
	const hostname = window.location.hostname;

	// Enable on production and staging only
	return hostname === 'glider.flights' || hostname === 'staging.glider.flights';
};

// Only initialize Sentry on staging/production
if (shouldEnableSentry()) {
	const environment = window.location.hostname === 'glider.flights' ? 'production' : 'staging';

	Sentry.init({
		dsn: 'https://5d2b053d9c52b539568f9bb038cfae06@o4510021799706624.ingest.us.sentry.io/4510173675520000',
		environment,

		// Adjust trace sample rate for production
		tracesSampleRate: 0.1,

		// Session replay settings
		replaysSessionSampleRate: 0.1,
		replaysOnErrorSampleRate: 1.0,

		integrations: [replayIntegration()]
	});
}

export const handleError: HandleClientError = ({ error, event }) => {
	// Only send to Sentry on staging/production
	if (shouldEnableSentry()) {
		Sentry.captureException(error, { contexts: { sveltekit: { event } } });
	} else {
		// Log in development/test
		logger.error('Client error: {error}', { error, event });
	}

	return {
		message: 'An error occurred'
	};
};

// Register service worker for PWA support
if ('serviceWorker' in navigator && import.meta.env.PROD) {
	navigator.serviceWorker
		.register('/service-worker.js')
		.then((registration) => {
			logger.info('Service Worker registered with scope: {scope}', { scope: registration.scope });
		})
		.catch((error) => {
			logger.error('Service Worker registration failed: {error}', { error });
		});
}
