// Google Maps API type declarations
declare global {
	interface Window {
		google: {
			maps: {
				Map: new (element: HTMLElement, options?: GoogleMapOptions) => GoogleMap;
				Marker: new (options?: GoogleMarkerOptions) => GoogleMarker;
				Size: new (width: number, height: number) => GoogleSize;
				Point: new (x: number, y: number) => GooglePoint;
				MapTypeId: {
					TERRAIN: string;
				};
				MapTypeControlStyle: {
					HORIZONTAL_BAR: string;
				};
				ControlPosition: {
					TOP_CENTER: string;
					RIGHT_CENTER: string;
					RIGHT_TOP: string;
				};
			};
		};
	}

	interface GoogleMapOptions {
		center?: { lat: number; lng: number };
		zoom?: number;
		mapTypeId?: string;
		restriction?: {
			latLngBounds?: {
				north: number;
				south: number;
				east: number;
				west: number;
			};
			strictBounds?: boolean;
		};
		mapTypeControl?: boolean;
		mapTypeControlOptions?: {
			style?: string;
			position?: string;
		};
		zoomControl?: boolean;
		zoomControlOptions?: {
			position?: string;
		};
		scaleControl?: boolean;
		streetViewControl?: boolean;
		streetViewControlOptions?: {
			position?: string;
		};
		fullscreenControl?: boolean;
		fullscreenControlOptions?: {
			position?: string;
		};
	}

	interface GoogleMap {
		setCenter(latlng: { lat: number; lng: number }): void;
		setZoom(zoom: number): void;
	}

	interface GoogleMarkerOptions {
		position?: { lat: number; lng: number };
		map?: GoogleMap;
		title?: string;
		icon?: string | GoogleMarkerIcon;
	}

	interface GoogleMarker {
		setPosition(latlng: { lat: number; lng: number }): void;
		setMap(map: GoogleMap | null): void;
	}

	interface GoogleMarkerIcon {
		url?: string;
		size?: GoogleSize;
		origin?: GooglePoint;
		anchor?: GooglePoint;
	}

	interface GoogleSize {
		width: number;
		height: number;
	}

	interface GooglePoint {
		x: number;
		y: number;
	}
}

export {};
