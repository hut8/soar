/**
 * Cesium entity factory functions for geofence visualization
 * Creates stacked cylinders representing multi-layer geofences
 */

import {
	Entity,
	Cartesian3,
	Color,
	CylinderGraphics,
	LabelGraphics,
	VerticalOrigin,
	HeightReference,
	type Viewer
} from 'cesium';
import type { Geofence, GeofenceLayer } from '$lib/types';

/**
 * Feet to meters conversion factor
 */
const FEET_TO_METERS = 0.3048;

/**
 * Nautical miles to meters conversion factor
 */
const NM_TO_METERS = 1852;

/**
 * Default colors for geofence layers (with transparency)
 * Each layer gets a slightly different shade
 */
const LAYER_COLORS = [
	Color.fromCssColorString('rgba(59, 130, 246, 0.25)'), // Blue
	Color.fromCssColorString('rgba(34, 197, 94, 0.25)'), // Green
	Color.fromCssColorString('rgba(249, 115, 22, 0.25)'), // Orange
	Color.fromCssColorString('rgba(168, 85, 247, 0.25)'), // Purple
	Color.fromCssColorString('rgba(236, 72, 153, 0.25)') // Pink
];

const LAYER_OUTLINE_COLORS = [
	Color.fromCssColorString('rgba(37, 99, 235, 0.8)'), // Blue
	Color.fromCssColorString('rgba(22, 163, 74, 0.8)'), // Green
	Color.fromCssColorString('rgba(234, 88, 12, 0.8)'), // Orange
	Color.fromCssColorString('rgba(147, 51, 234, 0.8)'), // Purple
	Color.fromCssColorString('rgba(219, 39, 119, 0.8)') // Pink
];

/**
 * Get color for a layer by index
 */
function getLayerColor(index: number): Color {
	return LAYER_COLORS[index % LAYER_COLORS.length];
}

/**
 * Get outline color for a layer by index
 */
function getLayerOutlineColor(index: number): Color {
	return LAYER_OUTLINE_COLORS[index % LAYER_OUTLINE_COLORS.length];
}

/**
 * Create a Cesium Entity for a single geofence layer (cylinder)
 * @param geofence - The parent geofence
 * @param layer - The layer to render
 * @param layerIndex - Index of this layer
 * @returns Cesium Entity configured as a cylinder
 */
export function createGeofenceLayerEntity(
	geofence: Geofence,
	layer: GeofenceLayer,
	layerIndex: number
): Entity {
	const heightMeters = (layer.ceilingFt - layer.floorFt) * FEET_TO_METERS;
	const radiusMeters = layer.radiusNm * NM_TO_METERS;
	const bottomAltitudeMeters = layer.floorFt * FEET_TO_METERS;

	// Position at center of the cylinder (vertically)
	const centerAltitudeMeters = bottomAltitudeMeters + heightMeters / 2;

	return new Entity({
		id: `geofence-${geofence.id}-layer-${layerIndex}`,
		name: `${geofence.name} (${layer.floorFt.toLocaleString()}-${layer.ceilingFt.toLocaleString()} ft)`,
		position: Cartesian3.fromDegrees(
			geofence.centerLongitude,
			geofence.centerLatitude,
			centerAltitudeMeters
		),
		cylinder: new CylinderGraphics({
			length: heightMeters,
			topRadius: radiusMeters,
			bottomRadius: radiusMeters,
			material: getLayerColor(layerIndex),
			outline: true,
			outlineColor: getLayerOutlineColor(layerIndex),
			outlineWidth: 2,
			numberOfVerticalLines: 0 // Smooth cylinder
		}),
		properties: {
			geofenceId: geofence.id,
			geofenceName: geofence.name,
			layerIndex: layerIndex,
			floorFt: layer.floorFt,
			ceilingFt: layer.ceilingFt,
			radiusNm: layer.radiusNm
		}
	});
}

