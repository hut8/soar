// Core data types for the application

// Import auto-generated types from Rust
import type { Aircraft } from './generated/Aircraft';
import type { AircraftView } from './generated/AircraftView';
import type { AircraftCluster } from './generated/AircraftCluster';
import type { AircraftOrCluster } from './generated/AircraftOrCluster';
import type { AircraftSearchResponse } from './generated/AircraftSearchResponse';
import type { ClusterBounds } from './generated/ClusterBounds';
import type { Fix } from './generated/Fix';
import type { AdsbEmitterCategory } from './generated/AdsbEmitterCategory';
import type { AircraftType } from './generated/AircraftType';
import type { AircraftCategory } from './generated/AircraftCategory';
import type { FlightView } from './generated/FlightView';
import type { FlightState } from './generated/FlightState';
import type { AddressType } from './generated/AddressType';
import type { UserView } from './generated/UserView';
import type { ReceiverView } from './generated/ReceiverView';
import type { AirportView } from './generated/AirportView';
import type { RunwayView } from './generated/RunwayView';
import type { RunwayEnd as RunwayEndGenerated } from './generated/RunwayEnd';
import type { ModelDataView } from './generated/ModelDataView';
import type { AircraftRegistrationView } from './generated/AircraftRegistrationView';
import type { AircraftModelView } from './generated/AircraftModelView';

// Import auto-generated geocoding types from Rust
import type { ReverseGeocodeResponse } from './generated/ReverseGeocodeResponse';

// Import auto-generated data stream types from Rust
import type { DataStream } from './generated/DataStream';
import type { StreamFormat } from './generated/StreamFormat';

// Import auto-generated geofence types from Rust
import type { Geofence } from './generated/Geofence';
import type { GeofenceLayer } from './generated/GeofenceLayer';
import type { GeofenceWithCounts } from './generated/GeofenceWithCounts';
import type { GeofenceListResponse } from './generated/GeofenceListResponse';
import type { GeofenceDetailResponse } from './generated/GeofenceDetailResponse';
import type { CreateGeofenceRequest } from './generated/CreateGeofenceRequest';
import type { UpdateGeofenceRequest } from './generated/UpdateGeofenceRequest';
import type { GeofenceSubscriber } from './generated/GeofenceSubscriber';
import type { AircraftGeofence } from './generated/AircraftGeofence';
import type { GeofenceExitEvent } from './generated/GeofenceExitEvent';
import type { GeofenceExitEventsResponse } from './generated/GeofenceExitEventsResponse';

// Re-export them for external use
export type {
	Aircraft,
	AircraftView,
	AircraftCluster,
	AircraftOrCluster,
	AircraftSearchResponse,
	ClusterBounds,
	Fix,
	AdsbEmitterCategory,
	AircraftType,
	AircraftCategory,
	FlightView,
	FlightState,
	AddressType,
	UserView,
	ReceiverView,
	AirportView,
	RunwayView,
	// Geofence types
	Geofence,
	GeofenceLayer,
	GeofenceWithCounts,
	GeofenceListResponse,
	GeofenceDetailResponse,
	CreateGeofenceRequest,
	UpdateGeofenceRequest,
	GeofenceSubscriber,
	AircraftGeofence,
	GeofenceExitEvent,
	GeofenceExitEventsResponse,
	ModelDataView,
	// Data stream types
	DataStream,
	StreamFormat,
	// Geocoding types
	ReverseGeocodeResponse
};

// Type aliases for backward compatibility
export type Flight = FlightView;
export type User = UserView;
export type Receiver = ReceiverView;
export type Airport = AirportView;
export type Runway = RunwayView;
export type RunwayEnd = RunwayEndGenerated;

// API Response Wrapper Types
export interface DataResponse<T> {
	data: T;
}

export interface DataListResponse<T> {
	data: T[];
}

export interface DataListResponseWithTotal<T> {
	data: T[];
	total: number;
}

export interface PaginationMetadata {
	page: number;
	totalPages: number;
	totalCount: number;
}

export interface PaginatedDataResponse<T> {
	data: T[];
	metadata: PaginationMetadata;
}

export interface Point {
	latitude: number;
	longitude: number;
}

// Lightweight path point for flight trail rendering (from RDP-compressed /path endpoint)
export interface PathPoint {
	latitude: number;
	longitude: number;
	altitudeFeet: number | null;
	speedKnots: number | null;
}

export interface Location {
	id: string;
	street1?: string;
	street2?: string;
	city?: string;
	state?: string;
	zipCode?: string;
	countryCode?: string;
	geolocation?: Point;
	createdAt: string;
	updatedAt: string;
}

export interface Club {
	id: string;
	name: string;
	homeBaseAirportId?: number;
	homeBaseAirportIdent?: string;
	location?: Location;
	createdAt: string;
	updatedAt: string;
	similarityScore?: number;
	distanceMeters?: number;
}

// For backward compatibility, extend Club with isSoaring for club selector
export interface ClubWithSoaring extends Club {
	isSoaring?: boolean;
}

