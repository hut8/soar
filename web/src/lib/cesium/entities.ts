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
	type Cartesian2,
	PolygonHierarchy
} from 'cesium';
import type { Aircraft, Fix, Flight, Airport, Receiver, AircraftCluster } from '$lib/types';
import { altitudeToColor, formatAltitudeWithTime } from '$lib/utils/mapColors';
import { getAircraftTitle } from '$lib/formatters';

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
 * @param showLabel - Whether to show the text label (default: true)
 * @returns Cesium Entity configured as aircraft marker
 */
export function createAircraftEntity(
	aircraft: Aircraft,
	fix: Fix,
	showLabel: boolean = true
): Entity {
	const altitude = fix.altitudeMslFeet || 0;
	const altitudeMeters = altitude * FEET_TO_METERS;

	// Get altitude-based color
	const color = altitudeToColor(altitude);

	// Format altitude with time
	const { altitudeText, isOld } = formatAltitudeWithTime(altitude, fix.receivedAt);

	// Create icon URL with aircraft heading
	const iconUrl = createAircraftIconSVG(color, fix.trackDegrees || 0);

	const displayName = getAircraftTitle(aircraft);

	return new Entity({
		id: aircraft.id,
		name: displayName,
		position: Cartesian3.fromDegrees(fix.longitude, fix.latitude, altitudeMeters),
		billboard: {
			image: iconUrl,
			scale: 1.0,
			verticalOrigin: VerticalOrigin.CENTER, // Center the icon so selection box is centered
			horizontalOrigin: HorizontalOrigin.CENTER,
			heightReference: HeightReference.NONE // Use absolute altitude
		},
		label: showLabel
			? {
					text: `${displayName}\n${altitudeText}`,
					font: '12px sans-serif',
					fillColor: Color.WHITE,
					outlineColor: Color.BLACK,
					outlineWidth: 3,
					pixelOffset: { x: 0, y: -30 } as unknown as Cartesian2, // Adjusted offset for centered billboard
					heightReference: HeightReference.NONE
				}
			: undefined, // Hide label when showLabel is false
		description: `
			<h3>${displayName}</h3>
			<p><strong>Model:</strong> ${aircraft.aircraftModel || 'Unknown'}</p>
			<p><strong>Altitude:</strong> ${altitude} ft MSL</p>
			<p><strong>Speed:</strong> ${fix.groundSpeedKnots || '---'} kts</p>
			<p><strong>Heading:</strong> ${fix.trackDegrees || '---'}°</p>
			<p><strong>Last seen:</strong> ${altitudeText}</p>
		`,
		properties: {
			aircraftId: aircraft.id,
			registration: aircraft.registration,
			fixId: fix.id,
			altitude: altitude,
			timestamp: fix.receivedAt,
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
		const altitude = fix.altitudeMslFeet || 0;
		return Cartesian3.fromDegrees(fix.longitude, fix.latitude, altitude * FEET_TO_METERS);
	});

	// Calculate average color for the path
	let pathColor: Color;
	if (colorScheme === 'altitude') {
		const altitudes = fixes.map((f) => f.altitudeMslFeet || 0);
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
		name: `Flight ${flight.registration || flight.deviceAddress}`,
		polyline: {
			positions,
			width: 3,
			material: pathColor,
			clampToGround: false // Show actual altitude
		},
		description: `
			<h3>Flight Path</h3>
			<p><strong>Aircraft:</strong> ${flight.registration || flight.deviceAddress}</p>
			<p><strong>Takeoff:</strong> ${flight.takeoffTime ? new Date(flight.takeoffTime).toLocaleString() : 'Unknown'}</p>
			<p><strong>Landing:</strong> ${flight.landingTime ? new Date(flight.landingTime).toLocaleString() : 'In Progress'}</p>
			<p><strong>Duration:</strong> ${flight.durationSeconds ? Math.round(flight.durationSeconds / 60) + ' min' : 'N/A'}</p>
			<p><strong>Max Altitude:</strong> ${Math.max(...fixes.map((f) => f.altitudeMslFeet || 0))} ft</p>
		`,
		properties: {
			flightId: flight.id,
			aircraftId: flight.aircraftId,
			takeoffTime: flight.takeoffTime,
			landingTime: flight.landingTime,
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
	const latitude = airport.latitudeDeg ?? 0;
	const longitude = airport.longitudeDeg ?? 0;
	const elevation = (airport.elevationFt || 0) * FEET_TO_METERS;

	// Create airport icon SVG with runway symbol
	const iconSvg = `
		<svg width="32" height="32" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
			<!-- Outer circle -->
			<circle cx="16" cy="16" r="14" fill="#10b981" stroke="white" stroke-width="3"/>
			<!-- Runway cross -->
			<rect x="14" y="6" width="4" height="20" fill="white" rx="1"/>
			<rect x="6" y="14" width="20" height="4" fill="white" rx="1"/>
			<!-- Center dot -->
			<circle cx="16" cy="16" r="3" fill="white"/>
		</svg>
	`;
	const iconUrl = `data:image/svg+xml;base64,${btoa(iconSvg)}`;

	return new Entity({
		id: `airport-${airport.id}`,
		name: airport.ident,
		position: Cartesian3.fromDegrees(longitude, latitude, elevation),
		billboard: {
			image: iconUrl,
			scale: 1.2,
			verticalOrigin: VerticalOrigin.CENTER,
			horizontalOrigin: HorizontalOrigin.CENTER
		},
		label: {
			text: airport.ident,
			font: 'bold 13px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 3,
			pixelOffset: { x: 0, y: -28 } as unknown as Cartesian2,
			disableDepthTestDistance: 50000 // Hide when far away
		},
		description: `
			<h3>${airport.name}</h3>
			<p><strong>Identifier:</strong> ${airport.ident}</p>
			<p><strong>Type:</strong> ${airport.airportType}</p>
			<p><strong>Elevation:</strong> ${airport.elevationFt || '---'} ft</p>
			<p><strong>Location:</strong> ${airport.municipality || '---'}, ${airport.isoCountry || '---'}</p>
			${airport.icaoCode ? `<p><strong>ICAO:</strong> ${airport.icaoCode}</p>` : ''}
			${airport.iataCode ? `<p><strong>IATA:</strong> ${airport.iataCode}</p>` : ''}
		`,
		properties: {
			airportId: airport.id,
			ident: airport.ident,
			type: airport.airportType
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

/**
 * Create cluster entity with bounding box and label
 * Shows aggregated aircraft count in a geographic area
 */
export function createClusterEntity(cluster: AircraftCluster): Entity {
	const { north, south, east, west } = cluster.bounds;
	const centerLat = (north + south) / 2;
	const centerLon = (east + west) / 2;

	// Create polygon outline showing cluster bounds
	const positions = [
		Cartesian3.fromDegrees(west, north, 0),
		Cartesian3.fromDegrees(east, north, 0),
		Cartesian3.fromDegrees(east, south, 0),
		Cartesian3.fromDegrees(west, south, 0)
	];

	return new Entity({
		id: `cluster-${cluster.id}`,
		name: `${cluster.count} aircraft`,
		position: Cartesian3.fromDegrees(centerLon, centerLat, 0),
		label: {
			text: cluster.count.toString(),
			font: 'bold 20px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 3,
			pixelOffset: { x: 0, y: 0 } as unknown as Cartesian2,
			heightReference: HeightReference.CLAMP_TO_GROUND
		},
		polygon: {
			hierarchy: new PolygonHierarchy(positions),
			material: Color.fromCssColorString('rgba(239, 68, 68, 0.4)'), // Red with 40% opacity
			outline: true,
			outlineColor: Color.fromCssColorString('rgba(220, 38, 38, 0.9)'), // Darker red outline
			outlineWidth: 3,
			heightReference: HeightReference.CLAMP_TO_GROUND
		},
		description: `
			<h3>Aircraft Cluster</h3>
			<p><strong>Aircraft Count:</strong> ${cluster.count}</p>
			<p><strong>Area:</strong> ${Math.abs(north - south).toFixed(2)}° × ${Math.abs(east - west).toFixed(2)}°</p>
			<p>Click to zoom in and see individual aircraft</p>
		`,
		properties: {
			clusterId: cluster.id,
			clusterCount: cluster.count,
			clusterBounds: cluster.bounds,
			isCluster: true
		}
	});
}
