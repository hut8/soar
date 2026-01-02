// AR-specific TypeScript type definitions

export interface ARDeviceOrientation {
	heading: number; // Compass heading (0-360°, 0 = North)
	pitch: number; // Device pitch (beta: forward/back tilt, -180 to 180)
	roll: number; // Device roll (gamma: left/right tilt, -90 to 90)
	absolute: boolean; // True if relative to magnetic north
}

export interface ARUserPosition {
	latitude: number;
	longitude: number;
	altitude: number; // MSL in meters
	accuracy: number; // Position accuracy in meters
}

export interface ARAircraftPosition {
	aircraftId: string;
	registration: string | null;
	latitude: number;
	longitude: number;
	altitudeFeet: number;
	groundSpeedKnots: number | null;
	climbFpm: number | null;
	timestamp: string;
	distance: number; // km from user
	bearing: number; // 0-360°
	elevation: number; // degrees above horizon (-90 to 90)
}

export interface ARScreenPosition {
	x: number; // pixels from left
	y: number; // pixels from top
	visible: boolean; // within camera FOV
	distance: number; // km
	bearing: number; // relative to device heading
	elevation: number; // angle
}

export interface ARSettings {
	rangeKm: number; // 5-100
	filterAirborne: boolean; // Only show flying aircraft
	showDebug: boolean;
	fovHorizontal: number; // Camera FOV horizontal (degrees)
	fovVertical: number; // Camera FOV vertical (degrees)
}
