/**
 * ViewportAircraftController - Manages aircraft/cluster display based on map viewport
 *
 * This controller handles:
 * - Fetching aircraft or clusters based on the current viewport
 * - Switching between clustered and non-clustered display modes
 * - Managing the periodic refresh timer for clustered mode
 * - Coordinating between the aircraft registry and marker managers
 */

import { browser } from '$app/environment';
import { getLogger } from '$lib/logging';
import { isAircraftItem, isClusterItem, type AircraftOrCluster } from '$lib/types';
import type { AircraftRegistry } from './AircraftRegistry';
import type { FixFeed } from './FixFeed';
import type { AircraftMarkerManager } from './markers/AircraftMarkerManager';
import type { ClusterMarkerManager } from './markers/ClusterMarkerManager';

const logger = getLogger(['soar', 'ViewportAircraftController']);

export interface ViewportAircraftControllerOptions {
	/** The FixFeed instance for fetching aircraft data */
	fixFeed: FixFeed;
	/** The AircraftRegistry for managing aircraft state */
	aircraftRegistry: AircraftRegistry;
	/** The marker manager for individual aircraft */
	aircraftMarkerManager: AircraftMarkerManager;
	/** The marker manager for clusters */
	clusterMarkerManager: ClusterMarkerManager;
	/** Maximum number of aircraft to display before clustering */
	maxAircraftDisplay?: number;
	/** Callback when clustered mode changes */
	onClusteredModeChanged?: (isClustered: boolean) => void;
	/** Callback to check if area tracker is active */
	getAreaTrackerActive?: () => boolean;
	/** Callback to update WebSocket area subscriptions */
	updateAreaSubscriptions?: () => void;
	/** Callback to clear WebSocket area subscriptions */
	clearAreaSubscriptions?: () => void;
}

export class ViewportAircraftController {
	private map: google.maps.Map | null = null;
	private fixFeed: FixFeed;
	private aircraftRegistry: AircraftRegistry;
	private aircraftMarkerManager: AircraftMarkerManager;
	private clusterMarkerManager: ClusterMarkerManager;
	private maxAircraftDisplay: number;
	private onClusteredModeChanged?: (isClustered: boolean) => void;
	private getAreaTrackerActive: () => boolean;
	private updateAreaSubscriptions: () => void;
	private clearAreaSubscriptions: () => void;

	private _isClusteredMode = false;
	private clusterRefreshTimer: ReturnType<typeof setInterval> | null = null;

	constructor(options: ViewportAircraftControllerOptions) {
		this.fixFeed = options.fixFeed;
		this.aircraftRegistry = options.aircraftRegistry;
		this.aircraftMarkerManager = options.aircraftMarkerManager;
		this.clusterMarkerManager = options.clusterMarkerManager;
		this.maxAircraftDisplay = options.maxAircraftDisplay ?? 50;
		this.onClusteredModeChanged = options.onClusteredModeChanged;
		this.getAreaTrackerActive = options.getAreaTrackerActive ?? (() => false);
		this.updateAreaSubscriptions = options.updateAreaSubscriptions ?? (() => {});
		this.clearAreaSubscriptions = options.clearAreaSubscriptions ?? (() => {});
	}

	/**
	 * Set the Google Maps instance
	 */
	setMap(map: google.maps.Map): void {
		this.map = map;
	}

	/**
	 * Whether we're currently displaying clusters instead of individual aircraft
	 */
	get isClusteredMode(): boolean {
		return this._isClusteredMode;
	}

	/**
	 * Fetch and display aircraft or clusters based on the current viewport
	 */
	async fetchAndDisplay(): Promise<void> {
		if (!this.map) return;

		const bounds = this.map.getBounds();
		if (!bounds) return;

		const ne = bounds.getNorthEast();
		const sw = bounds.getSouthWest();

		try {
			logger.debug('[REST] Fetching aircraft in viewport...');

			const response = await this.fixFeed.fetchAircraftInBoundingBox(
				sw.lat(), // south
				ne.lat(), // north
				sw.lng(), // west
				ne.lng(), // east
				undefined,
				this.maxAircraftDisplay
			);

			const { items, total, clustered } = response;

			logger.debug('[REST] Received {count} items (total: {total}, clustered: {clustered})', {
				count: items.length,
				total,
				clustered
			});

			if (clustered) {
				this.handleClusteredResponse(items);
			} else {
				await this.handleAircraftResponse(items);
			}
		} catch (error) {
			logger.error('[REST] Failed to fetch aircraft in viewport: {error}', { error });
		}
	}

