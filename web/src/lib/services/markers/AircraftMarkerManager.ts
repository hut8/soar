/**
 * AircraftMarkerManager - Manages aircraft markers and trails on a Google Map
 *
 * Handles creating, updating, and clearing aircraft markers with automatic
 * scaling based on zoom level. Also manages position trails for aircraft.
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

interface TrailData {
	polylines: google.maps.Polyline[];
	dots: google.maps.Circle[];
}

export class AircraftMarkerManager {
	private map: google.maps.Map | null = null;
	private markers = new SvelteMap<string, google.maps.marker.AdvancedMarkerElement>();
	private latestFixes = new SvelteMap<string, Fix>();
	private trails = new SvelteMap<string, TrailData>();
	private options: AircraftMarkerManagerOptions;
	private positionFixWindow: number = 8; // hours

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
	 * Get the latest fixes map (for external access)
	 */
	getLatestFixes(): SvelteMap<string, Fix> {
		return this.latestFixes;
	}

	/**
	 * Set the position fix window for trails (in hours)
	 */
	setPositionFixWindow(hours: number): void {
		this.positionFixWindow = hours;
	}

	/**
	 * Update or create an aircraft marker from aircraft data
	 */
	updateMarkerFromAircraft(aircraft: Aircraft): void {
		if (!this.map) return;

		// Use currentFix if available (it's a full Fix object stored as JSONB)
		if (aircraft.currentFix) {
			const currentFix = aircraft.currentFix as unknown as Fix;
			this.updateMarkerFromDevice(aircraft, currentFix);
		} else {
			// Fallback to using fixes array if present
			const fixes = aircraft.fixes || [];
			const latestFix = fixes.length > 0 ? fixes[0] : null;
			if (latestFix) {
				this.updateMarkerFromDevice(aircraft, latestFix);
			} else {
				logger.debug('[MARKER] No position data available for aircraft: {id}', {
					id: aircraft.id
				});
			}
		}
	}

	/**
	 * Update or create an aircraft marker from device and fix data
	 */
	updateMarkerFromDevice(aircraft: Aircraft, latestFix: Fix): void {
		logger.debug('[MARKER] updateMarkerFromDevice called: {params}', {
			params: {
				deviceId: aircraft.id,
				registration: aircraft.registration,
				latestFix: {
					lat: latestFix.latitude,
					lng: latestFix.longitude,
					alt: latestFix.altitudeMslFeet,
					timestamp: latestFix.timestamp
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
			logger.warn('[MARKER] No device ID available');
			return;
		}

		// Update latest fix for this device
		this.latestFixes.set(aircraftKey, latestFix);
		logger.debug('[MARKER] Updated latest fix for aircraft: {key}', { key: aircraftKey });

		// Get or create marker for this aircraft
		let marker = this.markers.get(aircraftKey);

		if (!marker) {
			logger.debug('[MARKER] Creating new marker for aircraft: {key}', { key: aircraftKey });
			// Create new aircraft marker with device info
			marker = this.createMarker(aircraft, latestFix);
			this.markers.set(aircraftKey, marker);
			logger.debug('[MARKER] New marker created and stored. Total markers: {count}', {
				count: this.markers.size
			});
		} else {
			logger.debug('[MARKER] Updating existing marker for aircraft: {key}', {
				key: aircraftKey
			});
			// Update existing marker position and info
			this.updateMarkerPosition(marker, aircraft, latestFix);
		}

		// Update trail for this aircraft
		this.updateTrail(aircraft);
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
	 * Update all aircraft trails (e.g., when position fix window changes)
	 */
	updateAllTrails(activeDevices: Aircraft[]): void {
		activeDevices.forEach((device) => {
			this.updateTrail(device);
		});
	}

	/**
	 * Clear all aircraft markers and trails
	 */
	clear(): void {
		logger.debug('[MARKER] Clearing all aircraft markers. Count: {count}', {
			count: this.markers.size
		});
		this.markers.forEach((marker) => {
			marker.map = null;
		});
		this.markers.clear();
		this.latestFixes.clear();
		this.clearAllTrails();
		logger.debug('[MARKER] All aircraft markers and trails cleared');
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
				deviceId: aircraft.id,
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

		// Use proper device registration, fallback to address
		const tailNumber = aircraft.registration || aircraft.address || 'Unknown';
		const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
		// Use aircraftModel string from device, or detailed model name if available
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
				deviceId: aircraft.id,
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
				// Use proper device registration, fallback to address
				const tailNumber = aircraft.registration || aircraft.address || 'Unknown';
				const { altitudeText, isOld } = formatAltitudeWithTime(fix.altitudeMslFeet, fix.timestamp);
				// Use aircraftModel string from device
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

		// Apply transform to the entire marker content
		markerContent.style.transform = `scale(${scale})`;
		markerContent.style.transformOrigin = 'center bottom'; // Anchor at bottom center
	}

	/**
	 * Update trail for an aircraft
	 */
	private updateTrail(aircraft: Aircraft): void {
		if (!this.map || this.positionFixWindow === 0) {
			// Remove trail if disabled
			this.clearTrailForAircraft(aircraft.id);
			return;
		}

		const fixes = aircraft.fixes || []; // Get all fixes from device

		// Filter fixes to those within the position fix window
		const cutoffTime = dayjs().subtract(this.positionFixWindow, 'hour');
		const trailFixes = fixes.filter((fix) => dayjs(fix.timestamp).isAfter(cutoffTime));

		if (trailFixes.length < 2) {
			// Need at least 2 points to draw a trail
			this.clearTrailForAircraft(aircraft.id);
			return;
		}

		// Clear existing trail
		this.clearTrailForAircraft(aircraft.id);

		// Create polyline segments with progressive transparency
		const polylines: google.maps.Polyline[] = [];
		for (let i = 0; i < trailFixes.length - 1; i++) {
			// Calculate opacity: newest segment (i=0) = 0.7, oldest = 0.2
			const segmentOpacity = 0.7 - (i / (trailFixes.length - 2)) * 0.5;

			// Use color based on active status and altitude from the newer fix in the segment
			const segmentColor = getMarkerColor(trailFixes[i].active, trailFixes[i].altitudeMslFeet);

			const segment = new google.maps.Polyline({
				path: [
					{ lat: trailFixes[i].latitude, lng: trailFixes[i].longitude },
					{ lat: trailFixes[i + 1].latitude, lng: trailFixes[i + 1].longitude }
				],
				geodesic: true,
				strokeColor: segmentColor,
				strokeOpacity: segmentOpacity,
				strokeWeight: 2,
				map: this.map
			});

			polylines.push(segment);
		}

		// Create dots at each fix position
		const dots: google.maps.Circle[] = [];
		trailFixes.forEach((fix, index) => {
			// Calculate opacity: newest (index 0) = 0.7, oldest = 0.2
			const opacity = 0.7 - (index / (trailFixes.length - 1)) * 0.5;

			// Use color based on active status and altitude for each dot
			const dotColor = getMarkerColor(fix.active, fix.altitudeMslFeet);

			const dot = new google.maps.Circle({
				center: { lat: fix.latitude, lng: fix.longitude },
				radius: 10, // 10 meters radius
				strokeColor: dotColor,
				strokeOpacity: opacity,
				strokeWeight: 1,
				fillColor: dotColor,
				fillOpacity: opacity * 0.5,
				map: this.map
			});

			dots.push(dot);
		});

		// Store trail data
		this.trails.set(aircraft.id, { polylines, dots });
	}

	/**
	 * Clear trail for a specific aircraft
	 */
	private clearTrailForAircraft(aircraftId: string): void {
		const trail = this.trails.get(aircraftId);
		if (trail) {
			trail.polylines.forEach((polyline) => polyline.setMap(null));
			trail.dots.forEach((dot) => dot.setMap(null));
			this.trails.delete(aircraftId);
		}
	}

	/**
	 * Clear all trails
	 */
	private clearAllTrails(): void {
		this.trails.forEach((trail) => {
			trail.polylines.forEach((polyline) => polyline.setMap(null));
			trail.dots.forEach((dot) => dot.setMap(null));
		});
		this.trails.clear();
	}
}
