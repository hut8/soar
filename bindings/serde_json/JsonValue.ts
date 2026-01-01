// Type definition for Rust's serde_json::Value
// This represents any valid JSON value
export type JsonValue =
	| string
	| number
	| boolean
	| null
	| JsonValue[]
	| { [key: string]: JsonValue };
