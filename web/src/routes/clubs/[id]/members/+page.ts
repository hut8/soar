import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';
import type { PageLoad } from './$types';

// Disable SSR for this page to ensure client-side authentication check works
export const ssr = false;

export const load: PageLoad = async () => {
	if (browser) {
		// Check if user is authenticated
		const token = localStorage.getItem('auth_token');
		if (!token) {
			// Redirect to login if not authenticated
			redirect(307, '/login');
		}
	}

	return {};
};
