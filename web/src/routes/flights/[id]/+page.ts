import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { serverCall } from '$lib/api/server';

export const load: PageLoad = async ({ params }) => {
	const { id } = params;

	try {
		const [flightResponse, fixesResponse] = await Promise.all([
			serverCall<{
				flight: {
					id: string;
					device_id?: string;
					device_address: string;
					device_address_type: string;
					takeoff_time?: string;
					landing_time?: string;
					timed_out_at?: string;
					state: 'active' | 'complete' | 'timed_out';
					departure_airport?: string;
					departure_airport_id?: number;
					arrival_airport?: string;
					arrival_airport_id?: number;
					tow_aircraft_id?: string;
					tow_release_height_msl?: number;
					club_id?: string;
					takeoff_altitude_offset_ft?: number;
					landing_altitude_offset_ft?: number;
					takeoff_runway_ident?: string;
					landing_runway_ident?: string;
					total_distance_meters?: number;
					maximum_displacement_meters?: number;
					runways_inferred?: boolean;
					created_at: string;
					updated_at: string;
				};
				device?: {
					id: string;
					address: number;
					address_type: string;
					registration?: string;
					aircraft_model?: string;
					competition_number?: string;
					tracked: boolean;
					identified: boolean;
					from_ddb: boolean;
					aircraft_type_ogn?: string;
					last_fix_at?: string;
					created_at: string;
					updated_at?: string;
				};
			}>(`/flights/${id}`),
			serverCall<{
				fixes: Array<{
					id: string;
					device_id?: string;
					device_address_hex?: string;
					timestamp: string;
					latitude: number;
					longitude: number;
					altitude_msl_feet?: number;
					altitude_agl_feet?: number;
					track_degrees?: number;
					ground_speed_knots?: number;
					climb_fpm?: number;
					registration?: string;
					model?: string;
					flight_id?: string;
					active: boolean;
					raw_packet: string;
				}>;
				count: number;
			}>(`/flights/${id}/fixes`)
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
