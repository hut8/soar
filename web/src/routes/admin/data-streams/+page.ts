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
			redirect(307, '/login');
		}

		// Check if user is admin
		try {
			const userStr = localStorage.getItem('auth_user');
			if (userStr) {
				const user = JSON.parse(userStr);
				if (!user.isAdmin) {
					redirect(307, '/');
				}
			} else {
				redirect(307, '/login');
			}
		} catch {
			redirect(307, '/login');
		}
	}

	return {};
};