/**
 * Create a Cesium Entity for the geofence center marker
 * @param geofence - The geofence
 * @returns Cesium Entity for the center point
 */
export function createGeofenceCenterEntity(geofence: Geofence): Entity {
	return new Entity({
		id: `geofence-${geofence.id}-center`,
		name: geofence.name,
		position: Cartesian3.fromDegrees(geofence.centerLongitude, geofence.centerLatitude, 0),
		point: {
			pixelSize: 10,
			color: Color.RED,
			outlineColor: Color.WHITE,
			outlineWidth: 2,
			heightReference: HeightReference.CLAMP_TO_GROUND
		},
		label: new LabelGraphics({
			text: geofence.name,
			font: '14px sans-serif',
			fillColor: Color.WHITE,
			outlineColor: Color.BLACK,
			outlineWidth: 2,
			style: 2, // FILL_AND_OUTLINE
			verticalOrigin: VerticalOrigin.BOTTOM,
			pixelOffset: new Cartesian3(0, -15, 0) as unknown as import('cesium').Cartesian2,
			heightReference: HeightReference.CLAMP_TO_GROUND
		}),
		properties: {
			geofenceId: geofence.id,
			geofenceName: geofence.name,
			isCenter: true
		}
	});
}

/**
 * Create all Cesium entities for a geofence
 * Includes one cylinder per layer plus a center point marker
 * @param geofence - The geofence to visualize
 * @returns Array of Cesium Entities
 */
export function createGeofenceEntities(geofence: Geofence): Entity[] {
	const entities: Entity[] = [];

	// Sort layers by floor altitude (bottom to top) for consistent coloring
	const sortedLayers = [...geofence.layers].sort((a, b) => a.floorFt - b.floorFt);

	// Create cylinder for each layer
	sortedLayers.forEach((layer, index) => {
		entities.push(createGeofenceLayerEntity(geofence, layer, index));
	});

	// Add center point marker
	entities.push(createGeofenceCenterEntity(geofence));

	return entities;
}

/**
 * Add all geofence entities to a Cesium viewer
 * @param viewer - The Cesium Viewer
 * @param geofence - The geofence to add
 * @returns Array of added Entity IDs
 */
export function addGeofenceToViewer(viewer: Viewer, geofence: Geofence): string[] {
	const entities = createGeofenceEntities(geofence);
	const ids: string[] = [];

	entities.forEach((entity) => {
		viewer.entities.add(entity);
		if (entity.id) {
			ids.push(entity.id);
		}
	});

	return ids;
}

/**
 * Remove all entities for a geofence from a Cesium viewer
 * @param viewer - The Cesium Viewer
 * @param geofenceId - The geofence ID
 */
export function removeGeofenceFromViewer(viewer: Viewer, geofenceId: string): void {
	// Find and remove all entities with matching geofence ID prefix
	const toRemove: Entity[] = [];
	viewer.entities.values.forEach((entity) => {
		if (entity.id?.startsWith(`geofence-${geofenceId}`)) {
			toRemove.push(entity);
		}
	});

	toRemove.forEach((entity) => {
		viewer.entities.remove(entity);
	});
}

/**
 * Fly the camera to show the entire geofence
 * @param viewer - The Cesium Viewer
 * @param geofence - The geofence to fly to
 */
export function flyToGeofence(viewer: Viewer, geofence: Geofence): void {
	// Find the largest radius for camera positioning
	const maxRadius = Math.max(...geofence.layers.map((l) => l.radiusNm));

	// Calculate camera distance based on max radius (in meters)
	const radiusMeters = maxRadius * NM_TO_METERS;
	const cameraDistance = radiusMeters * 3; // 3x the radius for good viewing

	viewer.camera.flyTo({
		destination: Cartesian3.fromDegrees(
			geofence.centerLongitude,
			geofence.centerLatitude,
			cameraDistance
		),
		duration: 1.5
	});
}
