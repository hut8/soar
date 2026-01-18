/**
 * AircraftMarkerManager - Manages aircraft markers on a Google Map
 *
 * Handles creating, updating, and clearing aircraft markers with automatic
 * scaling based on zoom level. Stores only the current fix per aircraft.
 */

import { SvelteMap } from 'svelte/reactivity';
import { getLogger } from '$lib/logging';
import { getMarkerColor, formatAltitudeWithTime } from '$lib/utils/mapColors';
import type { Aircraft, Fix } from '$lib/types';
import dayjs from 'dayjs';

const logger = getLogger(['soar', 'AircraftMarkerManager']);

/** Z-index for aircraft markers (above airports and receivers) */
const AIRCRAFT_MARKER_Z_INDEX = 1000;

/** Z-index for aircraft markers when hovered */
const AIRCRAFT_MARKER_HOVER_Z_INDEX = 10000;

export interface AircraftMarkerManagerOptions {
	/** Callback when an aircraft marker is clicked */
	onAircraftClick?: (aircraft: Aircraft) => void;
}

export class AircraftMarkerManager {
	private map: google.maps.Map | null = null;
	private markers = new SvelteMap<string, google.maps.marker.AdvancedMarkerElement>();
	private currentFixes = new SvelteMap<string, Fix>();
	private options: AircraftMarkerManagerOptions;

	constructor(options: AircraftMarkerManagerOptions = {}) {
		this.options = options;
	}

	/**
	 * Set the map instance for this manager
	 */
	setMap(map: google.maps.Map): void {
		this.map = map;
	}

	/**
	 * Get the current markers map (for external access)
	 */
	getMarkers(): SvelteMap<string, google.maps.marker.AdvancedMarkerElement> {
		return this.markers;
	}

	/**
	 * Get the current fixes map (for external access)
	 */
	getCurrentFixes(): SvelteMap<string, Fix> {
		return this.currentFixes;
	}

	/**
	 * Update or create an aircraft marker from aircraft data
	 */
	updateMarkerFromAircraft(aircraft: Aircraft): void {
		if (!this.map) return;

		// Use currentFix if available (it's a full Fix object stored as JSONB)
		if (aircraft.currentFix) {
			const fix = aircraft.currentFix as unknown as Fix;
			this.updateMarker(aircraft, fix);
		} else {
			logger.debug('[MARKER] No position data available for aircraft: {id}', {
				id: aircraft.id
			});
		}
	}

	/**
	 * Update or create an aircraft marker from aircraft and fix data
	 */
	updateMarker(aircraft: Aircraft, fix: Fix): void {
		logger.debug('[MARKER] updateMarker called: {params}', {
			params: {
				aircraftId: aircraft.id,
				registration: aircraft.registration,
				fix: {
					lat: fix.latitude,
					lng: fix.longitude,
					alt: fix.altitudeMslFeet,
					timestamp: fix.timestamp
				},
				mapExists: !!this.map
			}
		});

		if (!this.map) {
			logger.warn('[MARKER] No map available for marker update');
			return;
		}

		const aircraftKey = aircraft.id;
		if (!aircraftKey) {
			logger.warn('[MARKER] No aircraft ID available');
			return;
		}

		// Update current fix for this aircraft
		this.currentFixes.set(aircraftKey, fix);
		logger.debug('[MARKER] Updated current fix for aircraft: {key}', { key: aircraftKey });

		// Get or create marker for this aircraft
		let marker = this.markers.get(aircraftKey);

		if (!marker) {
			logger.debug('[MARKER] Creating new marker for aircraft: {key}', { key: aircraftKey });
			// Create new aircraft marker
			marker = this.createMarker(aircraft, fix);
			this.markers.set(aircraftKey, marker);
			logger.debug('[MARKER] New marker created and stored. Total markers: {count}', {
				count: this.markers.size
			});
		} else {
			logger.debug('[MARKER] Updating existing marker for aircraft: {key}', {
				key: aircraftKey
			});
			// Update existing marker position and info
			this.updateMarkerPosition(marker, aircraft, fix);
		}
	}

	/**
	 * Update marker scaling for all aircraft based on zoom level
	 */
	updateAllMarkersScale(): void {
		if (!this.map) return;

		const currentZoom = this.map.getZoom() || 4;
		this.markers.forEach((marker) => {
			const markerContent = marker.content as HTMLElement;
			if (markerContent) {
				this.updateMarkerScale(markerContent, currentZoom);
			}
		});
	}

	/**
	 * Clear all aircraft markers
	 */
	clear(): void {
		logger.debug('[MARKER] Clearing all aircraft markers. Count: {count}', {
			count: this.markers.size
		});
		this.markers.forEach((marker) => {
			marker.map = null;
		});
		this.markers.clear();
		this.currentFixes.clear();
		logger.debug('[MARKER] All aircraft markers cleared');
	}

