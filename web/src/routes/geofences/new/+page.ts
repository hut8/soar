import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';
import type { PageLoad } from './$types';

// Disable SSR for this page
export const ssr = false;

export const load: PageLoad = async () => {
	if (browser) {
		const token = localStorage.getItem('auth_token');
		if (!token) {
			redirect(307, '/login');
		}
	}

	return {};
};
