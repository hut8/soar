import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { serverCall } from '$lib/api/server';
import type { Flight, Aircraft, Fix, DataListResponse } from '$lib/types';

interface FlightResponse {
	flight: Flight;
}

export const load: PageLoad = async ({ params, fetch }) => {
	const { id } = params;

	try {
		// First fetch flight and fixes in parallel
		const [flightResponse, fixesResponse] = await Promise.all([
			serverCall<FlightResponse>(`/flights/${id}`, { fetch }),
			serverCall<DataListResponse<Fix>>(`/flights/${id}/fixes`, { fetch })
		]);

		// Then fetch aircraft separately if the flight has one
		const aircraft = flightResponse.flight.aircraftId
			? await serverCall<Aircraft>(`/flights/${id}/device`, { fetch }).catch(() => undefined)
			: undefined;

		const fixes = fixesResponse.data || [];

		return {
			flight: flightResponse.flight,
			aircraft,
			fixes,
			fixesCount: fixes.length
		};
	} catch (err) {
		console.error('Failed to load flight:', err);
		throw error(404, 'Flight not found');
	}
};
