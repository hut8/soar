/**
 * Cesium entity factory functions for creating aircraft, flight paths, airports, and receivers
 * Provides a consistent interface for creating 3D visualizations on the globe
 */

import {
	Entity,
	Cartesian3,
	Color,
	VerticalOrigin,
	HorizontalOrigin,
	HeightReference,
	type Cartesian2
} from 'cesium';
import type { Aircraft, Fix, Flight, Airport, Receiver } from '$lib/types';
import { altitudeToColor, formatAltitudeWithTime } from '$lib/utils/mapColors';

/**
 * Feet to meters conversion factor
 */
const FEET_TO_METERS = 0.3048;

/**
 * Create SVG icon for aircraft marker (rotated based on heading)
 * @param color - RGB color string
 * @param heading - Aircraft heading in degrees (0 = North)
 * @returns Data URI for SVG icon
 */
export function createAircraftIconSVG(color: string, heading: number = 0): string {
	// Parse RGB color
	const match = color.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/);
	const r = match ? match[1] : '255';
	const g = match ? match[2] : '0';
	const b = match ? match[3] : '0';

	// Create SVG with aircraft icon (simple triangle pointing up)
	// Rotation is applied via transform attribute
	const svg = `
		<svg width="24" height="24" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
			<g transform="rotate(${heading} 12 12)">
				<!-- Aircraft body (triangle) -->
				<path d="M 12 4 L 8 18 L 12 16 L 16 18 Z"
					fill="rgb(${r}, ${g}, ${b})"
					stroke="white"
					stroke-width="1.5"/>
				<!-- Nose dot -->
				<circle cx="12" cy="4" r="2" fill="white"/>
			</g>
		</svg>
	`;

	return `data:image/svg+xml;base64,${btoa(svg)}`;
}

/**
 * Create a Cesium Entity for an aircraft
 * @param aircraft - Aircraft data
 * @param fix - Latest fix for the aircraft
 * @returns Cesium Entity configured as aircraft marker
 */
export function createAircraftEntity(aircraft: Aircraft, fix: Fix): Entity {
	const altitude = fix.altitude_msl_feet || 0;
	const altitudeMeters = altitude * FEET_TO_METERS;

	// Get altitude-based color
	const color = altitudeToColor(altitude);

	// Format altitude with time
	const { altitudeText, isOld } = formatAltitudeWithTime(altitude, fix.timestamp);

	// Create icon URL with aircraft heading
	const iconUrl = createAircraftIconSVG(color, fix.track_degrees || 0);

	return new Entity({
		id: aircraft.id,
		name: aircraft.registration || aircraft.device_address,
		position: Cartesian3.fromDegrees(fix.longitude, fix.latitude, altitudeMeters),
		billboard: {
			image: iconUrl,
			scale: 1.0,
			verticalOrigin: VerticalOrigin.BOTTOM,
			horizontalOrigin: HorizontalOrigin.CENTER,
			heightReference: HeightReference.NONE // Use absolute altitude
		},
		label: {
			text: `${aircraft.registration || aircraft.device_address}\n${altitudeText}`,
			font: '12px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 3,
			pixelOffset: { x: 0, y: -40 } as unknown as Cartesian2,
			heightReference: HeightReference.NONE,
			disableDepthTestDistance: Number.POSITIVE_INFINITY // Always show label
		},
		description: `
			<h3>${aircraft.registration || aircraft.device_address}</h3>
			<p><strong>Model:</strong> ${aircraft.aircraft_model || 'Unknown'}</p>
			<p><strong>Altitude:</strong> ${altitude} ft MSL</p>
			<p><strong>Speed:</strong> ${fix.ground_speed_knots || '---'} kts</p>
			<p><strong>Heading:</strong> ${fix.track_degrees || '---'}°</p>
			<p><strong>Last seen:</strong> ${altitudeText}</p>
		`,
		properties: {
			aircraftId: aircraft.id,
			registration: aircraft.registration,
			fixId: fix.id,
			altitude: altitude,
			timestamp: fix.timestamp,
			isOld: isOld
		}
	});
}