	/**
	 * Handle a clustered response from the API
	 */
	private handleClusteredResponse(items: AircraftOrCluster[]): void {
		logger.debug('[REST] Response is clustered, rendering cluster markers');

		// Enter clustered mode
		const wasClusteredMode = this._isClusteredMode;
		this._isClusteredMode = true;

		if (!wasClusteredMode) {
			this.onClusteredModeChanged?.(true);
		}

		this.startClusterRefreshTimer();

		// Clear WebSocket area subscriptions - clustered mode uses REST API polling instead
		this.clearAreaSubscriptions();

		// Clear aircraft registry - we're forgetting all aircraft outside viewport
		this.aircraftRegistry.clear();

		this.aircraftMarkerManager.clear();
		this.clusterMarkerManager.clear();

		for (const item of items) {
			if (isClusterItem(item)) {
				this.clusterMarkerManager.createMarker(item.data);
			}
		}

		logger.debug('[AIRCRAFT COUNT] {count} cluster markers on map', {
			count: this.clusterMarkerManager.getMarkers().size
		});
	}

	/**
	 * Handle an individual aircraft response from the API
	 */
	private async handleAircraftResponse(items: AircraftOrCluster[]): Promise<void> {
		logger.debug('[REST] Response has individual aircraft, rendering aircraft markers');

		// Exit clustered mode
		const wasClusteredMode = this._isClusteredMode;
		this._isClusteredMode = false;

		if (wasClusteredMode) {
			this.onClusteredModeChanged?.(false);
		}

		this.stopClusterRefreshTimer();

		// Restore WebSocket area subscriptions for real-time updates
		if (this.getAreaTrackerActive()) {
			this.updateAreaSubscriptions();
		}

		// Clear aircraft registry - we're forgetting all aircraft outside viewport
		this.aircraftRegistry.clear();

		this.clusterMarkerManager.clear();

		for (const item of items) {
			if (isAircraftItem(item)) {
				await this.aircraftRegistry.updateAircraftFromAircraftData(item.data);
			}
		}

		// Log the count of aircraft now on the map
		logger.debug('[AIRCRAFT COUNT] {count} aircraft markers on map', {
			count: this.aircraftMarkerManager.getMarkers().size
		});
	}

	/**
	 * Start the cluster refresh timer (refreshes every 60 seconds when tab is visible)
	 */
	private startClusterRefreshTimer(): void {
		// Clear any existing timer
		this.stopClusterRefreshTimer();

		logger.debug('[CLUSTER REFRESH] Starting 60-second refresh timer');

		this.clusterRefreshTimer = setInterval(async () => {
			// Only refresh if the page is visible (user has the tab active)
			if (browser && document.visibilityState === 'visible') {
				logger.debug('[CLUSTER REFRESH] Tab is visible, refreshing clusters...');
				await this.fetchAndDisplay();
			} else {
				logger.debug('[CLUSTER REFRESH] Tab is hidden, skipping refresh');
			}
		}, 60000); // 60 seconds
	}

	/**
	 * Stop the cluster refresh timer
	 */
	private stopClusterRefreshTimer(): void {
		if (this.clusterRefreshTimer) {
			logger.debug('[CLUSTER REFRESH] Stopping refresh timer');
			clearInterval(this.clusterRefreshTimer);
			this.clusterRefreshTimer = null;
		}
	}

	/**
	 * Stop the cluster refresh timer (public method for external control)
	 */
	stopRefreshTimer(): void {
		this.stopClusterRefreshTimer();
	}

	/**
	 * Clean up resources
	 */
	dispose(): void {
		this.stopClusterRefreshTimer();
		this.map = null;
	}
}
