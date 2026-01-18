/**
 * ClusterMarkerManager - Manages cluster markers and polygons on a Google Map
 *
 * Handles creating, updating, and clearing cluster markers that represent
 * groups of aircraft when zoomed out.
 */

import { SvelteMap } from 'svelte/reactivity';
import { getLogger } from '$lib/logging';
import type { AircraftCluster } from '$lib/types';

const logger = getLogger(['soar', 'ClusterMarkerManager']);

/** Z-index for cluster polygons */
const CLUSTER_POLYGON_Z_INDEX = 400;

/** Z-index for cluster markers */
const CLUSTER_MARKER_Z_INDEX = 500;

export interface ClusterMarkerManagerOptions {
	/** Callback when a cluster is clicked */
	onClusterClick?: (cluster: AircraftCluster) => void;
}

export class ClusterMarkerManager {
	private map: google.maps.Map | null = null;
	private markers = new SvelteMap<string, google.maps.marker.AdvancedMarkerElement>();
	private polygons = new SvelteMap<string, google.maps.Polygon>();
	private options: ClusterMarkerManagerOptions;

	constructor(options: ClusterMarkerManagerOptions = {}) {
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
	 * Create a cluster marker with polygon outline
	 */
	createMarker(cluster: AircraftCluster): google.maps.marker.AdvancedMarkerElement {
		logger.debug('[CLUSTER] Creating cluster marker: {params}', {
			params: {
				id: cluster.id,
				position: { lat: cluster.latitude, lng: cluster.longitude },
				count: cluster.count
			}
		});

		// Create polygon outline for the cluster bounds
		// DEBUG: Using bright red outline to visualize grid cells
		logger.debug('[CLUSTER DEBUG] Bounds: {bounds}', {
			bounds: {
				id: cluster.id,
				north: cluster.bounds.north,
				south: cluster.bounds.south,
				east: cluster.bounds.east,
				west: cluster.bounds.west,
				width: cluster.bounds.east - cluster.bounds.west,
				height: cluster.bounds.north - cluster.bounds.south
			}
		});

		const polygon = new google.maps.Polygon({
			paths: [
				{ lat: cluster.bounds.north, lng: cluster.bounds.west },
				{ lat: cluster.bounds.north, lng: cluster.bounds.east },
				{ lat: cluster.bounds.south, lng: cluster.bounds.east },
				{ lat: cluster.bounds.south, lng: cluster.bounds.west }
			],
			strokeColor: '#FF0000', // DEBUG: Bright red
			strokeOpacity: 1.0, // DEBUG: Fully opaque
			strokeWeight: 4, // DEBUG: Thick outline
			fillColor: '#FF0000', // DEBUG: Red fill
			fillOpacity: 0.1,
			map: this.map,
			zIndex: CLUSTER_POLYGON_Z_INDEX
		});

		// Store the polygon for later cleanup
		this.polygons.set(cluster.id, polygon);

		// Add click listener to polygon
		if (this.options.onClusterClick) {
			const clickHandler = this.options.onClusterClick;
			polygon.addListener('click', () => {
				clickHandler(cluster);
			});
		}

		// Create label marker at centroid - no solid background, just text with shadow for visibility
		const markerContent = document.createElement('div');
		markerContent.className = 'cluster-label';
		markerContent.style.display = 'flex';
		markerContent.style.flexDirection = 'column';
		markerContent.style.alignItems = 'center';
		markerContent.style.justifyContent = 'center';
		markerContent.style.gap = '2px';
		markerContent.style.cursor = 'pointer';
		markerContent.style.pointerEvents = 'auto';
		markerContent.style.position = 'relative';

		// Airplane SVG icon with white fill and shadow - SMALLER
		const iconDiv = document.createElement('div');
		iconDiv.style.display = 'flex';
		iconDiv.style.alignItems = 'center';
		iconDiv.style.justifyContent = 'center';
		iconDiv.style.filter = 'drop-shadow(0 2px 4px rgba(0, 0, 0, 0.8))';
		iconDiv.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="white">
			<path d="M21 16v-2l-8-5V3.5c0-.83-.67-1.5-1.5-1.5S10 2.67 10 3.5V9l-8 5v2l8-2.5V19l-2 1.5V22l3.5-1 3.5 1v-1.5L13 19v-5.5l8 2.5z"/>
		</svg>`;

		// Count label with shadow for visibility - SMALLER
		const countLabel = document.createElement('div');
		countLabel.style.color = 'white';
		countLabel.style.fontWeight = 'bold';
		countLabel.style.fontSize = '14px';
		countLabel.style.textShadow = '0 2px 4px rgba(0, 0, 0, 0.8), 0 0 8px rgba(0, 0, 0, 0.6)';
		countLabel.style.whiteSpace = 'nowrap';
		countLabel.style.lineHeight = '1';
		countLabel.textContent = cluster.count.toString();

		markerContent.appendChild(iconDiv);
		markerContent.appendChild(countLabel);

		const marker = new google.maps.marker.AdvancedMarkerElement({
			position: { lat: cluster.latitude, lng: cluster.longitude },
			map: this.map,
			title: `${cluster.count} aircraft in this area`,
			content: markerContent,
			zIndex: CLUSTER_MARKER_Z_INDEX
		});

		if (this.options.onClusterClick) {
			const clickHandler = this.options.onClusterClick;
			marker.addListener('click', () => {
				clickHandler(cluster);
			});
		}

		markerContent.addEventListener('mouseenter', () => {
			markerContent.style.transform = 'scale(1.15)';
		});

		markerContent.addEventListener('mouseleave', () => {
			markerContent.style.transform = 'scale(1)';
		});

		// Store the marker
		this.markers.set(cluster.id, marker);

		return marker;
	}

	/**
	 * Handle cluster click - zoom to cluster bounds
	 */
	zoomToCluster(cluster: AircraftCluster): void {
		logger.debug('[CLUSTER] Zooming to cluster: {id}', { id: cluster.id });

		if (!this.map) return;

		const bounds = new google.maps.LatLngBounds(
			{ lat: cluster.bounds.south, lng: cluster.bounds.west },
			{ lat: cluster.bounds.north, lng: cluster.bounds.east }
		);

		this.map.fitBounds(bounds);

		// Zoom in slightly more than just fitting bounds
		const currentZoom = this.map.getZoom() || 10;
		this.map.setZoom(currentZoom + 1);
	}

	/**
	 * Clear all cluster markers and polygons
	 */
	clear(): void {
		logger.debug('[CLUSTER] Clearing all cluster markers. Count: {count}', {
			count: this.markers.size
		});
		this.markers.forEach((marker) => {
			marker.map = null;
		});
		this.markers.clear();

		// Also clear cluster polygons
		logger.debug('[CLUSTER] Clearing all cluster polygons. Count: {count}', {
			count: this.polygons.size
		});
		this.polygons.forEach((polygon) => {
			polygon.setMap(null);
		});
		this.polygons.clear();
		logger.debug('[CLUSTER] All cluster markers and polygons cleared');
	}

	/**
	 * Dispose of the manager and clean up resources
	 */
	dispose(): void {
		this.clear();
		this.map = null;
	}
}