/**
 * Create Cesium polyline entities for a flight path with gradient colors
 * Returns array of entities (one per segment for gradient effect)
 * @param flight - Flight data
 * @param fixes - Array of fixes for the flight
 * @param colorScheme - Color scheme: 'altitude' or 'time'
 * @returns Array of Cesium Entities for flight path segments
 */
export function createFlightPathEntity(
	flight: Flight,
	fixes: Fix[],
	colorScheme: 'altitude' | 'time' = 'altitude'
): Entity {
	if (fixes.length === 0) {
		throw new Error('Cannot create flight path with zero fixes');
	}

	// For simple solid color polyline (average color)
	// Convert fixes to Cartesian3 positions
	const positions = fixes.map((fix) => {
		const altitude = fix.altitude_msl_feet || 0;
		return Cartesian3.fromDegrees(fix.longitude, fix.latitude, altitude * FEET_TO_METERS);
	});

	// Calculate average color for the path
	let pathColor: Color;
	if (colorScheme === 'altitude') {
		const altitudes = fixes.map((f) => f.altitude_msl_feet || 0);
		const avgAltitude = altitudes.reduce((a, b) => a + b, 0) / altitudes.length;
		const minAlt = Math.min(...altitudes);
		const maxAlt = Math.max(...altitudes);
		const colorStr = altitudeToColor(avgAltitude, minAlt, maxAlt);
		pathColor = Color.fromCssColorString(colorStr);
	} else {
		// For time-based, use orange (mid-range color)
		pathColor = Color.fromCssColorString('rgb(199, 98, 147)'); // Mid purple-orange
	}

	return new Entity({
		id: `flight-${flight.id}`,
		name: `Flight ${flight.registration || flight.device_address}`,
		polyline: {
			positions,
			width: 3,
			material: pathColor,
			clampToGround: false // Show actual altitude
		},
		description: `
			<h3>Flight Path</h3>
			<p><strong>Aircraft:</strong> ${flight.registration || flight.device_address}</p>
			<p><strong>Takeoff:</strong> ${flight.takeoff_time ? new Date(flight.takeoff_time).toLocaleString() : 'Unknown'}</p>
			<p><strong>Landing:</strong> ${flight.landing_time ? new Date(flight.landing_time).toLocaleString() : 'In Progress'}</p>
			<p><strong>Duration:</strong> ${flight.duration_seconds ? Math.round(flight.duration_seconds / 60) + ' min' : 'N/A'}</p>
			<p><strong>Max Altitude:</strong> ${Math.max(...fixes.map((f) => f.altitude_msl_feet || 0))} ft</p>
		`,
		properties: {
			flightId: flight.id,
			aircraftId: flight.aircraft_id,
			takeoffTime: flight.takeoff_time,
			landingTime: flight.landing_time,
			colorScheme
		}
	});
}

/**
 * Create a Cesium entity for an airport marker
 * @param airport - Airport data
 * @returns Cesium Entity configured as airport marker
 */
