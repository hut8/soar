// Core data types for the application
// All API response types are auto-generated from Rust via ts-rs.
// Only frontend-only types (UI extensions, compositions, browser APIs) are defined manually here.

// Import auto-generated types from Rust
import type { ClubJoinRequestView } from './generated/ClubJoinRequestView';
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
import type { EngineType } from './generated/EngineType';
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

// Import auto-generated location types from Rust
import type { Point } from './generated/Point';
import type { Location } from './generated/Location';

// Import auto-generated data stream types from Rust
import type { DataStream } from './generated/DataStream';
import type { StreamFormat } from './generated/StreamFormat';

// Import auto-generated payment types from Rust
import type { PaymentView } from './generated/PaymentView';
import type { PaymentType } from './generated/PaymentType';
import type { PaymentStatus } from './generated/PaymentStatus';
import type { CreateChargeRequest } from './generated/CreateChargeRequest';
import type { CheckoutResponse } from './generated/CheckoutResponse';
import type { StripeOnboardingResponse } from './generated/StripeOnboardingResponse';
import type { StripeConnectStatusView } from './generated/StripeConnectStatusView';
import type { StripeDashboardLinkResponse } from './generated/StripeDashboardLinkResponse';

// Import auto-generated club types from Rust
import type { CreateClubRequest } from './generated/CreateClubRequest';
import type { UpdateClubRequest } from './generated/UpdateClubRequest';

// Import auto-generated auth types from Rust
import type { LoginResponse } from './generated/LoginResponse';

// Import auto-generated flight types from Rust
import type { FlightGap } from './generated/FlightGap';
import type { PathPoint } from './generated/PathPoint';

// Import auto-generated aircraft image types from Rust
import type { AircraftImage } from './generated/AircraftImage';
import type { AircraftImageCollection } from './generated/AircraftImageCollection';
import type { AircraftImageSource } from './generated/AircraftImageSource';

// Import auto-generated receiver status and raw message view types from Rust
import type { ReceiverStatusView } from './generated/ReceiverStatusView';
import type { RawMessageView } from './generated/RawMessageView';

// Import auto-generated receiver alert types from Rust
import type { ReceiverAlertView } from './generated/ReceiverAlertView';
import type { UpsertReceiverAlertRequest } from './generated/UpsertReceiverAlertRequest';

// Import auto-generated receiver aggregate stats types from Rust
import type { AprsTypeCount } from './generated/AprsTypeCount';
import type { AircraftFixCount } from './generated/AircraftFixCount';
import type { ReceiverAggregateStatsResponse } from './generated/ReceiverAggregateStatsResponse';
import type { ReceiverStatisticsResponse } from './generated/ReceiverStatisticsResponse';

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

// Import auto-generated raw message types from Rust
import type { MessageSourceType } from './generated/MessageSourceType';
import type { RawMessageResponse } from './generated/RawMessageResponse';

// Import auto-generated watchlist types from Rust
import type { WatchlistEntry } from './generated/WatchlistEntry';

// Import auto-generated pagination types from Rust
import type { PaginationMetadata } from './generated/PaginationMetadata';

// Import auto-generated coverage types from Rust
import type { CoverageHexProperties } from './generated/CoverageHexProperties';
import type { CoverageHexFeature } from './generated/CoverageHexFeature';
import type { CoverageGeoJsonResponse } from './generated/CoverageGeoJsonResponse';
import type { FixesInHexResponse } from './generated/FixesInHexResponse';
import type { HexReceiversResponse } from './generated/HexReceiversResponse';

// Import auto-generated fix-with-aircraft type from Rust
import type { FixWithAircraftInfo } from './generated/FixWithAircraftInfo';

// Import auto-generated airspace types from Rust
import type { AirspaceGeoJson } from './generated/AirspaceGeoJson';
import type { AirspaceProperties } from './generated/AirspaceProperties';
import type { AirspaceClass } from './generated/AirspaceClass';
import type { AirspaceType as AirspaceTypeEnum } from './generated/AirspaceType';
import type { AirspaceSource } from './generated/AirspaceSource';

// Import auto-generated club view type from Rust
import type { ClubView } from './generated/ClubView';
import type { TowFeeView } from './generated/TowFeeView';

