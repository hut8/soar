/**
 * ReceiverMarkerManager - Manages receiver (radio station) markers on a Google Map
 *
 * Handles fetching, displaying, and clearing receiver markers with automatic
 * visibility based on zoom level and viewport area.
 */

import { serverCall } from '$lib/api/server';
import { getLogger } from '$lib/logging';
import type { Receiver, DataListResponse } from '$lib/types';

const logger = getLogger(['soar', 'ReceiverMarkerManager']);

/** Maximum viewport area (in square miles) at which receivers are displayed */
const MAX_VIEWPORT_AREA_FOR_RECEIVERS = 10000;

/** Z-index for receiver markers (between airports at 100 and aircraft at 1000) */
const RECEIVER_MARKER_Z_INDEX = 150;

export class ReceiverMarkerManager {
	private map: google.maps.Map | null = null;
	private receivers: Receiver[] = [];
	private markers: google.maps.marker.AdvancedMarkerElement[] = [];
	private shouldShow: boolean = false;
	private debounceTimer: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: google.maps.Map): void {
		this.map = map;
	}

	/**
	 * Check viewport and update receiver visibility/markers
	 * @param viewportArea - Current viewport area in square miles
	 * @param settingEnabled - Whether receiver markers are enabled in settings
	 */
	checkAndUpdate(viewportArea: number, settingEnabled: boolean): void {
		// Clear any existing debounce timer
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
		}

		// Debounce updates by 100ms to prevent excessive API calls
		this.debounceTimer = setTimeout(() => {
			const shouldShow = viewportArea < MAX_VIEWPORT_AREA_FOR_RECEIVERS && settingEnabled;

			if (shouldShow !== this.shouldShow) {
				this.shouldShow = shouldShow;

				if (this.shouldShow) {
					this.fetchAndDisplay();
				} else {
					this.clear();
				}
			} else if (this.shouldShow) {
				// Still showing receivers, update them for the new viewport
				this.fetchAndDisplay();
			}

			this.debounceTimer = null;
		}, 100);
	}

	/**
	 * Force clear all markers and reset state
	 */
	clear(): void {
		this.clearMarkers();
		this.receivers = [];
		this.shouldShow = false;
	}

	/**
	 * Dispose of the manager and clean up resources
	 */
	dispose(): void {
		if (this.debounceTimer !== null) {
			clearTimeout(this.debounceTimer);
			this.debounceTimer = null;
		}
		this.clearMarkers();
		this.receivers = [];
		this.map = null;
	}

	/**
	 * Fetch receivers in the current viewport and display them
	 */
	private async fetchAndDisplay(): Promise<void> {
		if (!this.map) return;

		const bounds = this.map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		// Validate bounding box coordinates
		const nwLat = ne.lat();
		const nwLng = sw.lng();
		const seLat = sw.lat();
		const seLng = ne.lng();

		// Ensure northwest latitude is greater than southeast latitude
		if (nwLat <= seLat) {
			logger.warn(
				'Invalid bounding box: northwest latitude must be greater than southeast latitude'
			);
			return;
		}

		// Validate latitude bounds
		if (nwLat > 90 || nwLat < -90 || seLat > 90 || seLat < -90) {
			logger.warn('Invalid latitude values in bounding box');
			return;
		}

		// Validate longitude bounds
		if (nwLng < -180 || nwLng > 180 || seLng < -180 || seLng > 180) {
			logger.warn('Invalid longitude values in bounding box');
			return;
		}

		try {
			const params = new URLSearchParams({
				latitude_min: seLat.toString(),
				latitude_max: nwLat.toString(),
				longitude_min: nwLng.toString(),
				longitude_max: seLng.toString()
			});

			const response = await serverCall<DataListResponse<Receiver>>(`/receivers?${params}`);
			this.receivers = response.data || [];

			this.displayMarkers();
		} catch (error) {
			logger.error('Error fetching receivers: {error}', { error });
		}
	}

	/**
	 * Display markers for all loaded receivers
	 */
	private displayMarkers(): void {
		// Clear existing markers first
		this.clearMarkers();

		if (!this.map) return;

		this.receivers.forEach((receiver) => {
			if (!receiver.latitude || !receiver.longitude) return;

			// Validate coordinates are valid numbers and within expected ranges
			if (
				isNaN(receiver.latitude) ||
				isNaN(receiver.longitude) ||
				receiver.latitude < -90 ||
				receiver.latitude > 90 ||
				receiver.longitude < -180 ||
				receiver.longitude > 180
			) {
				logger.warn('Invalid coordinates for receiver {callsign}: {lat}, {lng}', {
					callsign: receiver.callsign,
					lat: receiver.latitude,
					lng: receiver.longitude
				});
				return;
			}

			// Create marker content with Radio icon and link
			const markerLink = document.createElement('a');
			markerLink.href = `/receivers/${receiver.id}`;
			markerLink.target = '_blank';
			markerLink.rel = 'noopener noreferrer';
			markerLink.className = 'receiver-marker';

			const iconDiv = document.createElement('div');
			iconDiv.className = 'receiver-icon';
			// Create SVG for Radio icon (antenna symbol)
			iconDiv.innerHTML = `
				<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
					<path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/>
					<path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/>
					<circle cx="12" cy="12" r="2"/>
					<path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/>
					<path d="M19.1 4.9C23 8.8 23 15.1 19.1 19"/>
				</svg>
			`;

			const labelDiv = document.createElement('div');
			labelDiv.className = 'receiver-label';
			labelDiv.textContent = receiver.callsign;

			markerLink.appendChild(iconDiv);
			markerLink.appendChild(labelDiv);

			const marker = new google.maps.marker.AdvancedMarkerElement({
				position: { lat: receiver.latitude, lng: receiver.longitude },
				map: this.map,
				title: `${receiver.callsign}${receiver.description ? ` - ${receiver.description}` : ''}`,
				content: markerLink,
				zIndex: RECEIVER_MARKER_Z_INDEX
			});

			this.markers.push(marker);
		});
	}

	/**
	 * Clear all markers from the map
	 */
	private clearMarkers(): void {
		this.markers.forEach((marker) => {
			marker.map = null;
		});
		this.markers = [];
	}
}