export interface ComboboxData {
	label: string;
	value: string;
	club: ClubWithSoaring;
}

// Aircraft registration and model types (auto-generated from Rust via ts-rs)
export type AircraftRegistration = AircraftRegistrationView;
export type AircraftModel = AircraftModelView;

// Type guards for AircraftOrCluster (not auto-generated)
export function isAircraftItem(
	item: AircraftOrCluster
): item is { type: 'aircraft'; data: Aircraft } {
	return item.type === 'aircraft';
}

export function isClusterItem(
	item: AircraftOrCluster
): item is { type: 'cluster'; data: AircraftCluster } {
	return item.type === 'cluster';
}

// AircraftWithRegistration extends Aircraft with optional aircraft registration and detailed model information
export interface AircraftWithRegistration extends Aircraft {
	aircraftRegistration?: AircraftRegistration;
	// Detailed aircraft model information from FAA database
	aircraftModelDetails?: AircraftModel;
}

// Frontend-specific Aircraft interface that matches how we actually use Aircraft in the UI
// Uses Partial to make all fields optional for flexibility in partial objects
export type AircraftPartial = Partial<Aircraft> & {
	id: string; // ID is always required
};

// Extended Fix interface for WebSocket/legacy responses that include extra fields not in the generated Rust Fix type
// Note: New code should avoid using these extra fields and rely on proper joins instead
export interface FixWithExtras extends Fix {
	deviceAddressHex?: string; // Legacy field, prefer aircraftId with proper join
	registration?: string; // Aircraft registration (joined, not in Rust Fix)
	model?: string; // Aircraft model (joined, not in Rust Fix)
	aprsType?: string; // APRS message type (from sourceMetadata, used in WebSocket)
	via?: string[]; // APRS via path (from sourceMetadata, used in WebSocket)
}

// FixWithAircraft type from the backend - fix with optional aircraft data
// Used by receiver fixes endpoint
export interface FixWithAircraft extends Fix {
	aircraft: import('./generated/AircraftView').AircraftView | null;
}

// Raw message response from /data/raw-messages/{id} endpoint
export interface RawMessageResponse {
	id: string;
	rawMessage: string; // UTF-8 for APRS/SBS, hex-encoded for Beast
	source: 'aprs' | 'beast' | 'sbs';
	receivedAt: string;
	receiverId: string | null;
	debugFormat?: string; // Pretty-printed Rust debug format of parsed message
}

// FlightDetails combines Flight with full Aircraft data
// Used when displaying flight lists that need complete aircraft information
export interface FlightDetails {
	flight: Flight;
	aircraft: Aircraft | null; // null if aircraft data couldn't be fetched
}

export interface WatchlistEntry {
	userId: string;
	aircraftId: string;
	sendEmail: boolean;
	createdAt: string;
	updatedAt: string;
}

export interface WatchlistEntryWithAircraft extends WatchlistEntry {
	aircraft?: Aircraft;
}

// Airspace interface - GeoJSON Feature format
export interface Airspace {
	type: 'Feature';
	geometry: {
		type: 'Polygon' | 'MultiPolygon';
		coordinates: number[][][] | number[][][][];
	};
	properties: {
		id: string;
		openaipId: string;
		name: string;
		airspaceClass: 'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'SUA' | null;
		airspaceType: string;
		lowerLimit: string;
		upperLimit: string;
		remarks: string | null;
		countryCode: string | null;
		activityType: string | null;
	};
}

// Airspace collection - GeoJSON FeatureCollection format
export interface AirspaceFeatureCollection {
	type: 'FeatureCollection';
	features: Airspace[];
}

// Coverage map - H3 hexagonal coverage visualization
export interface CoverageHexProperties {
	h3Index: string;
	resolution: number;
	fixCount: number;
	firstSeenAt: string;
	lastSeenAt: string;
	minAltitudeMslFeet: number | null;
	maxAltitudeMslFeet: number | null;
	avgAltitudeMslFeet: number | null;
	coverageHours: number;
}

export interface CoverageHexFeature {
	type: 'Feature';
	geometry: {
		type: 'Polygon';
		coordinates: number[][][];
	};
	properties: CoverageHexProperties;
}

export interface CoverageGeoJsonResponse {
	type: 'FeatureCollection';
	features: CoverageHexFeature[];
}

// Hex fixes modal - individual position fixes within a coverage hex
export interface FixesInHexResponse {
	data: Fix[];
	total: number;
	h3Index: string;
	resolution: number;
}

// Hex receivers response - receivers that contributed to a coverage hex
export interface HexReceiversResponse {
	data: Receiver[];
	h3Index: string;
}

// Browser API Extensions
// DeviceOrientationEvent with iOS-specific webkitCompassHeading property
export interface DeviceOrientationEventWithCompass extends DeviceOrientationEvent {
	webkitCompassHeading?: number; // iOS-specific: true magnetic heading (0-360 degrees)
}
