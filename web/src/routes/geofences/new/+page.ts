import { redirect } from '@sveltejs/kit';
import { browser } from '$app/environment';
import type { PageLoad } from './$types';

// Disable SSR for this page
export const ssr = false;

export const load: PageLoad = async ({ url }) => {
	if (browser) {
		const token = localStorage.getItem('auth_token');
		if (!token) {
			redirect(307, '/login');
		}
	}

	const airportIdParam = url.searchParams.get('airportId');
	const clubId = url.searchParams.get('clubId');

	let airportId: number | undefined = undefined;
	if (airportIdParam !== null) {
		const parsed = parseInt(airportIdParam, 10);
		if (Number.isFinite(parsed) && parsed > 0) {
			airportId = parsed;
		}
	}

	return {
		airportId,
		clubId: clubId || undefined
	};
};
