// Google Maps API type declarations
declare global {
	interface Window {
		google: {
			maps: {
				Map: any;
				Marker: any;
				Size: any;
				Point: any;
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
}

export {};