// Re-export all generated types for external use
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
	EngineType,
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
	ReverseGeocodeResponse,
	// Location types
	Point,
	Location,
	// Payment types
	PaymentView,
	PaymentType,
	PaymentStatus,
	CreateChargeRequest,
	CheckoutResponse,
	StripeOnboardingResponse,
	StripeConnectStatusView,
	StripeDashboardLinkResponse,
	ClubJoinRequestView,
	ClubView,
	TowFeeView,
	CreateClubRequest,
	UpdateClubRequest,
	// Auth types
	LoginResponse,
	// Flight types
	FlightGap,
	PathPoint,
	// Aircraft image types
	AircraftImage,
	AircraftImageCollection,
	AircraftImageSource,
	// Receiver types
	ReceiverStatusView,
	RawMessageView,
	AprsTypeCount,
	AircraftFixCount,
	ReceiverAggregateStatsResponse,
	ReceiverStatisticsResponse,
	// Receiver alert types
	ReceiverAlertView,
	UpsertReceiverAlertRequest,
	// Raw message types
	MessageSourceType,
	RawMessageResponse,
	// Watchlist types
	WatchlistEntry,
	// Pagination types
	PaginationMetadata,
	// Coverage types
	CoverageHexProperties,
	CoverageHexFeature,
	CoverageGeoJsonResponse,
	FixesInHexResponse,
	HexReceiversResponse,
	// Fix with aircraft
	FixWithAircraftInfo,
	// Airspace types
	AirspaceGeoJson,
	AirspaceProperties,
	AirspaceClass,
	AirspaceTypeEnum,
	AirspaceSource
};

// Type aliases for backward compatibility
export type Flight = FlightView;
export type User = UserView;
export type Receiver = ReceiverView;
export type Airport = AirportView;
export type Runway = RunwayView;
export type RunwayEnd = RunwayEndGenerated;
export type Club = ClubView;
export type Airspace = AirspaceGeoJson;
export type FixWithAircraft = FixWithAircraftInfo;

// Aircraft registration and model types (auto-generated from Rust via ts-rs)
export type AircraftRegistration = AircraftRegistrationView;
export type AircraftModel = AircraftModelView;

// API Response Wrapper Types (generic wrappers used across all endpoints)
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

export interface PaginatedDataResponse<T> {
	data: T[];
	metadata: PaginationMetadata;
}

// Airspace collection - GeoJSON FeatureCollection format
// (Constructed via serde_json::json!() in the backend, not a direct Rust struct)
export interface AirspaceFeatureCollection {
	type: 'FeatureCollection';
	features: Airspace[];
}

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

// === Frontend-only types (no Rust counterpart) ===

// Frontend extension: Club with computed isSoaring field
export type ClubWithSoaring = ClubView & {
	isSoaring?: boolean;
};

// UI component data for club selector combobox
export interface ComboboxData {
	label: string;
	value: string;
	club: ClubWithSoaring;
}

// AircraftWithRegistration extends Aircraft with optional registration and model details
export interface AircraftWithRegistration extends Aircraft {
	aircraftRegistration?: AircraftRegistration;
	aircraftModelDetails?: AircraftModel;
}

// Frontend-specific partial Aircraft with required ID
export type AircraftPartial = Partial<Aircraft> & {
	id: string;
};

// Extended Fix for WebSocket/legacy responses with extra fields not in the generated Rust Fix type
export interface FixWithExtras extends Fix {
	deviceAddressHex?: string;
	registration?: string;
	model?: string;
	aprsType?: string;
	via?: string[];
}

// FlightDetails combines Flight with full Aircraft data (frontend composition)
export interface FlightDetails {
	flight: Flight;
	aircraft: Aircraft | null;
}

// Watchlist entry with joined aircraft data (frontend composition)
export interface WatchlistEntryWithAircraft extends WatchlistEntry {
	aircraft?: Aircraft;
}

// Browser API extension: DeviceOrientationEvent with iOS-specific compass heading
export interface DeviceOrientationEventWithCompass extends DeviceOrientationEvent {
	webkitCompassHeading?: number;
}
