use anyhow::{anyhow, Context, Result};
use chrono::NaiveDate;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Convenience: inclusive 1-based positions from the spec → 0-based Rust slice range
fn fw(s: &str, start_1: usize, end_1: usize) -> &str {
    let start = start_1.saturating_sub(1);
    let end = end_1.min(s.len());
    if start >= end || end > s.len() {
        ""
    } else {
        &s[start..end]
    }
}

fn to_opt_string(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn to_string_trim(s: &str) -> String {
    s.trim().to_string()
}

fn to_opt_date(yyyymmdd: &str) -> Option<NaiveDate> {
    let t = yyyymmdd.trim();
    if t.len() != 8 || !t.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let y = &t[0..4].parse::<i32>().ok()?;
    let m = &t[4..6].parse::<u32>().ok()?;
    let d = &t[6..8].parse::<u32>().ok()?;
    NaiveDate::from_ymd_opt(*y, *m, *d)
}

fn to_opt_u32(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() { return None; }
    t.parse::<u32>().ok()
}

fn yn_to_bool(s: &str) -> Option<bool> {
    match s.trim() {
        "Y" | "y" => Some(true),
        "N" | "n" => Some(false),
        "" => None,
        _ => None,
    }
}

/// FAA stores Mode S in octal (257–264) and hex (602–611). We store a single number:
/// - Prefer HEX if present (common 24-bit ICAO code)
/// - Else try OCTAL
fn parse_transponder_number(line: &str) -> Option<u32> {
    let hex_raw = fw(line, 602, 611).trim(); // 10 chars (but often 6 hex significant)
    if !hex_raw.is_empty() {
        // Strip non-hex padding and parse
        let hex = hex_raw.trim();
        // Some files right-align; just remove spaces and parse as hex
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            return Some(v);
        }
    }
    let oct_raw = fw(line, 257, 264).trim();
    if !oct_raw.is_empty() && oct_raw.chars().all(|c| c >= '0' && c <= '7') {
        // Convert octal to decimal
        if let Ok(v) = u32::from_str_radix(oct_raw, 8) {
            return Some(v);
        }
    }
    None
}

/// Named Approved Operation flags (first 8 pages only).
/// Keep the raw 9-char string for audit, and set flags via mapping by class/slots.
/// You can tweak the mapping tables below to match your ingestion guide precisely.
#[derive(Debug, Clone, Default)]
pub struct ApprovedOps {
    // Restricted (examples)
    pub restricted_other: bool,
    pub restricted_ag_pest_control: bool,
    pub restricted_aerial_surveying: bool,
    pub restricted_aerial_advertising: bool,
    pub restricted_forest: bool,
    pub restricted_patrolling: bool,
    pub restricted_weather_control: bool,
    pub restricted_carriage_of_cargo: bool,

    // Experimental (examples)
    pub exp_show_compliance: bool,
    pub exp_research_development: bool,
    pub exp_amateur_built: bool,
    pub exp_exhibition: bool,
    pub exp_racing: bool,
    pub exp_crew_training: bool,
    pub exp_market_survey: bool,
    pub exp_operating_kit_built: bool,
    pub exp_lsa_reg_prior_2008: bool,        // 8A (legacy)
    pub exp_lsa_operating_kit_built: bool,   // 8B
    pub exp_lsa_prev_21_190: bool,           // 8C
    pub exp_uas_research_development: bool,  // 9A
    pub exp_uas_market_survey: bool,         // 9B
    pub exp_uas_crew_training: bool,         // 9C
    pub exp_uas_exhibition: bool,            // 9D
    pub exp_uas_compliance_with_cfr: bool,   // 9E

    // Special Flight Permit
    pub sfp_ferry_for_repairs_alterations_storage: bool,
    pub sfp_evacuate_impending_danger: bool,
    pub sfp_excess_of_max_certificated: bool,
    pub sfp_delivery_or_export: bool,
    pub sfp_production_flight_testing: bool,
    pub sfp_customer_demo: bool,
}

