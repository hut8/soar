//! Normalize aircraft manufacturer names from various data sources.
//!
//! Data sources (FAA registry, ADS-B Exchange, FlarmNet, OGN DDB) use
//! inconsistent corporate legal names like "THE BOEING COMPANY" or
//! "CESSNA AIRCRAFT CO". This module maps them to clean canonical forms.

use std::sync::LazyLock;

/// A mapping entry: a case-folded prefix to match and the canonical replacement.
struct ManufacturerMapping {
    /// Uppercase pattern to match (must be stored uppercase for case-insensitive comparison)
    pattern: &'static str,
    /// The clean canonical name to use as a replacement
    canonical: &'static str,
}

/// Sorted longest-first so greedy prefix matching works correctly.
/// For example, "CESSNA AIRCRAFT INC" must be checked before "CESSNA AIRCRAFT".
static MANUFACTURER_MAPPINGS: LazyLock<Vec<ManufacturerMapping>> = LazyLock::new(|| {
    let mut mappings = vec![
        // Boeing
        m("THE BOEING COMPANY", "Boeing"),
        m("BOEING CO THE", "Boeing"),
        m("BOEING COMPANY", "Boeing"),
        m("BOEING CO", "Boeing"),
        m("BOEING", "Boeing"),
        // Cessna
        m("CESSNA AIRCRAFT INC", "Cessna"),
        m("CESSNA AIRCRAFT CO", "Cessna"),
        m("CESSNA AIRCRAFT", "Cessna"),
        m("CESSNA", "Cessna"),
        // Piper
        m("NEW PIPER AIRCRAFT INC", "Piper"),
        m("NEW PIPER AIRCRAFT", "Piper"),
        m("PIPER AIRCRAFT CORP", "Piper"),
        m("PIPER AIRCRAFT INC", "Piper"),
        m("PIPER AIRCRAFT", "Piper"),
        m("NEW PIPER", "Piper"),
        m("PIPER", "Piper"),
        // Beechcraft
        m("HAWKER BEECHCRAFT CORP", "Beechcraft"),
        m("BEECHCRAFT AIRCRAFT CORP", "Beechcraft"),
        m("BEECHCRAFT CORP", "Beechcraft"),
        m("BEECH AIRCRAFT CORP", "Beechcraft"),
        m("BEECHCRAFT", "Beechcraft"),
        m("BEECH", "Beechcraft"),
        // Airbus
        m("AIRBUS HELICOPTERS", "Airbus Helicopters"),
        m("AIRBUS INDUSTRIE", "Airbus"),
        m("AIRBUS S A S", "Airbus"),
        m("AIRBUS SAS", "Airbus"),
        m("AIRBUS", "Airbus"),
        // Cirrus
        m("CIRRUS DESIGN CORPORATION", "Cirrus"),
        m("CIRRUS DESIGN CORP", "Cirrus"),
        m("CIRRUS", "Cirrus"),
        // Bell
        m("BELL HELICOPTER TEXTRON", "Bell"),
        m("BELL TEXTRON INC", "Bell"),
        m("BELL", "Bell"),
        // Robinson
        m("ROBINSON HELICOPTER COMPANY", "Robinson"),
        m("ROBINSON HELICOPTER CO", "Robinson"),
        m("ROBINSON HELICOPTER", "Robinson"),
        m("ROBINSON", "Robinson"),
        // Mooney
        m("MOONEY INTERNATIONAL CORP", "Mooney"),
        m("MOONEY AIRCRAFT CORP", "Mooney"),
        m("MOONEY", "Mooney"),
        // Textron Aviation
        m("TEXTRON AVIATION INC", "Textron Aviation"),
        m("TEXTRON AVIATION", "Textron Aviation"),
        // Bombardier
        m("BOMBARDIER INC CANADAIR", "Bombardier"),
        m("BOMBARDIER AEROSPACE", "Bombardier"),
        m("BOMBARDIER INC", "Bombardier"),
        m("BOMBARDIER", "Bombardier"),
        // Gulfstream
        m("GULFSTREAM AEROSPACE CORP", "Gulfstream"),
        m("GULFSTREAM AEROSPACE", "Gulfstream"),
        m("GULFSTREAM", "Gulfstream"),
        // Grumman / Northrop Grumman
        m("NORTHROP GRUMMAN", "Northrop Grumman"),
        m("GRUMMAN AMERICAN AVN CORP", "Grumman"),
        m("GRUMMAN AIRCRAFT ENG CORP", "Grumman"),
        m("GRUMMAN", "Grumman"),
        // Embraer
        m("EMBRAER EMPRESA BRASILEIRA DE", "Embraer"),
        m("EMBRAER S A", "Embraer"),
        m("EMBRAER SA", "Embraer"),
        m("EMBRAER", "Embraer"),
        // Diamond
        m("DIAMOND AIRCRAFT INDUSTRIES", "Diamond"),
        m("DIAMOND AIRCRAFT IND", "Diamond"),
        m("DIAMOND AIRCRAFT", "Diamond"),
        m("DIAMOND", "Diamond"),
        // Maule
        m("MAULE AEROSPACE TECHNOLOGY INC", "Maule"),
        m("MAULE AEROSPACE TECHNOLOGY", "Maule"),
        m("MAULE AIR INC", "Maule"),
        m("MAULE", "Maule"),
        // Socata
        m("SOCATA GROUP AEROSPATIALE", "Socata"),
        m("EADS SOCATA", "Socata"),
        m("SOCATA", "Socata"),
        // Pilatus
        m("PILATUS FLUGZEUGWERKE AG", "Pilatus"),
        m("PILATUS AIRCRAFT LTD", "Pilatus"),
        m("PILATUS", "Pilatus"),
        // Dassault
        m("AVIONS MARCEL DASSAULT", "Dassault"),
        m("DASSAULT BREGUET", "Dassault"),
        m("DASSAULT FALCON", "Dassault"),
        m("DASSAULT AVIATION", "Dassault"),
        m("DASSAULT", "Dassault"),
        // Sikorsky
        m("SIKORSKY AIRCRAFT CORP", "Sikorsky"),
        m("SIKORSKY AIRCRAFT", "Sikorsky"),
        m("SIKORSKY", "Sikorsky"),
        // Raytheon
        m("RAYTHEON AIRCRAFT COMPANY", "Raytheon"),
        m("RAYTHEON AIRCRAFT CO", "Raytheon"),
        m("RAYTHEON", "Raytheon"),
        // McDonnell Douglas
        m("MCDONNELL DOUGLAS HELICOPTER", "McDonnell Douglas"),
        m("MCDONNELL DOUGLAS CORP", "McDonnell Douglas"),
        m("MCDONNELL DOUGLAS", "McDonnell Douglas"),
        // Lockheed Martin
        m("LOCKHEED MARTIN CORP", "Lockheed Martin"),
        m("LOCKHEED MARTIN", "Lockheed Martin"),
        m("LOCKHEED", "Lockheed"),
        // de Havilland
        m("DE HAVILLAND AIRCRAFT OF CANADA", "De Havilland Canada"),
        m("DE HAVILLAND CANADA", "De Havilland Canada"),
        m("DE HAVILLAND", "De Havilland"),
        // Eurocopter / Aerospatiale
        m("EUROCOPTER FRANCE", "Eurocopter"),
        m("EUROCOPTER DEUTSCHLAND", "Eurocopter"),
        m("EUROCOPTER", "Eurocopter"),
        m("AEROSPATIALE", "Aerospatiale"),
        // Learjet
        m("LEARJET INC", "Learjet"),
        m("LEARJET", "Learjet"),
        // Fairchild
        m("FAIRCHILD INDUSTRIES INC", "Fairchild"),
        m("FAIRCHILD AIRCRAFT INC", "Fairchild"),
        m("FAIRCHILD", "Fairchild"),
        // American Champion
        m("AMERICAN CHAMPION AIRCRAFT CORP", "American Champion"),
        m("AMERICAN CHAMPION AIRCRAFT", "American Champion"),
        m("AMERICAN CHAMPION", "American Champion"),
        // Extra
        m("EXTRA FLUGZEUGBAU GMBH", "Extra"),
        m("EXTRA FLUGZEUGPRODUKTIONS", "Extra"),
        // Schempp-Hirth (gliders)
        m("SCHEMPP-HIRTH FLUGZEUGBAU GMBH", "Schempp-Hirth"),
        m("SCHEMPP-HIRTH", "Schempp-Hirth"),
        // Schleicher (gliders)
        m("ALEXANDER SCHLEICHER GMBH", "Schleicher"),
        m("ALEXANDER SCHLEICHER", "Schleicher"),
        // DG Flugzeugbau (gliders)
        m("DG FLUGZEUGBAU GMBH", "DG Flugzeugbau"),
        m("DG FLUGZEUGBAU", "DG Flugzeugbau"),
        // Rolladen-Schneider (gliders)
        m("ROLLADEN-SCHNEIDER FLUGZEUGBAU", "Rolladen-Schneider"),
        m("ROLLADEN-SCHNEIDER", "Rolladen-Schneider"),
    ];

    // Sort by pattern length descending so longest match wins
    mappings.sort_by(|a, b| b.pattern.len().cmp(&a.pattern.len()));
    mappings
});

