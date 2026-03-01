/**
 * RFLinkLayerManager - Draws brief, fading lines between aircraft and the
 * OGN receiver that heard each fix, visualizing the RF link in real time.
 *
 * Lines fade out over ~2 seconds using a requestAnimationFrame loop that
 * updates per-feature opacity in a GeoJSON source.
 */

import type maplibregl from 'maplibre-gl';
import { getLogger } from '$lib/logging';

const logger = getLogger(['soar', 'maplibre', 'RFLinkLayerManager']);

/** How long each link line stays visible (ms) */
const LINK_LIFETIME_MS = 2000;

/** Source and layer IDs */
const SOURCE_ID = 'rf-links-source';
const LAYER_ID = 'rf-links-layer';

/** Line color — light sky blue that stands out on both light and dark maps */
const LINE_COLOR = '#38bdf8';

/** Minimum interval between source updates (~30fps) */
const MIN_FRAME_MS = 33;

interface ActiveLink {
	aircraftLng: number;
	aircraftLat: number;
	receiverLng: number;
	receiverLat: number;
	createdAt: number;
}

export class RFLinkLayerManager {
	private map: maplibregl.Map | null = null;
	private links: ActiveLink[] = [];
	private animFrameId: number | null = null;
	private layerAdded: boolean = false;
	private lastFrameTime: number = 0;

	/**
	 * Set the map instance and add the source + layer.
	 */
	setMap(map: maplibregl.Map): void {
		this.map = map;
		this.addSourceAndLayer();
	}

	/**
	 * Add an RF link line from an aircraft position to a receiver position.
	 * Starts the animation loop if it isn't already running.
	 */
	addLink(
		aircraftLat: number,
		aircraftLng: number,
		receiverLat: number,
		receiverLng: number
	): void {
		this.links.push({
			aircraftLat,
			aircraftLng,
			receiverLat,
			receiverLng,
			createdAt: performance.now()
		});

		if (this.animFrameId === null) {
			this.tick();
		}
	}

	/**
	 * Clear all active links immediately.
	 */
	clear(): void {
		this.links = [];
		if (this.animFrameId !== null) {
			cancelAnimationFrame(this.animFrameId);
			this.animFrameId = null;
		}
		this.updateSource();
	}

	/**
	 * Clean up all resources.
	 */
	dispose(): void {
		if (this.animFrameId !== null) {
			cancelAnimationFrame(this.animFrameId);
			this.animFrameId = null;
		}
		this.links = [];
		this.removeLayer();
		this.map = null;
	}

	// ── private ──────────────────────────────────────────────

	private addSourceAndLayer(): void {
		if (!this.map) return;

		// Avoid duplicates after style changes
		if (this.map.getSource(SOURCE_ID)) return;

		this.map.addSource(SOURCE_ID, {
			type: 'geojson',
			data: this.emptyCollection()
		});

		this.map.addLayer({
			id: LAYER_ID,
			type: 'line',
			source: SOURCE_ID,
			paint: {
				'line-color': LINE_COLOR,
				'line-width': 1.5,
				'line-opacity': ['get', 'opacity']
			}
		});

		this.layerAdded = true;
		logger.debug('RF link layer added');
	}

	private removeLayer(): void {
		if (!this.map || !this.layerAdded) return;

		if (this.map.getLayer(LAYER_ID)) {
			this.map.removeLayer(LAYER_ID);
		}
		if (this.map.getSource(SOURCE_ID)) {
			this.map.removeSource(SOURCE_ID);
		}

		this.layerAdded = false;
	}

	/**
	 * Animation loop: update opacities, prune expired links, push GeoJSON.
	 */
	private tick = (): void => {
		const now = performance.now();

		// Remove expired links
		this.links = this.links.filter((l) => now - l.createdAt < LINK_LIFETIME_MS);

		if (this.links.length === 0) {
			// Nothing to draw — update source to clear and stop looping
			this.updateSource();
			this.animFrameId = null;
			return;
		}

		// Throttle source updates to ~30fps
		if (now - this.lastFrameTime >= MIN_FRAME_MS) {
			this.updateSource(now);
			this.lastFrameTime = now;
		}

		this.animFrameId = requestAnimationFrame(this.tick);
	};

	private updateSource(now?: number): void {
		if (!this.map) return;

		const source = this.map.getSource(SOURCE_ID) as maplibregl.GeoJSONSource | undefined;
		if (!source) return;

		const ts = now ?? performance.now();

		const features: GeoJSON.Feature<GeoJSON.LineString>[] = this.links.map((link) => {
			const age = ts - link.createdAt;
			const opacity = Math.max(0, 1 - age / LINK_LIFETIME_MS);

			return {
				type: 'Feature' as const,
				geometry: {
					type: 'LineString' as const,
					coordinates: [
						[link.aircraftLng, link.aircraftLat],
						[link.receiverLng, link.receiverLat]
					]
				},
				properties: {
					opacity
				}
			};
		});

		source.setData({
			type: 'FeatureCollection',
			features
		});
	}

	private emptyCollection(): GeoJSON.FeatureCollection {
		return { type: 'FeatureCollection', features: [] };
	}
}