/// Minimal, adjustable mapping from the 9-char ops string to flags.
/// This *must* be aligned with your specific FAA slot key for the file vintage you ingest.
/// As a safe default, we only set a few widely used Experimental flags by digit presence.
fn parse_approved_ops(airworthiness_class_code: &str, raw_239_247: &str) -> ApprovedOps {
    let mut ops = ApprovedOps::default();
    let raw = raw_239_247.trim();

    // If Experimental (code '4'), interpret common digits:
    // 1=show compliance, 2=R&D, 3=amateur built, 4=exhibition, 5=racing,
    // 6=crew training, 7=market survey, 8=kit-built (legacy), 9=UAS bucket (sub-letters A..E)
    if airworthiness_class_code == "4" {
        for ch in raw.chars() {
            match ch {
                '1' => ops.exp_show_compliance = true,
                '2' => ops.exp_research_development = true,
                '3' => ops.exp_amateur_built = true,
                '4' => ops.exp_exhibition = true,
                '5' => ops.exp_racing = true,
                '6' => ops.exp_crew_training = true,
                '7' => ops.exp_market_survey = true,
                '8' => ops.exp_operating_kit_built = true, // generic legacy bucket
                '9' => {
                    // UAS sub-bucket; the actual sub-letter may be in the same field in some vintages.
                    // If your source emits '9A'...'9E' explicitly, extend parsing here.
                }
                // Occasionally letters present for subcategories (8A/8B/8C, 9A..9E)
                'A' => { ops.exp_lsa_reg_prior_2008 = true; ops.exp_uas_research_development = true; }
                'B' => { ops.exp_lsa_operating_kit_built = true; ops.exp_uas_market_survey = true; }
                'C' => { ops.exp_lsa_prev_21_190 = true; ops.exp_uas_crew_training = true; }
                'D' => ops.exp_uas_exhibition = true,
                'E' => ops.exp_uas_compliance_with_cfr = true,
                _ => {}
            }
        }
    }

    // If Restricted (code '3'), you may map by position index (239..247).
    // Leave default false unless you have an authoritative slot meaning table.
    // If Special Flight Permit (code '8'), similarly map as needed.
    ops
}

#[derive(Debug, Clone)]
pub struct Aircraft {
    pub n_number: String,                 // 1–5
    pub serial_number: String,            // 7–36
    pub mfr_mdl_code: Option<String>,     // 38–44
    pub eng_mfr_mdl_code: Option<String>, // 46–50
    pub year_mfr: Option<u16>,            // 52–55

    // Registrant / address
    pub type_registration_code: Option<String>, // 57
    pub registrant_name: Option<String>,        // 59–108
    pub street1: Option<String>,                // 110–142
    pub street2: Option<String>,                // 144–176
    pub city: Option<String>,                   // 178–195
    pub state: Option<String>,                  // 197–198
    pub zip_code: Option<String>,               // 200–209
    pub region_code: Option<String>,            // 211
    pub county_mail_code: Option<String>,       // 213–215
    pub country_mail_code: Option<String>,      // 217–218

    // Dates
    pub last_action_date: Option<NaiveDate>,     // 220–227
    pub certificate_issue_date: Option<NaiveDate>, // 229–236

    // Airworthiness & ops
    pub airworthiness_class_code: Option<String>, // 238
    pub approved_operations_raw: Option<String>,  // 239–247
    pub approved_ops: ApprovedOps,                // mapped flags (best effort)

    pub type_aircraft_code: Option<String>,      // 249
    pub type_engine_code: Option<String>,        // 251–252
    pub status_code: Option<String>,             // 254–255

    // Mode S transponder as a single number
    pub transponder_code: Option<u32>,           // from 602–611 (hex) or 257–264 (octal)

    pub fractional_owner: Option<bool>,          // 266
    pub airworthiness_date: Option<NaiveDate>,   // 268–275

    // Other Names (up to 5)
    pub other_names: Vec<String>,                // 277–326, 328–377, 379–428, 430–479, 481–530

    // Registration expiration
    pub expiration_date: Option<NaiveDate>,      // 532–539

    // FAA unique ID
    pub unique_id: Option<String>,               // 541–548

    // Amateur/kit
    pub kit_mfr_name: Option<String>,            // 550–579
    pub kit_model_name: Option<String>,          // 581–600
}