fn m(pattern: &'static str, canonical: &'static str) -> ManufacturerMapping {
    ManufacturerMapping { pattern, canonical }
}

/// Corporate suffixes to strip when no explicit mapping matches.
const CORPORATE_SUFFIXES: &[&str] = &[
    " INCORPORATED",
    " CORPORATION",
    " INDUSTRIES",
    " COMPANY",
    " AIRCRAFT",
    " CORP",
    " INC",
    " LLC",
    " LTD",
    " CO",
];

/// Title-case a string: capitalize the first letter of each word.
fn title_case(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut result = String::with_capacity(lower.len());
    let mut capitalize_next = true;

    for ch in lower.chars() {
        if ch == ' ' || ch == '-' {
            result.push(ch);
            capitalize_next = true;
        } else if capitalize_next {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

/// Normalize a standalone manufacturer name.
///
/// Used for fields that contain only the manufacturer (e.g. FAA `manufacturer_name`,
/// ADS-B Exchange `manufacturer`).
///
/// 1. Checks against the known manufacturer mapping (case-insensitive exact match)
/// 2. Strips common corporate suffixes
/// 3. Title-cases the result
///
/// # Examples
///
/// ```
/// use soar::manufacturer_names::normalize_manufacturer;
///
/// assert_eq!(normalize_manufacturer("THE BOEING COMPANY"), "Boeing");
/// assert_eq!(normalize_manufacturer("CESSNA AIRCRAFT CO"), "Cessna");
/// assert_eq!(normalize_manufacturer("ACME AVIATION INC"), "Acme Aviation");
/// ```
pub fn normalize_manufacturer(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let upper = trimmed.to_uppercase();

    // Check for exact match against known manufacturers
    for mapping in MANUFACTURER_MAPPINGS.iter() {
        if upper == mapping.pattern {
            return mapping.canonical.to_string();
        }
    }

    // No exact match — strip corporate suffixes and title-case
    let mut cleaned = upper.clone();
    // Strip leading "THE "
    if cleaned.starts_with("THE ") {
        cleaned = cleaned[4..].to_string();
    }
    // Strip trailing corporate suffixes (may need multiple passes)
    loop {
        let mut stripped = false;
        for suffix in CORPORATE_SUFFIXES {
            if cleaned.ends_with(suffix) {
                cleaned.truncate(cleaned.len() - suffix.len());
                stripped = true;
                break;
            }
        }
        if !stripped {
            break;
        }
    }

    let cleaned = cleaned.trim();
    if cleaned.is_empty() {
        return title_case(trimmed);
    }

    title_case(cleaned)
}

/// Normalize a combined "manufacturer model" string.
///
/// Used for fields like `aircraft.aircraft_model` that contain both the manufacturer
/// and model number (e.g. "THE BOEING COMPANY 737-8").
///
/// 1. Checks if the string starts with any known manufacturer pattern (case-insensitive)
/// 2. Replaces the manufacturer prefix with the canonical name
/// 3. Returns the cleaned string
///
/// Strings that don't match any known manufacturer prefix are returned as-is
/// (e.g. glider types like "ASW-27" or "Discus-2").
///
/// # Examples
///
/// ```
/// use soar::manufacturer_names::normalize_aircraft_model;
///
/// assert_eq!(
///     normalize_aircraft_model("THE BOEING COMPANY 737-8"),
///     "Boeing 737-8"
/// );
/// assert_eq!(normalize_aircraft_model("ASW-27"), "ASW-27");
/// ```
pub fn normalize_aircraft_model(model: &str) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let upper = trimmed.to_uppercase();

    // Check for prefix match against known manufacturers (longest first)
    for mapping in MANUFACTURER_MAPPINGS.iter() {
        if upper.starts_with(mapping.pattern) {
            let rest = &trimmed[mapping.pattern.len()..];
            let rest = rest.trim_start();
            if rest.is_empty() {
                return mapping.canonical.to_string();
            }
            return format!("{} {}", mapping.canonical, rest);
        }
    }

    // No known manufacturer prefix — return as-is
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- normalize_manufacturer tests ---

    #[test]
    fn test_known_manufacturer_exact_match() {
        assert_eq!(normalize_manufacturer("THE BOEING COMPANY"), "Boeing");
        assert_eq!(normalize_manufacturer("CESSNA AIRCRAFT CO"), "Cessna");
        assert_eq!(normalize_manufacturer("AIRBUS S A S"), "Airbus");
        assert_eq!(normalize_manufacturer("PIPER AIRCRAFT CORP"), "Piper");
        assert_eq!(normalize_manufacturer("BEECH AIRCRAFT CORP"), "Beechcraft");
        assert_eq!(
            normalize_manufacturer("HAWKER BEECHCRAFT CORP"),
            "Beechcraft"
        );
        assert_eq!(normalize_manufacturer("CIRRUS DESIGN CORP"), "Cirrus");
        assert_eq!(normalize_manufacturer("BELL HELICOPTER TEXTRON"), "Bell");
        assert_eq!(normalize_manufacturer("ROBINSON HELICOPTER CO"), "Robinson");
        assert_eq!(normalize_manufacturer("MOONEY AIRCRAFT CORP"), "Mooney");
        assert_eq!(
            normalize_manufacturer("TEXTRON AVIATION INC"),
            "Textron Aviation"
        );
        assert_eq!(normalize_manufacturer("BOMBARDIER INC"), "Bombardier");
        assert_eq!(
            normalize_manufacturer("GULFSTREAM AEROSPACE CORP"),
            "Gulfstream"
        );
        assert_eq!(
            normalize_manufacturer("EMBRAER EMPRESA BRASILEIRA DE"),
            "Embraer"
        );
        assert_eq!(normalize_manufacturer("DIAMOND AIRCRAFT IND"), "Diamond");
        assert_eq!(
            normalize_manufacturer("MAULE AEROSPACE TECHNOLOGY INC"),
            "Maule"
        );
        assert_eq!(
            normalize_manufacturer("SOCATA GROUP AEROSPATIALE"),
            "Socata"
        );
        assert_eq!(normalize_manufacturer("PILATUS AIRCRAFT LTD"), "Pilatus");
        assert_eq!(normalize_manufacturer("DASSAULT AVIATION"), "Dassault");
        assert_eq!(normalize_manufacturer("SIKORSKY AIRCRAFT CORP"), "Sikorsky");
        assert_eq!(
            normalize_manufacturer("RAYTHEON AIRCRAFT COMPANY"),
            "Raytheon"
        );
        assert_eq!(
            normalize_manufacturer("MCDONNELL DOUGLAS"),
            "McDonnell Douglas"
        );
        assert_eq!(normalize_manufacturer("LOCKHEED MARTIN"), "Lockheed Martin");
    }

    #[test]
    fn test_case_insensitivity() {
        assert_eq!(normalize_manufacturer("the boeing company"), "Boeing");
        assert_eq!(normalize_manufacturer("The Boeing Company"), "Boeing");
        assert_eq!(normalize_manufacturer("cessna aircraft co"), "Cessna");
    }

    #[test]
    fn test_suffix_stripping_unknown_manufacturer() {
        assert_eq!(normalize_manufacturer("ACME AVIATION INC"), "Acme Aviation");
        assert_eq!(
            normalize_manufacturer("FOOBAR AEROSPACE LLC"),
            "Foobar Aerospace"
        );
        assert_eq!(normalize_manufacturer("SKYWORKS CORP"), "Skyworks");
    }

    #[test]
    fn test_leading_the_stripped() {
        assert_eq!(normalize_manufacturer("THE UNKNOWN MFG"), "Unknown Mfg");
    }

    #[test]
    fn test_empty_and_whitespace() {
        assert_eq!(normalize_manufacturer(""), "");
        assert_eq!(normalize_manufacturer("   "), "");
    }

    #[test]
    fn test_short_names_passthrough() {
        // Short or already-clean names that don't match any mapping
        // should be title-cased after suffix stripping
        assert_eq!(normalize_manufacturer("GROB"), "Grob");
    }

    // --- normalize_aircraft_model tests ---

    #[test]
    fn test_known_prefix_replacement() {
        assert_eq!(
            normalize_aircraft_model("THE BOEING COMPANY 737-8"),
            "Boeing 737-8"
        );
        assert_eq!(
            normalize_aircraft_model("CESSNA AIRCRAFT CO 172S"),
            "Cessna 172S"
        );
        assert_eq!(
            normalize_aircraft_model("AIRBUS S A S A320-214"),
            "Airbus A320-214"
        );
        assert_eq!(
            normalize_aircraft_model("PIPER AIRCRAFT CORP PA-28-181"),
            "Piper PA-28-181"
        );
        assert_eq!(
            normalize_aircraft_model("BOMBARDIER INC CANADAIR CL-600"),
            "Bombardier CL-600"
        );
    }

    #[test]
    fn test_model_case_insensitive_prefix() {
        assert_eq!(
            normalize_aircraft_model("the boeing company 737-8"),
            "Boeing 737-8"
        );
    }

    #[test]
    fn test_model_preserves_model_number_case() {
        // The model number portion after the manufacturer should be preserved as-is
        assert_eq!(
            normalize_aircraft_model("CESSNA AIRCRAFT CO 172S Skyhawk"),
            "Cessna 172S Skyhawk"
        );
    }

    #[test]
    fn test_model_no_match_passthrough() {
        // Glider types and other short model names should pass through unchanged
        assert_eq!(normalize_aircraft_model("ASW-27"), "ASW-27");
        assert_eq!(normalize_aircraft_model("Discus-2"), "Discus-2");
        assert_eq!(normalize_aircraft_model("LS-8"), "LS-8");
        assert_eq!(normalize_aircraft_model("DG-800"), "DG-800");
    }

    #[test]
    fn test_model_empty() {
        assert_eq!(normalize_aircraft_model(""), "");
        assert_eq!(normalize_aircraft_model("   "), "");
    }

    #[test]
    fn test_manufacturer_only_in_model_field() {
        // Sometimes model field contains just the manufacturer name
        assert_eq!(normalize_aircraft_model("THE BOEING COMPANY"), "Boeing");
        assert_eq!(normalize_aircraft_model("CESSNA"), "Cessna");
    }

    #[test]
    fn test_new_piper_variants() {
        assert_eq!(normalize_manufacturer("NEW PIPER AIRCRAFT INC"), "Piper");
        assert_eq!(
            normalize_aircraft_model("NEW PIPER AIRCRAFT INC PA-32R-301T"),
            "Piper PA-32R-301T"
        );
    }

    #[test]
    fn test_airbus_helicopters_preserved() {
        assert_eq!(
            normalize_manufacturer("AIRBUS HELICOPTERS"),
            "Airbus Helicopters"
        );
        assert_eq!(
            normalize_aircraft_model("AIRBUS HELICOPTERS EC135T2+"),
            "Airbus Helicopters EC135T2+"
        );
    }

    #[test]
    fn test_title_case_function() {
        assert_eq!(title_case("HELLO WORLD"), "Hello World");
        assert_eq!(title_case("hello-world"), "Hello-World");
        assert_eq!(title_case(""), "");
        assert_eq!(title_case("A"), "A");
    }

    #[test]
    fn test_glider_manufacturers() {
        assert_eq!(
            normalize_manufacturer("SCHEMPP-HIRTH FLUGZEUGBAU GMBH"),
            "Schempp-Hirth"
        );
        assert_eq!(
            normalize_manufacturer("ALEXANDER SCHLEICHER GMBH"),
            "Schleicher"
        );
        assert_eq!(
            normalize_manufacturer("DG FLUGZEUGBAU GMBH"),
            "DG Flugzeugbau"
        );
        assert_eq!(
            normalize_manufacturer("ROLLADEN-SCHNEIDER FLUGZEUGBAU"),
            "Rolladen-Schneider"
        );
    }
}