	/**
	 * Dispose of the manager and clean up resources
	 */
	dispose(): void {
		this.clear();
		this.map = null;
	}

	/**
	 * Create a new aircraft marker
	 */
	private createMarker(aircraft: Aircraft, fix: Fix): google.maps.marker.AdvancedMarkerElement {
		logger.debug('[MARKER] Creating marker for aircraft: {params}', {
			params: {
				aircraftId: aircraft.id,
				registration: aircraft.registration,
				address: aircraft.address,
				position: { lat: fix.latitude, lng: fix.longitude },
				track: fix.trackDegrees
			}
		});

		// Create aircraft icon with rotation based on track
		const markerContent = document.createElement('div');
		markerContent.className = 'aircraft-marker';

		// Aircraft icon (rotated based on track) - using a more visible SVG plane
		const aircraftIcon = document.createElement('div');
		aircraftIcon.className = 'aircraft-icon';

		// Calculate color based on active status and altitude (used for label)
		const markerColor = getMarkerColor(fix.active, fix.altitudeMslFeet);

		// Create SVG airplane icon that's more visible and oriented correctly
		aircraftIcon.innerHTML = `
			<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
				<path d="M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z"/>
			</svg>
		`;

		// Rotate icon based on track degrees (default to 0 if not available)
		const track = fix.trackDegrees || 0;
		aircraftIcon.style.transform = `rotate(${track}deg)`;
		logger.debug('[MARKER] Set icon rotation to: {track} degrees', { track });

		// Info label below the icon - show proper aircraft information
		const infoLabel = document.createElement('div');
		infoLabel.className = 'aircraft-label';
		infoLabel.style.background = markerColor.replace('rgb', 'rgba').replace(')', ', 0.75)');
		infoLabel.style.borderColor = markerColor;

		// Use proper aircraft registration, fallback to address
		const tailNumber = aircraft.registration || aircraft.address || 'Unknown';
		const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
		// Use aircraftModel string from aircraft
		const aircraftModel = aircraft.aircraftModel || null;

		logger.debug('[MARKER] Aircraft info: {info}', {
			info: {
				tailNumber,
				altitude: altitudeText,
				model: aircraftModel,
				isOld
			}
		});

		// Create label with tail number + model (if available) on top, altitude on bottom
		const tailDiv = document.createElement('div');
		tailDiv.className = 'aircraft-tail';
		// Include aircraft model after tail number if available
		tailDiv.textContent = aircraftModel ? `${tailNumber} (${aircraftModel})` : tailNumber;

		const altDiv = document.createElement('div');
		altDiv.className = 'aircraft-altitude';
		altDiv.textContent = altitudeText;

		// Apply transparency if fix is old (>5 minutes)
		if (isOld) {
			aircraftIcon.style.opacity = '0.5';
			tailDiv.style.opacity = '0.5';
			altDiv.style.opacity = '0.5';
		}

		infoLabel.appendChild(tailDiv);
		infoLabel.appendChild(altDiv);

		markerContent.appendChild(aircraftIcon);
		markerContent.appendChild(infoLabel);

		// Create the marker with proper title including aircraft model and full timestamp
		const fullTimestamp = dayjs(fix.timestamp).format('YYYY-MM-DD HH:mm:ss UTC');
		const title = aircraft.aircraftModel
			? `${tailNumber} (${aircraft.aircraftModel}) - ${altitudeText} - Last seen: ${fullTimestamp}`
			: `${tailNumber} - ${altitudeText} - Last seen: ${fullTimestamp}`;

		logger.debug('[MARKER] Creating AdvancedMarkerElement with: {params}', {
			params: {
				position: { lat: fix.latitude, lng: fix.longitude },
				title,
				hasContent: !!markerContent
			}
		});

		const marker = new google.maps.marker.AdvancedMarkerElement({
			position: { lat: fix.latitude, lng: fix.longitude },
			map: this.map,
			title: title,
			content: markerContent,
			zIndex: AIRCRAFT_MARKER_Z_INDEX
		});

		// Add click event listener to open aircraft status modal
		if (this.options.onAircraftClick) {
			const clickHandler = this.options.onAircraftClick;
			marker.addListener('click', () => {
				clickHandler(aircraft);
			});
		}

		// Add hover listeners to bring marker to front when overlapping with other aircraft
		markerContent.addEventListener('mouseenter', () => {
			marker.zIndex = AIRCRAFT_MARKER_HOVER_Z_INDEX;
		});

		markerContent.addEventListener('mouseleave', () => {
			marker.zIndex = AIRCRAFT_MARKER_Z_INDEX;
		});

		// Apply initial zoom-based scaling
		this.updateMarkerScale(markerContent, this.map!.getZoom() || 4);

		logger.debug('[MARKER] AdvancedMarkerElement created successfully');
		return marker;
	}

