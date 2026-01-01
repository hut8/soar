import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { serverCall } from '$lib/api/server';
import type { Flight, Aircraft, Fix, DataListResponse, DataResponse } from '$lib/types';

export const load: PageLoad = async ({ params, fetch }) => {
	const { id } = params;

	try {
		// Only await flight data (fast) - let components load aircraft and fixes progressively
		const flightResponse = await serverCall<DataResponse<Flight>>(`/flights/${id}`, { fetch });

		// Return promises for progressive loading - components can await these individually
		const aircraftPromise = flightResponse.data.aircraftId
			? serverCall<DataResponse<Aircraft>>(`/flights/${id}/device`, { fetch }).then(
					(res) => res.data
				)
			: Promise.resolve(undefined);

		const fixesPromise = serverCall<DataListResponse<Fix>>(`/flights/${id}/fixes`, {
			fetch
		}).then((res) => res.data || []);

		return {
			flight: flightResponse.data,
			// Return promises for progressive loading
			aircraftPromise,
			fixesPromise
		};
	} catch (err) {
		console.error('Failed to load flight:', err);
		throw error(404, 'Flight not found');
	}
};
