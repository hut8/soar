//! A Status Report announces the station's current mission or any other single
//! line status to everyone. The report starts with the '>' APRS Data Type Identifier.
//! The report may optionally contain a timestamp.
//!
//! Examples:
//! - ">12.6V 0.2A 22degC"              (report without timestamp)
//! - ">120503hFatal error"             (report with timestamp in HMS format)
//! - ">281205zSystem will shutdown"    (report with timestamp in DHM format)

use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::Serialize;

use crate::AprsError;
use crate::Timestamp;
use crate::status_comment::StatusComment;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct AprsStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Timestamp>,
    #[serde(flatten)]
    pub comment: StatusComment,
}

impl FromStr for AprsStatus {
    type Err = AprsError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        // Interpret the first 7 bytes as a timestamp, if valid.
        // Otherwise the whole field is the comment.
        let timestamp = if s.len() >= 7 {
            match s[0..7].parse::<Timestamp>() {
                Ok(Timestamp::Invalid(_)) => None,
                Ok(ts) => Some(ts),
                Err(_) => None,
            }
        } else {
            None
        };
        let comment = if timestamp.is_some() { &s[7..] } else { s };

        Ok(AprsStatus {
            timestamp,
            comment: comment.parse::<StatusComment>().unwrap(),
        })
    }
}

impl Display for AprsStatus {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, ">")?;

        if let Some(ts) = &self.timestamp {
            write!(f, "{ts}")?;
        }
        write!(f, "{:#?}", self.comment)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use csv::WriterBuilder;
    use std::io::stdout;

    use super::*;

    #[test]
    fn parse_without_timestamp_or_comment() {
        let result = "".parse::<AprsStatus>().unwrap();
        assert_eq!(result.timestamp, None);
        assert_eq!(result.comment, StatusComment::default());
    }

    #[test]
    fn parse_with_timestamp_without_comment() {
        let result = "312359z".parse::<AprsStatus>().unwrap();
        assert_eq!(result.timestamp, Some(Timestamp::DDHHMM(31, 23, 59)));
        assert_eq!(result.comment, StatusComment::default());
    }

    #[test]
    fn parse_without_timestamp_with_comment() {
        let result = "Hi there!".parse::<AprsStatus>().unwrap();
        assert_eq!(result.timestamp, None);
        assert_eq!(result.comment.unparsed.unwrap(), "Hi there!");
    }

    #[test]
    fn parse_with_timestamp_and_comment() {
        let result = "235959hHi there!".parse::<AprsStatus>().unwrap();
        assert_eq!(result.timestamp, Some(Timestamp::HHMMSS(23, 59, 59)));
        assert_eq!(result.comment.unparsed.unwrap(), "Hi there!");
    }

    #[ignore = "status_comment serialization not implemented"]
    #[test]
    fn test_serialize() {
        let aprs_position = "235959hHi there!".parse::<AprsStatus>().unwrap();
        let mut wtr = WriterBuilder::new().from_writer(stdout());
        wtr.serialize(aprs_position).unwrap();
        wtr.flush().unwrap();
    }

    #[test]
    fn test_problematic_name_parsing() {
        // This should not panic and should parse successfully with the malformed Name="
        let input = r#"215504h Name=""#;
        let result = input.parse::<AprsStatus>();

        // Should parse successfully with the unparsed part containing "Name="
        assert!(result.is_ok());
        let status = result.unwrap();
        assert_eq!(status.timestamp, Some(Timestamp::HHMMSS(21, 55, 4)));
        assert_eq!(status.comment.name, None); // Name should not be parsed due to malformed quotes
        assert_eq!(status.comment.unparsed, Some("Name=\"".to_string()));
    }
}
