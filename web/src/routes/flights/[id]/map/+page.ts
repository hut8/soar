import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { serverCall } from '$lib/api/server';
import type { Flight, Aircraft, Fix, PathPoint, DataListResponse } from '$lib/types';
import { getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'FlightMapLoader']);

interface FlightResponse {
	flight: Flight;
}

export const load: PageLoad = async ({ params, fetch }) => {
	const { id } = params;

	try {
		// Fetch flight, compressed path (for trail), and fixes (for chart/details) in parallel
		const [flightResponse, pathResponse, fixesResponse] = await Promise.all([
			serverCall<FlightResponse>(`/flights/${id}`, { fetch }),
			serverCall<DataListResponse<PathPoint>>(`/flights/${id}/path?epsilon=50`, { fetch }),
			serverCall<DataListResponse<Fix>>(`/flights/${id}/fixes`, { fetch })
		]);

		// Then fetch device separately if the flight has one
		const device = flightResponse.flight.aircraftId
			? await serverCall<Aircraft>(`/flights/${id}/device`, { fetch }).catch(() => undefined)
			: undefined;

		const path = pathResponse.data || [];
		const fixes = fixesResponse.data || [];

		return {
			flight: flightResponse.flight,
			device,
			path, // Compressed path for trail rendering
			fixes, // Full fixes for chart and info windows
			fixesCount: fixes.length
		};
	} catch (err) {
		logger.error('Failed to load flight: {error}', { error: err });
		throw error(404, 'Flight not found');
	}
};
