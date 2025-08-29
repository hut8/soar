//! FAA Aircraft Registration Data Parser
//!
//! This module provides parsing capabilities for FAA aircraft registration data
//! in fixed-width format. The data format combines CSV structure with fixed-width
//! field positioning, and this parser uses the fixed-width positions for accuracy.

use std::str::FromStr;

/// Represents the type of aircraft registrant based on FAA codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrantType {
    /// 1 - Individual
    Individual,
    /// 2 - Partnership
    Partnership,
    /// 3 - Corporation
    Corporation,
    /// 4 - Co-Owned
    CoOwned,
    /// 5 - Government
    Government,
    /// 7 - LLC
    LLC,
    /// 8 - Non Citizen Corporation
    NonCitizenCorporation,
    /// 9 - Non Citizen Co-Owned
    NonCitizenCoOwned,
    /// Unknown or invalid registrant type
    Unknown(char),
}

impl From<char> for RegistrantType {
    fn from(c: char) -> Self {
        match c {
            '1' => RegistrantType::Individual,
            '2' => RegistrantType::Partnership,
            '3' => RegistrantType::Corporation,
            '4' => RegistrantType::CoOwned,
            '5' => RegistrantType::Government,
            '7' => RegistrantType::LLC,
            '8' => RegistrantType::NonCitizenCorporation,
            '9' => RegistrantType::NonCitizenCoOwned,
            _ => RegistrantType::Unknown(c),
        }
    }
}

/// Represents a complete FAA aircraft registration record
#[derive(Debug, Clone, PartialEq)]
pub struct AircraftRegistration {
    /// N-Number: Identification number assigned to aircraft (positions 1-5)
    pub n_number: String,
    /// Serial Number: Complete aircraft serial number (positions 7-36)
    pub serial_number: String,
    /// Aircraft Manufacturer Model Code (positions 38-44)
    pub aircraft_mfr_model_code: String,
    /// Engine Manufacturer Mode Code (positions 46-50)
    pub engine_mfr_mode_code: String,
    /// Year Manufactured (positions 52-55)
    pub year_mfr: Option<u16>,
    /// Type of Registrant (position 57)
    pub type_registrant: RegistrantType,
    /// First registrant's name (positions 59-108)
    pub registrant_name: String,
    /// Street address (positions 110-142)
    pub street1: String,
}

/// Errors that can occur during parsing of aircraft registration data
#[derive(Debug, Clone, PartialEq)]
pub enum ParseRegistrationError {
    /// Line is too short to contain all required fields
    LineTooShort { expected: usize, actual: usize },
    /// Failed to parse year as a number
    InvalidYear(String),
    /// General parsing error with context
    ParseError(String),
}

impl std::fmt::Display for ParseRegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseRegistrationError::LineTooShort { expected, actual } => {
                write!(f, "Line too short: expected at least {} characters, got {}", expected, actual)
            }
            ParseRegistrationError::InvalidYear(year_str) => {
                write!(f, "Invalid year: '{}'", year_str)
            }
            ParseRegistrationError::ParseError(msg) => {
                write!(f, "Parse error: {}", msg)
            }
        }
    }
}

impl std::error::Error for ParseRegistrationError {}

impl FromStr for AircraftRegistration {
    type Err = ParseRegistrationError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        // Minimum length needed to reach the last required field (street1 ends at position 142)
        const MIN_LENGTH: usize = 142;

        if line.len() < MIN_LENGTH {
            return Err(ParseRegistrationError::LineTooShort {
                expected: MIN_LENGTH,
                actual: line.len(),
            });
        }

        // Extract fields using fixed-width positions (converting to 0-based indexing)
        // N-Number: positions 1-5 (0-4 in 0-based)
        let n_number = extract_field(line, 0, 5).trim().to_string();

        // Serial Number: positions 7-36 (6-35 in 0-based)
        let serial_number = extract_field(line, 6, 30).trim().to_string();

        // Aircraft Mfr Model Code: positions 38-44 (37-43 in 0-based)
        let aircraft_mfr_model_code = extract_field(line, 37, 7).trim().to_string();

        // Engine Mfr Mode Code: positions 46-50 (45-49 in 0-based)
        let engine_mfr_mode_code = extract_field(line, 45, 5).trim().to_string();

        // Year Mfr: positions 52-55 (51-54 in 0-based)
        let year_str = extract_field(line, 51, 4).trim();
        let year_mfr = if year_str.is_empty() {
            None
        } else {
            Some(year_str.parse::<u16>().map_err(|_| {
                ParseRegistrationError::InvalidYear(year_str.to_string())
            })?)
        };

        // Type Registrant: position 57 (56 in 0-based)
        let type_registrant_char = line.chars().nth(56).unwrap_or(' ');
        let type_registrant = RegistrantType::from(type_registrant_char);