impl Aircraft {
    pub fn from_fixed_width_line(line: &str) -> Result<Self> {
        // Expect at least the last position we touch. Many files are 611/612 chars.
        if line.len() < 611 {
            return Err(anyhow!("Line too short: expected ~611 chars, got {}", line.len()));
        }

        let n_number = to_string_trim(fw(line, 1, 5));
        if n_number.is_empty() {
            return Err(anyhow!("Missing N-number at positions 1–5"));
        }

        let serial_number = to_string_trim(fw(line, 7, 36));

        let mfr_mdl_code = to_opt_string(fw(line, 38, 44));
        let eng_mfr_mdl_code = to_opt_string(fw(line, 46, 50));
        let year_mfr = to_opt_u32(fw(line, 52, 55)).map(|v| v as u16);

        let type_registration_code = to_opt_string(fw(line, 57, 57));
        let registrant_name = to_opt_string(fw(line, 59, 108));
        let street1 = to_opt_string(fw(line, 110, 142));
        let street2 = to_opt_string(fw(line, 144, 176));
        let city = to_opt_string(fw(line, 178, 195));
        let state = to_opt_string(fw(line, 197, 198));
        let zip_code = to_opt_string(fw(line, 200, 209));
        let region_code = to_opt_string(fw(line, 211, 211));
        let county_mail_code = to_opt_string(fw(line, 213, 215));
        let country_mail_code = to_opt_string(fw(line, 217, 218));

        let last_action_date = to_opt_date(fw(line, 220, 227));
        let certificate_issue_date = to_opt_date(fw(line, 229, 236));

        let airworthiness_class_code = to_opt_string(fw(line, 238, 238));
        let approved_operations_raw = to_opt_string(fw(line, 239, 247));

        let approved_ops = if let (Some(class_code), Some(raw)) =
            (&airworthiness_class_code, &approved_operations_raw)
        {
            parse_approved_ops(class_code.as_str(), raw.as_str())
        } else {
            ApprovedOps::default()
        };

        let type_aircraft_code = to_opt_string(fw(line, 249, 249));
        let type_engine_code = to_opt_string(fw(line, 251, 252));
        let status_code = to_opt_string(fw(line, 254, 255));

        let transponder_code = parse_transponder_number(line);

        let fractional_owner = yn_to_bool(fw(line, 266, 266));
        let airworthiness_date = to_opt_date(fw(line, 268, 275));

        let other1 = to_opt_string(fw(line, 277, 326));
        let other2 = to_opt_string(fw(line, 328, 377));
        let other3 = to_opt_string(fw(line, 379, 428));
        let other4 = to_opt_string(fw(line, 430, 479));
        let other5 = to_opt_string(fw(line, 481, 530));
        let mut other_names = Vec::new();
        for o in [other1, other2, other3, other4, other5] {
            if let Some(v) = o { other_names.push(v); }
        }

        let expiration_date = to_opt_date(fw(line, 532, 539));
        let unique_id = to_opt_string(fw(line, 541, 548));
        let kit_mfr_name = to_opt_string(fw(line, 550, 579));
        let kit_model_name = to_opt_string(fw(line, 581, 600));

        Ok(Aircraft {
            n_number,
            serial_number,
            mfr_mdl_code,
            eng_mfr_mdl_code,
            year_mfr,

            type_registration_code,
            registrant_name,
            street1,
            street2,
            city,
            state,
            zip_code,
            region_code,
            county_mail_code,
            country_mail_code,

            last_action_date,
            certificate_issue_date,

            airworthiness_class_code,
            approved_operations_raw,
            approved_ops,

            type_aircraft_code,
            type_engine_code,
            status_code,

            transponder_code,

            fractional_owner,
            airworthiness_date,

            other_names,

            expiration_date,

            unique_id,

            kit_mfr_name,
            kit_model_name,
        })
    }
}

/// Read a fixed-width FAA Aircraft Master file (first 8 pages spec) and parse all rows.
/// Skips blank lines. Returns an error on the first malformed (too-short) line.
pub fn read_aircraft_file<P: AsRef<Path>>(path: P) -> Result<Vec<Aircraft>> {
    let f = File::open(path.as_ref())
        .with_context(|| format!("Opening {:?}", path.as_ref()))?;
    let reader = BufReader::new(f);
    let mut out = Vec::new();

    for (lineno, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("Reading line {}", lineno + 1))?;
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);

        if trimmed.trim().is_empty() {
            continue;
        }
        let rec = Aircraft::from_fixed_width_line(trimmed)
            .with_context(|| format!("Parsing line {}", lineno + 1))?;
        out.push(rec);
    }

    Ok(out)
}
