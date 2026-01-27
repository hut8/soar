/**
 * Geofence API client
 */

import { serverCall } from './server';
import type {
	GeofenceListResponse,
	GeofenceDetailResponse,
	CreateGeofenceRequest,
	UpdateGeofenceRequest,
	GeofenceSubscriber,
	AircraftGeofence,
	GeofenceExitEventsResponse,
	DataResponse,
	DataListResponse
} from '$lib/types';

/**
 * List geofences for the current user
 */
export async function listGeofences(clubId?: string): Promise<GeofenceListResponse> {
	return serverCall<GeofenceListResponse>('/geofences', {
		params: clubId ? { clubId } : undefined
	});
}

/**
 * Create a new geofence
 */
export async function createGeofence(
	request: CreateGeofenceRequest
): Promise<GeofenceDetailResponse> {
	return serverCall<GeofenceDetailResponse>('/geofences', {
		method: 'POST',
		body: JSON.stringify(request)
	});
}

/**
 * Get geofence details by ID
 */
export async function getGeofence(id: string): Promise<GeofenceDetailResponse> {
	return serverCall<GeofenceDetailResponse>(`/geofences/${id}`);
}

/**
 * Update a geofence
 */
export async function updateGeofence(
	id: string,
	request: UpdateGeofenceRequest
): Promise<GeofenceDetailResponse> {
	return serverCall<GeofenceDetailResponse>(`/geofences/${id}`, {
		method: 'PUT',
		body: JSON.stringify(request)
	});
}

/**
 * Delete a geofence (soft delete)
 */
export async function deleteGeofence(id: string): Promise<void> {
	await serverCall<Record<string, never>>(`/geofences/${id}`, {
		method: 'DELETE'
	});
}

// ==================== Aircraft Links ====================

/**
 * Get aircraft linked to a geofence
 */
export async function getGeofenceAircraft(geofenceId: string): Promise<string[]> {
	const response = await serverCall<DataListResponse<string>>(`/geofences/${geofenceId}/aircraft`);
	return response.data;
}

/**
 * Link an aircraft to a geofence
 */
export async function addGeofenceAircraft(
	geofenceId: string,
	aircraftId: string
): Promise<AircraftGeofence> {
	const response = await serverCall<DataResponse<AircraftGeofence>>(
		`/geofences/${geofenceId}/aircraft`,
		{
			method: 'POST',
			body: JSON.stringify({ aircraftId })
		}
	);
	return response.data;
}

/**
 * Unlink an aircraft from a geofence
 */
export async function removeGeofenceAircraft(
	geofenceId: string,
	aircraftId: string
): Promise<void> {
	await serverCall<Record<string, never>>(`/geofences/${geofenceId}/aircraft/${aircraftId}`, {
		method: 'DELETE'
	});
}

// ==================== Subscribers ====================

/**
 * Get subscribers for a geofence
 */
export async function getGeofenceSubscribers(geofenceId: string): Promise<GeofenceSubscriber[]> {
	const response = await serverCall<DataListResponse<GeofenceSubscriber>>(
		`/geofences/${geofenceId}/subscribers`
	);
	return response.data;
}

/**
 * Subscribe to a geofence
 */
export async function subscribeToGeofence(
	geofenceId: string,
	sendEmail = true
): Promise<GeofenceSubscriber> {
	const response = await serverCall<DataResponse<GeofenceSubscriber>>(
		`/geofences/${geofenceId}/subscribers`,
		{
			method: 'POST',
			body: JSON.stringify({ sendEmail })
		}
	);
	return response.data;
}

/**
 * Unsubscribe from a geofence
 */
export async function unsubscribeFromGeofence(geofenceId: string, userId: string): Promise<void> {
	await serverCall<Record<string, never>>(`/geofences/${geofenceId}/subscribers/${userId}`, {
		method: 'DELETE'
	});
}

// ==================== Exit Events ====================

/**
 * Get exit events for a geofence
 */
export async function getGeofenceEvents(
	geofenceId: string,
	limit?: number
): Promise<GeofenceExitEventsResponse> {
	return serverCall<GeofenceExitEventsResponse>(`/geofences/${geofenceId}/events`, {
		params: limit ? { limit } : undefined
	});
}

/**
 * Get geofence exit events for a flight
 */
export async function getFlightGeofenceEvents(
	flightId: string
): Promise<GeofenceExitEventsResponse> {
	return serverCall<GeofenceExitEventsResponse>(`/flights/${flightId}/geofence-events`);
}
