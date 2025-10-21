//! # Flydent
//!
//! A Rust library for parsing aircraft registration callsigns and ICAO 24-bit identifiers.
//! This is a port of the Python flydenity library that identifies countries and organizations
//! from aircraft callsigns using ITU data.
//!
//! ## Features
//!
//! - Parse aircraft callsigns (e.g., "T6ABC" -> Afghanistan)
//! - Parse ICAO 24-bit identifiers (e.g., "700123" -> Afghanistan)
//! - Identify countries with ISO codes and organizations
//! - Compile-time CSV parsing for zero-runtime overhead
//! - No external CSV files required at runtime
//!
//! ## Usage
//!
//! ```rust
//! use flydent::{Parser, EntityResult};
//!
//! let parser = Parser::new();
//!
//! // Parse a callsign
//! if let Some(result) = parser.parse_simple("T6ABC") {
//!     match result {
//!         EntityResult::Country { nation, iso2, .. } => {
//!             println!("Country: {} ({})", nation, iso2);
//!         }
//!         EntityResult::Organization { name, .. } => {
//!             println!("Organization: {}", name);
//!         }
//!     }
//! }
//!
//! // Parse ICAO 24-bit identifier
//! if let Some(result) = parser.parse("700123", false, true) {
//!     println!("ICAO identifier parsed: {:?}", result);
//! }
//! ```

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

pub mod registration;

fn normalize_dashes(input: &str) -> String {
    // Convert common non-ASCII dash characters to ASCII hyphen-minus
    input
        .replace('–', "-")  // en dash (U+2013)
        .replace('—', "-")  // em dash (U+2014)
        .replace('−', "-")  // minus sign (U+2212)
}

fn generate_canonical_form(input: &str, iso2: &str, callsign_prefixes: &[String]) -> String {
    // Countries where canonical form has NO dash between prefix and suffix
    let no_dash_countries = [
        "US", "JP", "KR", "TW", "CN", "RU", "BY", "UA", "KZ", "UZ", "KG", "TJ", "TM", "AM", "AZ",
        "GE", "MD",
    ];

    if no_dash_countries.contains(&iso2) {
        // These countries canonicalize without dashes
        input.replace("-", "")
    } else {
        // Other countries (including Canada) canonicalize with dashes after prefix
        // Find the right place to insert dashes based on callsign prefixes
        for prefix in callsign_prefixes {
            if input.starts_with(prefix) && !input.contains("-") {
                // Input matches prefix but doesn't have dash, add one after prefix
                let suffix = &input[prefix.len()..];
                if !suffix.is_empty() {
                    return format!("{}-{}", prefix, suffix);
                }
            } else if input.starts_with(&format!("{}-", prefix)) {
                // Already has dash in right place
                return input.to_string();
            }
        }
        input.to_string()
    }
}

#[derive(Debug, Clone)]
pub enum EntityResult {
    Country {
        nation: String,
        description: String,
        iso2: String,
        iso3: String,
        canonical_callsign: String,
    },
    Organization {
        name: String,
        description: String,
        canonical_callsign: String,
    },
}

impl EntityResult {
    pub fn canonical_callsign(&self) -> &String {
        match self {
            EntityResult::Country { canonical_callsign, .. } => canonical_callsign,
            EntityResult::Organization { canonical_callsign, .. } => canonical_callsign,
        }
    }
}

#[derive(Debug, Clone)]
struct EntityData {
    entity_result: EntityResult,
    priority: i32,
    callsigns: Vec<String>,
    regex: String,
    strict_regex: String,
    icao24bit_prefixes: Vec<String>,
}

fn parse_python_list(s: &str) -> Vec<String> {
    if s.starts_with('[') && s.ends_with(']') {
        let inner = &s[1..s.len() - 1];
        if inner.is_empty() {
            Vec::new()
        } else {
            inner
                .split(", ")
                .map(|item| {
                    let item = item.trim();
                    if (item.starts_with('\'') && item.ends_with('\''))
                        || (item.starts_with('"') && item.ends_with('"'))
                    {
                        item[1..item.len() - 1].to_string()
                    } else {
                        item.to_string()
                    }
                })
                .collect()
        }
    } else {
        Vec::new()
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    // Escaped quote
                    current_field.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current_field.trim().to_string());
                current_field.clear();
            }
            _ => current_field.push(ch),
        }
    }
    fields.push(current_field.trim().to_string());
    fields
}

