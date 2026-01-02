// Re-export all generated types
export type { Aircraft } from './Aircraft';
export type { AircraftType } from './AircraftType';
export type { AircraftView } from './AircraftView';
export type { ClubView } from './ClubView';
export type { Fix } from './Fix';
export type { TowFeeView } from './TowFeeView';

// Define JsonValue type that ts-rs expects
// This represents a serde_json::Value from Rust
export type JsonValue =
	| string
	| number
	| boolean
	| null
	| JsonValue[]
	| { [key: string]: JsonValue };