	/**
	 * Update an existing marker's position and info
	 */
	private updateMarkerPosition(
		marker: google.maps.marker.AdvancedMarkerElement,
		aircraft: Aircraft,
		fix: Fix
	): void {
		logger.debug('[MARKER] Updating existing marker position: {params}', {
			params: {
				aircraftId: aircraft.id,
				oldPosition: marker.position,
				newPosition: { lat: fix.latitude, lng: fix.longitude }
			}
		});

		// Update position
		marker.position = { lat: fix.latitude, lng: fix.longitude };

		// Update icon rotation and label
		const markerContent = marker.content as HTMLElement;
		if (markerContent) {
			const aircraftIcon = markerContent.querySelector('.aircraft-icon') as HTMLElement;
			const infoLabel = markerContent.querySelector('.aircraft-label') as HTMLElement;
			const tailDiv = markerContent.querySelector('.aircraft-tail') as HTMLElement;
			const altDiv = markerContent.querySelector('.aircraft-altitude') as HTMLElement;

			// Calculate color based on active status and altitude
			const markerColor = getMarkerColor(fix.active, fix.altitudeMslFeet);

			if (aircraftIcon) {
				const track = fix.trackDegrees || 0;
				aircraftIcon.style.transform = `rotate(${track}deg)`;
				logger.debug('[MARKER] Updated icon rotation to: {track} degrees', { track });
			}

			if (infoLabel) {
				infoLabel.style.background = markerColor.replace('rgb', 'rgba').replace(')', ', 0.75)');
				infoLabel.style.borderColor = markerColor;
			}

			if (tailDiv && altDiv) {
				// Use proper aircraft registration, fallback to address
				const tailNumber = aircraft.registration || aircraft.address || 'Unknown';
				const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
				// Use aircraftModel string from aircraft
				const aircraftModel = aircraft.aircraftModel || null;

				// Include aircraft model after tail number if available
				tailDiv.textContent = aircraftModel ? `${tailNumber} (${aircraftModel})` : tailNumber;
				altDiv.textContent = altitudeText;

				// Apply transparency if fix is old (>5 minutes)
				if (isOld) {
					aircraftIcon.style.opacity = '0.5';
					tailDiv.style.opacity = '0.5';
					altDiv.style.opacity = '0.5';
				} else {
					// Reset opacity for fresh fixes
					aircraftIcon.style.opacity = '1';
					tailDiv.style.opacity = '1';
					altDiv.style.opacity = '1';
				}

				logger.debug('[MARKER] Updated label info: {info}', {
					info: {
						tailNumber,
						altitudeText,
						aircraftModel,
						isOld
					}
				});
			}
		} else {
			logger.warn('[MARKER] No marker content found for position update');
		}

		// Update scaling for the current zoom level
		const currentZoom = this.map!.getZoom() || 4;
		this.updateMarkerScale(markerContent, currentZoom);

		// Update the marker title with full timestamp
		const { altitudeText } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
		const fullTimestamp = dayjs(fix.timestamp).format('YYYY-MM-DD HH:mm:ss UTC');
		const title = aircraft.aircraftModel
			? `${aircraft.registration || aircraft.address} (${aircraft.aircraftModel}) - ${altitudeText} - Last seen: ${fullTimestamp}`
			: `${aircraft.registration || aircraft.address} - ${altitudeText} - Last seen: ${fullTimestamp}`;

		marker.title = title;
		logger.debug('[MARKER] Updated marker title: {title}', { title });
	}

	/**
	 * Update marker scale based on zoom level
	 */
	private updateMarkerScale(markerContent: HTMLElement, zoom: number): void {
		if (!markerContent) return;

		// Calculate scale based on zoom level
		// Zoom levels typically range from 1 (world) to 20+ (street level)
		// Keep markers small even when zoomed in to avoid clutter
		let scale: number;

		if (zoom <= 4) {
			// Very zoomed out (world/continental view) - minimum size
			scale = 0.3;
		} else if (zoom <= 8) {
			// Country/state level - small size
			scale = 0.4 + (zoom - 4) * 0.1; // 0.4 to 0.8
		} else if (zoom <= 12) {
			// Regional level - keep compact
			scale = 0.8 + (zoom - 8) * 0.025; // 0.8 to 0.9
		} else {
			// City/street level - maximum but still compact
			scale = 0.9 + Math.min(zoom - 12, 6) * 0.0167; // 0.9 to 1.0 max
		}

		// Apply transform to the entire marker content.
		// The AdvancedMarkerElement positions markers with the bottom-center of the content
		// at the geographic point. To align the aircraft ICON center with the fix point,
		// we translate down by the distance from icon center to marker bottom (~53px).
		// Using scale() first means translateY is in scaled coordinates, which automatically
		// adjusts the offset proportionally at different zoom levels.
		// Transform origin 'center top' ensures scaling keeps the icon horizontally centered.
		markerContent.style.transform = `scale(${scale}) translateY(53px)`;
		markerContent.style.transformOrigin = 'center top';
	}
}