        // Registrant's Name: positions 59-108 (58-107 in 0-based)
        let registrant_name = extract_field(line, 58, 50).trim().to_string();

        // Street1: positions 110-142 (109-141 in 0-based)
        let street1 = extract_field(line, 109, 33).trim().to_string();

        Ok(AircraftRegistration {
            n_number,
            serial_number,
            aircraft_mfr_model_code,
            engine_mfr_mode_code,
            year_mfr,
            type_registrant,
            registrant_name,
            street1,
        })
    }
}

/// Extract a field from a line at the specified position with the given length
fn extract_field(line: &str, start: usize, length: usize) -> &str {
    let end = std::cmp::min(start + length, line.len());
    if start >= line.len() {
        ""
    } else {
        &line[start..end]
    }
}

/// Parse multiple aircraft registration records from a string containing multiple lines
pub fn parse_registrations(data: &str) -> Vec<Result<AircraftRegistration, ParseRegistrationError>> {
    data.lines()
        .filter(|line| !line.trim().is_empty()) // Skip empty lines
        .map(|line| line.parse::<AircraftRegistration>())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registrant_type_conversion() {
        assert_eq!(RegistrantType::from('1'), RegistrantType::Individual);
        assert_eq!(RegistrantType::from('2'), RegistrantType::Partnership);
        assert_eq!(RegistrantType::from('3'), RegistrantType::Corporation);
        assert_eq!(RegistrantType::from('4'), RegistrantType::CoOwned);
        assert_eq!(RegistrantType::from('5'), RegistrantType::Government);
        assert_eq!(RegistrantType::from('7'), RegistrantType::LLC);
        assert_eq!(RegistrantType::from('8'), RegistrantType::NonCitizenCorporation);
        assert_eq!(RegistrantType::from('9'), RegistrantType::NonCitizenCoOwned);
        assert_eq!(RegistrantType::from('X'), RegistrantType::Unknown('X'));
    }

    #[test]
    fn test_extract_field() {
        let line = "12345 ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        assert_eq!(extract_field(line, 0, 5), "12345");
        assert_eq!(extract_field(line, 6, 10), "ABCDEFGHIJ");
        assert_eq!(extract_field(line, 25, 10), "TUVWXYZ"); // Beyond string length, returns what's available
        assert_eq!(extract_field(line, 50, 5), ""); // Start beyond string length
    }

    #[test]
    fn test_parse_valid_registration() {
        // Create a test line with proper fixed-width formatting (exactly 142 characters)
        let test_line = "N123X SERIALNUMBER123456789012345678 BOEING1 ENG12 2020 1 JOHN DOE                                          123 MAIN ST                       ";

        let registration = test_line.parse::<AircraftRegistration>().unwrap();

        assert_eq!(registration.n_number, "N123X");
        assert_eq!(registration.serial_number, "SERIALNUMBER123456789012345678");
        assert_eq!(registration.aircraft_mfr_model_code, "BOEING1");
        assert_eq!(registration.engine_mfr_mode_code, "ENG12");
        assert_eq!(registration.year_mfr, Some(2020));
        assert_eq!(registration.type_registrant, RegistrantType::Individual);
        assert_eq!(registration.registrant_name, "JOHN DOE");
        assert_eq!(registration.street1, "123 MAIN ST");
    }

    #[test]
    fn test_parse_registration_with_empty_year() {
        // Test with empty year field (exactly 142 characters)
        let test_line = "N456Y SERIALNUMBER                     CESSNA1 ENG34     3 JANE SMITH                                        456 OAK AVE                      ";

        let registration = test_line.parse::<AircraftRegistration>().unwrap();

        assert_eq!(registration.n_number, "N456Y");
        assert_eq!(registration.year_mfr, None);
        assert_eq!(registration.type_registrant, RegistrantType::Corporation);
        assert_eq!(registration.registrant_name, "JANE SMITH");
    }

    #[test]
    fn test_parse_line_too_short() {
        let short_line = "N123X";
        let result = short_line.parse::<AircraftRegistration>();

        assert!(matches!(result, Err(ParseRegistrationError::LineTooShort { .. })));
    }

    #[test]
    fn test_parse_invalid_year() {
        let test_line = "N789Z SERIALNUMBER                     PIPER12 ENG56 ABCD 5 BOB JOHNSON                                       789 ELM ST                      ";
        let result = test_line.parse::<AircraftRegistration>();

        assert!(matches!(result, Err(ParseRegistrationError::InvalidYear(_))));
    }

    #[test]
    fn test_parse_multiple_registrations() {
        let data = "N123X SERIALNUMBER123456789012345678 BOEING1 ENG12 2020 1 JOHN DOE                                          123 MAIN ST                       \nN456Y SERIALNUMBER                     CESSNA1 ENG34     3 JANE SMITH                                        456 OAK AVE                      ";

        let results = parse_registrations(data);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }
}
