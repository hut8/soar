import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { serverCall } from '$lib/api/server';
import type { Flight, Aircraft, Fix } from '$lib/types';

interface FlightResponse {
	flight: Flight;
}

interface FlightFixesResponse {
	fixes: Fix[];
	count: number;
}

export const load: PageLoad = async ({ params, fetch }) => {
	const { id } = params;

	try {
		// First fetch flight and fixes in parallel
		const [flightResponse, fixesResponse] = await Promise.all([
			serverCall<FlightResponse>(`/flights/${id}`, { fetch }),
			serverCall<FlightFixesResponse>(`/flights/${id}/fixes`, { fetch })
		]);

		// Then fetch device separately if the flight has one
		const device = flightResponse.flight.aircraft_id
			? await serverCall<Aircraft>(`/flights/${id}/device`, { fetch }).catch(() => undefined)
			: undefined;

		return {
			flight: flightResponse.flight,
			device,
			fixes: fixesResponse.fixes,
			fixesCount: fixesResponse.count
		};
	} catch (err) {
		console.error('Failed to load flight:', err);
		throw error(404, 'Flight not found');
	}
};
