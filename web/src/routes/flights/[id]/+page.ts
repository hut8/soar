import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { serverCall } from '$lib/api/server';
import type { Flight, Device, Fix } from '$lib/types';

interface FlightResponse {
	flight: Flight;
	device?: Device;
}

interface FlightFixesResponse {
	fixes: Fix[];
	count: number;
}

export const load: PageLoad = async ({ params, fetch }) => {
	const { id } = params;

	try {
		const [flightResponse, fixesResponse] = await Promise.all([
			serverCall<FlightResponse>(`/flights/${id}`, { fetch }),
			serverCall<FlightFixesResponse>(`/flights/${id}/fixes`, { fetch })
		]);

		return {
			flight: flightResponse.flight,
			device: flightResponse.device,
			fixes: fixesResponse.fixes,
			fixesCount: fixesResponse.count
		};
	} catch (err) {
		console.error('Failed to load flight:', err);
		throw error(404, 'Flight not found');
	}
};