export function createAirportEntity(airport: Airport): Entity {
	const latitude = parseFloat(airport.latitude_deg || '0');
	const longitude = parseFloat(airport.longitude_deg || '0');
	const elevation = (airport.elevation_ft || 0) * FEET_TO_METERS;

	// Create simple airport icon SVG
	const iconSvg = `
		<svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
			<circle cx="10" cy="10" r="8" fill="#10b981" stroke="white" stroke-width="2"/>
			<text x="10" y="14" font-size="10" font-weight="bold" fill="white" text-anchor="middle">A</text>
		</svg>
	`;
	const iconUrl = `data:image/svg+xml;base64,${btoa(iconSvg)}`;

	return new Entity({
		id: `airport-${airport.id}`,
		name: airport.ident,
		position: Cartesian3.fromDegrees(longitude, latitude, elevation),
		billboard: {
			image: iconUrl,
			scale: 0.8,
			verticalOrigin: VerticalOrigin.BOTTOM
		},
		label: {
			text: airport.ident,
			font: '11px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 2,
			pixelOffset: { x: 0, y: -25 } as unknown as Cartesian2,
			disableDepthTestDistance: 50000 // Hide when far away
		},
		description: `
			<h3>${airport.name}</h3>
			<p><strong>Identifier:</strong> ${airport.ident}</p>
			<p><strong>Type:</strong> ${airport.airport_type}</p>
			<p><strong>Elevation:</strong> ${airport.elevation_ft || '---'} ft</p>
			<p><strong>Location:</strong> ${airport.municipality || '---'}, ${airport.iso_country || '---'}</p>
			${airport.icao_code ? `<p><strong>ICAO:</strong> ${airport.icao_code}</p>` : ''}
			${airport.iata_code ? `<p><strong>IATA:</strong> ${airport.iata_code}</p>` : ''}
		`,
		properties: {
			airportId: airport.id,
			ident: airport.ident,
			type: airport.airport_type
		}
	});
}

/**
 * Create a Cesium entity for a receiver marker
 * @param receiver - Receiver data
 * @returns Cesium Entity configured as receiver marker
 */
export function createReceiverEntity(receiver: Receiver): Entity {
	const latitude = receiver.latitude || 0;
	const longitude = receiver.longitude || 0;

	// Create receiver icon SVG (radio tower)
	const iconSvg = `
		<svg width="20" height="20" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
			<circle cx="10" cy="10" r="8" fill="#3b82f6" stroke="white" stroke-width="2"/>
			<text x="10" y="14" font-size="10" font-weight="bold" fill="white" text-anchor="middle">R</text>
		</svg>
	`;
	const iconUrl = `data:image/svg+xml;base64,${btoa(iconSvg)}`;

	return new Entity({
		id: `receiver-${receiver.id}`,
		name: receiver.callsign,
		position: Cartesian3.fromDegrees(longitude, latitude, 0),
		billboard: {
			image: iconUrl,
			scale: 0.7,
			verticalOrigin: VerticalOrigin.BOTTOM
		},
		label: {
			text: receiver.callsign,
			font: '10px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 2,
			pixelOffset: { x: 0, y: -25 } as unknown as Cartesian2,
			disableDepthTestDistance: 50000 // Hide when far away
		},
		description: `
			<h3>${receiver.callsign}</h3>
			<p><strong>Description:</strong> ${receiver.description || '---'}</p>
			<p><strong>Location:</strong> ${latitude.toFixed(4)}, ${longitude.toFixed(4)}</p>
		`,
		properties: {
			receiverId: receiver.id,
			callsign: receiver.callsign
		}
	});
}

/**
 * Create takeoff marker (green circle)
 */
export function createTakeoffMarker(latitude: number, longitude: number, altitude: number): Entity {
	return new Entity({
		position: Cartesian3.fromDegrees(longitude, latitude, altitude * FEET_TO_METERS),
		point: {
			pixelSize: 10,
			color: Color.GREEN,
			outlineColor: Color.WHITE,
			outlineWidth: 2
		},
		label: {
			text: 'Takeoff',
			font: '11px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 2,
			pixelOffset: { x: 0, y: -20 } as unknown as Cartesian2
		}
	});
}

/**
 * Create landing marker (red circle)
 */
export function createLandingMarker(latitude: number, longitude: number, altitude: number): Entity {
	return new Entity({
		position: Cartesian3.fromDegrees(longitude, latitude, altitude * FEET_TO_METERS),
		point: {
			pixelSize: 10,
			color: Color.RED,
			outlineColor: Color.WHITE,
			outlineWidth: 2
		},
		label: {
			text: 'Landing',
			font: '11px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 2,
			pixelOffset: { x: 0, y: -20 } as unknown as Cartesian2
		}
	});
}
