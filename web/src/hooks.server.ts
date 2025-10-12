// Server-side hooks are minimal for static adapter
// Sentry error tracking is handled client-side only
import type { HandleServerError } from '@sveltejs/kit';

export const handleError: HandleServerError = ({ error }) => {
	console.error('Server error:', error);
	return {
		message: 'Internal error'
	};
};