macro_rules! build_data {
    () => {{
        let mut all_data = Vec::new();

        // Parse countries
        let countries_csv = include_str!("../data/processed_itu_countries_regex.csv");
        let mut lines = countries_csv.lines();
        let _header = lines.next().unwrap(); // Skip header

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let fields = parse_csv_line(line);
            if fields.len() >= 10 {
                let nation = fields[0].clone();
                let description = fields[1].clone();
                let priority: i32 = fields[2].parse().unwrap_or(0);
                let iso_codes = parse_python_list(&fields[3]);
                let callsigns = parse_python_list(&fields[4]);
                let regex_str = fields[6].clone();
                let icao24bit_prefixes = parse_python_list(&fields[9]);

                let iso2 = iso_codes.get(0).cloned().unwrap_or_default();
                let iso3 = iso_codes.get(1).cloned().unwrap_or_default();

                let strict_regex_str = regex_str.replace("-{0,1}", "\\-").replace("{0,1}$", "$");

                all_data.push(EntityData {
                    entity_result: EntityResult::Country {
                        nation,
                        description,
                        iso2,
                        iso3,
                        canonical_callsign: String::new(), // Placeholder, will be filled during parsing
                    },
                    priority,
                    callsigns,
                    regex: regex_str,
                    strict_regex: strict_regex_str,
                    icao24bit_prefixes,
                });
            }
        }

        // Parse organizations
        let orgs_csv = include_str!("../data/processed_itu_organizations_regex.csv");
        let mut lines = orgs_csv.lines();
        let _header = lines.next().unwrap(); // Skip header

        for line in lines {
            if line.trim().is_empty() {
                continue;
            }

            let fields = parse_csv_line(line);
            if fields.len() >= 9 {
                let name = fields[0].clone();
                let description = fields[1].clone();
                let priority: i32 = fields[2].parse().unwrap_or(0);
                let callsigns = parse_python_list(&fields[3]);
                let regex_str = fields[5].clone();
                let icao24bit_prefixes = parse_python_list(&fields[8]);

                let strict_regex_str = regex_str.replace("-{0,1}", "\\-").replace("{0,1}$", "$");

                all_data.push(EntityData {
                    entity_result: EntityResult::Organization {
                        name,
                        description,
                        canonical_callsign: String::new(), // Placeholder, will be filled during parsing
                    },
                    priority,
                    callsigns,
                    regex: regex_str,
                    strict_regex: strict_regex_str,
                    icao24bit_prefixes,
                });
            }
        }

        all_data
    }};
}

static DATA: Lazy<Vec<EntityData>> = Lazy::new(|| build_data!());

static CALLSIGNS_MAP: Lazy<HashMap<String, Vec<usize>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (i, data) in DATA.iter().enumerate() {
        for callsign in &data.callsigns {
            map.entry(callsign.clone()).or_insert_with(Vec::new).push(i);
        }
    }
    map
});

static ICAO_MAP: Lazy<HashMap<String, usize>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for (i, data) in DATA.iter().enumerate() {
        for prefix in &data.icao24bit_prefixes {
            map.insert(prefix.clone(), i);
        }
    }
    map
});

static MIN_CALLSIGN_LEN: Lazy<usize> =
    Lazy::new(|| CALLSIGNS_MAP.keys().map(|k| k.len()).min().unwrap_or(0));

static MAX_CALLSIGN_LEN: Lazy<usize> =
    Lazy::new(|| CALLSIGNS_MAP.keys().map(|k| k.len()).max().unwrap_or(0));

pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Self
    }

    fn parse_registration(&self, input: &str, strict: bool) -> Option<Vec<&EntityData>> {
        let mut datasets = Vec::new();

        for callsign_len in *MIN_CALLSIGN_LEN..=*MAX_CALLSIGN_LEN {
            if input.len() >= callsign_len {
                let prefix = &input[0..callsign_len];
                if let Some(indices) = CALLSIGNS_MAP.get(prefix) {
                    for &idx in indices {
                        datasets.push(&DATA[idx]);
                    }
                }
            }
        }

        if datasets.is_empty() {
            return None;
        }

        let mut matches_by_priority: HashMap<i32, Vec<&EntityData>> = HashMap::new();

        for data in datasets {
            let regex_str = if strict {
                &data.strict_regex
            } else {
                &data.regex
            };

            if let Ok(regex) = Regex::new(regex_str) {
                if regex.is_match(input) {
                    matches_by_priority
                        .entry(data.priority)
                        .or_default()
                        .push(data);
                }
            }
        }

        if let Some(max_priority) = matches_by_priority.keys().max() {
            matches_by_priority.get(max_priority).cloned()
        } else {
            None
        }
    }

    fn parse_icao24bit(&self, input: &str, strict: bool) -> Option<Vec<&EntityData>> {
        if strict && !Regex::new(r"^[0-9A-F]{6}$").unwrap().is_match(input) {
            eprintln!(
                "Warning: ICAO 24bit '{}' must be hexadecimal with length of 6 chars",
                input
            );
            return None;
        }

        let mut matches = Vec::new();

        for i in 0..input.len() {
            let prefix = &input[0..=i];
            if let Some(&idx) = ICAO_MAP.get(prefix) {
                matches.push(&DATA[idx]);
            }
        }

        if matches.is_empty() {
            None
        } else {
            Some(matches)
        }
    }

    pub fn parse(&self, input: &str, strict: bool, icao24bit: bool) -> Option<EntityResult> {
        let normalized_input = normalize_dashes(input);

        if icao24bit {
            if let Some(matches) = self.parse_icao24bit(&normalized_input, strict) {
                matches.first().map(|data| {
                    match &data.entity_result {
                        EntityResult::Country {
                            nation,
                            description,
                            iso2,
                            iso3,
                            ..
                        } => {
                            let canonical = generate_canonical_form(&normalized_input, iso2, &data.callsigns);
                            EntityResult::Country {
                                nation: nation.clone(),
                                description: description.clone(),
                                iso2: iso2.clone(),
                                iso3: iso3.clone(),
                                canonical_callsign: canonical,
                            }
                        }
                        EntityResult::Organization {
                            name, description, ..
                        } => {
                            let canonical = normalized_input.to_string(); // Organizations keep normalized format
                            EntityResult::Organization {
                                name: name.clone(),
                                description: description.clone(),
                                canonical_callsign: canonical,
                            }
                        }
                    }
                })
            } else {
                None
            }
        } else if let Some(matches) = self.parse_registration(&normalized_input, strict) {
            matches.first().map(|data| {
                match &data.entity_result {
                    EntityResult::Country {
                        nation,
                        description,
                        iso2,
                        iso3,
                        ..
                    } => {
                        let canonical = generate_canonical_form(&normalized_input, iso2, &data.callsigns);
                        EntityResult::Country {
                            nation: nation.clone(),
                            description: description.clone(),
                            iso2: iso2.clone(),
                            iso3: iso3.clone(),
                            canonical_callsign: canonical,
                        }
                    }
                    EntityResult::Organization {
                        name, description, ..
                    } => {
                        let canonical = normalized_input.to_string(); // Organizations keep normalized format
                        EntityResult::Organization {
                            name: name.clone(),
                            description: description.clone(),
                            canonical_callsign: canonical,
                        }
                    }
                }
            })
        } else {
            None
        }
    }

    pub fn parse_simple(&self, input: &str) -> Option<EntityResult> {
        self.parse(input, false, false)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let _parser = Parser::new();
        assert!(*MIN_CALLSIGN_LEN > 0);
        assert!(*MAX_CALLSIGN_LEN >= *MIN_CALLSIGN_LEN);
        assert!(!DATA.is_empty());
        assert!(!CALLSIGNS_MAP.is_empty());
    }

    #[test]
    fn test_parse_csv_line() {
        let line = r#"Afghanistan,general,0,"['AF', 'AFG']","['T6', 'YA']",['AAA-ZZZ'],"^(T6|YA)(-{0,1}([A-Z]{3}|[A-Z0-9]{1,4})){0,1}$",700000,700FFF,['700']"#;
        let fields = parse_csv_line(line);
        assert_eq!(fields[0], "Afghanistan");
        assert_eq!(fields[3], "['AF', 'AFG']");
        assert_eq!(fields[4], "['T6', 'YA']");
    }

    #[test]
    fn test_parse_python_list() {
        let result = parse_python_list("['T6', 'YA']");
        assert_eq!(result, vec!["T6", "YA"]);

        let result = parse_python_list("['700']");
        assert_eq!(result, vec!["700"]);
    }

    #[test]
    fn test_parse_simple() {
        let parser = Parser::new();

        // Test with a known callsign prefix
        if let Some(result) = parser.parse_simple("T6ABC") {
            match result {
                EntityResult::Country {
                    nation,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "Afghanistan");
                    assert_eq!(canonical_callsign, "T6-ABC"); // Afghanistan uses dashes in canonical form
                }
                _ => panic!("Expected country result for T6ABC"),
            }
        } else {
            panic!("T6ABC should match Afghanistan");
        }
    }

    #[test]
    fn test_comprehensive_parsing() {
        let parser = Parser::new();

        // Test Afghanistan callsign T6ABC
        if let Some(result) = parser.parse("T6ABC", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    description,
                    iso2,
                    iso3,
                    canonical_callsign,
                } => {
                    assert_eq!(nation, "Afghanistan");
                    assert_eq!(description, "general");
                    assert_eq!(iso2, "AF");
                    assert_eq!(iso3, "AFG");
                    assert_eq!(canonical_callsign, "T6-ABC"); // Afghanistan uses dashes in canonical form
                }
                _ => panic!("Expected country result for T6ABC"),
            }
        } else {
            panic!("T6ABC should match Afghanistan");
        }

        // Test organization callsign 4Y123
        if let Some(result) = parser.parse("4Y123", false, false) {
            match result {
                EntityResult::Organization {
                    name,
                    description,
                    canonical_callsign,
                } => {
                    assert_eq!(name, "International Civil Aviation Organization");
                    assert_eq!(description, "general");
                    assert_eq!(canonical_callsign, "4Y123");
                }
                _ => panic!("Expected organization result for 4Y123"),
            }
        } else {
            panic!("4Y123 should match ICAO");
        }

        // Test ICAO 24-bit identifier 700123
        if let Some(result) = parser.parse("700123", false, true) {
            match result {
                EntityResult::Country {
                    nation,
                    description,
                    iso2,
                    iso3,
                    canonical_callsign,
                } => {
                    assert_eq!(nation, "Afghanistan");
                    assert_eq!(description, "general");
                    assert_eq!(iso2, "AF");
                    assert_eq!(iso3, "AFG");
                    assert_eq!(canonical_callsign, "700123");
                }
                _ => panic!("Expected country result for ICAO 700123"),
            }
        } else {
            panic!("ICAO 700123 should match Afghanistan");
        }

        // Test non-existent callsign should return None
        assert!(parser.parse("N123ABC", false, false).is_none());
    }

    #[test]
    fn test_german_callsign_d_ekqm() {
        let parser = Parser::new();

        // Test German callsign D-EKQM (with ASCII dash)
        if let Some(result) = parser.parse("D-EKQM", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    iso2,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "Germany");
                    assert_eq!(iso2, "DE");
                    assert_eq!(canonical_callsign, "D-EKQM"); // Germany uses dashes in canonical form
                }
                _ => panic!("Expected country result for D-EKQM"),
            }
        } else {
            panic!("D-EKQM should match Germany");
        }

        // Test German callsign D–EKQM (with en-dash) - should be normalized to ASCII dash
        if let Some(result) = parser.parse("D–EKQM", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    iso2,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "Germany");
                    assert_eq!(iso2, "DE");
                    assert_eq!(canonical_callsign, "D-EKQM"); // Normalized to ASCII dash
                }
                _ => panic!("Expected country result for D–EKQM"),
            }
        } else {
            panic!("D–EKQM should match Germany after normalization");
        }

        // Test without dash - should add dash for German canonical form
        if let Some(result) = parser.parse("DEKQM", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "Germany");
                    assert_eq!(canonical_callsign, "D-EKQM"); // Dash added for German canonical form
                }
                _ => panic!("Expected country result for DEKQM"),
            }
        } else {
            panic!("DEKQM should match Germany");
        }
    }

    #[test]
    fn test_canonical_form_by_country() {
        let parser = Parser::new();

        // Test US callsign: canonical form removes dashes
        if let Some(result) = parser.parse("N-8437D", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "United States");
                    assert_eq!(canonical_callsign, "N8437D"); // Dash removed for US canonical form
                }
                _ => panic!("Expected country result for N-8437D"),
            }
        } else {
            panic!("N-8437D should match United States");
        }

        // Test US callsign without dash should stay the same
        if let Some(result) = parser.parse("N8437D", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "United States");
                    assert_eq!(canonical_callsign, "N8437D"); // Already canonical
                }
                _ => panic!("Expected country result for N8437D"),
            }
        } else {
            panic!("N8437D should match United States");
        }

        // Test Canadian callsign: canonical form includes dash after prefix
        if let Some(result) = parser.parse("CFAAA", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "Canada");
                    assert_eq!(canonical_callsign, "C-FAAA"); // Dash added for Canadian canonical form
                }
                _ => panic!("Expected country result for CFAAA"),
            }
        } else {
            panic!("CFAAA should match Canada");
        }

        // Test Canadian callsign with dash should stay the same
        if let Some(result) = parser.parse("C-FAAA", false, false) {
            match result {
                EntityResult::Country {
                    nation,
                    canonical_callsign,
                    ..
                } => {
                    assert_eq!(nation, "Canada");
                    assert_eq!(canonical_callsign, "C-FAAA"); // Already canonical
                }
                _ => panic!("Expected country result for C-FAAA"),
            }
        } else {
            panic!("C-FAAA should match Canada");
        }
    }
}
