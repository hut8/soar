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
	AircraftType
};

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

export interface RunwayEnd {
	ident: string | null;
	latitudeDeg: number | null;
	longitudeDeg: number | null;
	elevationFt: number | null;
	headingDegt: number | null;
	displacedThresholdFt: number | null;
}

export interface Runway {
	id: number;
	lengthFt: number | null;
	widthFt: number | null;
	surface: string | null;
	lighted: boolean;
	closed: boolean;
	low: RunwayEnd;
	high: RunwayEnd;
}

export interface Airport {
	id: number;
	ident: string;
	airportType: string;
	name: string;
	latitudeDeg: string | null; // BigDecimal serialized as string
	longitudeDeg: string | null; // BigDecimal serialized as string
	elevationFt: number | null;
	continent: string | null;
	isoCountry: string | null;
	isoRegion: string | null;
	municipality: string | null;
	scheduledService: boolean;
	icaoCode: string | null;
	iataCode: string | null;
	gpsCode: string | null;
	localCode: string | null;
	homeLink: string | null;
	wikipediaLink: string | null;
	keywords: string | null;
	runways: Runway[];
}

// Aircraft registration information (from FAA database)
export interface AircraftRegistration {
	registrationNumber: string;
	serialNumber: string;
	manufacturerCode?: string;
	modelCode?: string;
	seriesCode?: string;
	engineManufacturerCode?: string;
	engineModelCode?: string;
	yearManufactured?: number;
	registrantType?: string;
	registrantName?: string;
	aircraftType?: string;
	engineType?: number;
	statusCode?: string;
	transponderCode?: number;
	airworthinessClass?: string;
	airworthinessDate?: string;
	certificateIssueDate?: string;
	expirationDate?: string;
	clubId?: string;
	homeBaseAirportId?: string;
	kitManufacturerName?: string;
	kitModelName?: string;
	otherNames?: string[];
	lightSportType?: string;
	aircraftId?: string;
	model?: AircraftModel; // Embedded model data if available
	aircraftTypeOgn?: string; // OGN aircraft type if available
}

// Aircraft model information
export interface AircraftModel {
	manufacturerCode: string;
	modelCode: string;
	seriesCode: string;
	manufacturerName: string;
	modelName: string;
	aircraftType: string | null;
	engineType: string | null;
	aircraftCategory: string | null;
	builderCertification: string | null;
	numberOfEngines: number | null;
	numberOfSeats: number | null;
	weightClass: string | null;
	cruisingSpeed: number | null;
	typeCertificateDataSheet: string | null;
	typeCertificateDataHolder: string | null;
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
	rawPacket?: string; // Raw APRS packet data (joined from aprs_messages table)
	flight?: Flight; // Full flight information if part of an active flight (from websocket)
}

// User authentication and profile (now includes pilot fields)
export interface User {
	id: string;
	firstName: string;
	lastName: string;
	email?: string | null; // Nullable - pilots without login don't have email
	isAdmin: boolean;
	clubId?: string;
	emailVerified: boolean;
	createdAt: string;
	updatedAt: string;
	settings: Record<string, unknown>;
	// Pilot qualification fields
	isLicensed: boolean;
	isInstructor: boolean;
	isTowPilot: boolean;
	isExaminer: boolean;
	// Derived fields
	canLogin: boolean; // True if user has email and password
	isPilot: boolean; // True if any pilot qualification is true
}

// Flight interface matching backend FlightView
export interface Flight {
	id: string;
	aircraftId?: string; // UUID foreign key to aircraft table
	deviceAddress: string; // Hex format like "39D304"
	deviceAddressType: string; // F, O, I, or empty string
	takeoffTime?: string; // ISO datetime string - null for flights first seen airborne
	landingTime?: string; // ISO datetime string - null for flights in progress
	timedOutAt?: string; // ISO datetime string when flight timed out
	state: 'active' | 'complete' | 'timed_out'; // Flight state
	durationSeconds?: number; // Duration in seconds (null if takeoffTime or landingTime is null)
	departureAirport?: string; // Airport identifier
	departureAirportId?: number; // Airport ID in database
	departureAirportCountry?: string; // Country code
	arrivalAirport?: string; // Airport identifier
	arrivalAirportId?: number; // Airport ID in database
	arrivalAirportCountry?: string; // Country code
	// Geocoded location for flight start
	startLocationCity?: string;
	startLocationState?: string;
	startLocationCountry?: string;
	// Geocoded location for flight end
	endLocationCity?: string;
	endLocationState?: string;
	endLocationCountry?: string;
	towedByAircraftId?: string; // UUID of towplane aircraft that towed this glider
	towedByFlightId?: string; // UUID of towplane flight that towed this glider
	clubId?: string; // UUID of club that owns the aircraft
	takeoffAltitudeOffsetFt?: number; // Altitude offset at takeoff
	landingAltitudeOffsetFt?: number; // Altitude offset at landing
	takeoffRunwayIdent?: string; // Takeoff runway identifier
	landingRunwayIdent?: string; // Landing runway identifier
	totalDistanceMeters?: number; // Total distance flown in meters
	maximumDisplacementMeters?: number; // Maximum displacement from takeoff point
	runwaysInferred?: boolean; // Whether runways were inferred from heading vs matched to airport data
	createdAt: string; // ISO datetime string
	updatedAt: string; // ISO datetime string
	// Aircraft information (merged into FlightView from AircraftInfo)
	aircraftModel?: string;
	registration?: string;
	aircraftTypeOgn?: string;
	aircraftCountryCode?: string;
	// Latest fix information (for active flights)
	latestAltitudeMslFeet: number | null;
	latestAltitudeAglFeet: number | null;
	latestFixTimestamp: string | null;
	// Navigation to previous/next flights for the same aircraft (chronologically by takeoff time)
	previousFlightId?: string;
	nextFlightId?: string;
	// Flight callsign (from APRS packets)
	callsign?: string;
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

// Receiver interface matching backend ReceiverView
export interface Receiver {
	id: string;
	callsign: string;
	description: string | null;
	contact: string | null;
	email: string | null;
	ognDbCountry: string | null;
	latitude: number | null;
	longitude: number | null;
	streetAddress: string | null;
	city: string | null;
	region: string | null;
	country: string | null;
	postalCode: string | null;
	createdAt: string;
	updatedAt: string;
	latestPacketAt: string | null;
	fromOgnDb: boolean;
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
	receiverId: string;
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
