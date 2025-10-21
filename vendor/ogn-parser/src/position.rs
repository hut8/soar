use std::fmt::Write;
use std::str::FromStr;

use flat_projection::FlatProjection;
use serde::Serialize;

use crate::AprsData;
use crate::AprsError;
use crate::AprsPacket;
use crate::AprsSymbol;
use crate::Callsign;
use crate::EncodeError;
use crate::Timestamp;
use crate::lonlat::{Latitude, Longitude, encode_latitude, encode_longitude};
use crate::position_comment::PositionComment;

pub struct Relation {
    pub bearing: f64,
    pub distance: f64,
}

/// Type of position source based on the via field
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PositionSourceType {
    /// Position from a ground-based receiver (contains TCPIP* and qAC in via)
    Receiver,
    /// Position from an aircraft (contains qAS in via)
    Aircraft,
    /// Position from a weather station (symbol is weather station)
    WeatherStation,
    /// Unknown source type
    Unknown,
    /// Not a position packet
    NotPosition,
}

impl PositionSourceType {
    pub fn from_packet(packet: &AprsPacket) -> Self {
        match &packet.data {
            AprsData::Position(pos) => {
                if let Some(sym) = AprsSymbol::parse(pos.symbol_table, pos.symbol_code)
                    && sym == AprsSymbol::WeatherStation
                {
                    return PositionSourceType::WeatherStation;
                }
                PositionSourceType::from_via(&packet.via)
            }
            _ => PositionSourceType::NotPosition,
        }
    }

    /// Determine the source type based on the via field of an APRS packet
    pub fn from_via(via: &[Callsign]) -> Self {
        let has_tcpip = via.iter().any(|callsign| callsign.0.starts_with("TCPIP"));
        let has_qac = via.iter().any(|callsign| callsign.0 == "qAC");
        let has_qas = via.iter().any(|callsign| callsign.0 == "qAS");

        if has_tcpip && has_qac {
            PositionSourceType::Receiver
        } else if has_qas {
            PositionSourceType::Aircraft
        } else {
            PositionSourceType::Unknown
        }
    }
}

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct AprsPosition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<Timestamp>,
    pub messaging_supported: bool,
    pub latitude: Latitude,
    pub longitude: Longitude,
    pub symbol_table: char,
    pub symbol_code: char,
    #[serde(flatten)]
    pub comment: PositionComment,
}

impl FromStr for AprsPosition {
    type Err = AprsError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let messaging_supported = s.starts_with('=') || s.starts_with('@');
        let has_timestamp = s.starts_with('@') || s.starts_with('/');

        // check for minimal message length
        if (!has_timestamp && s.len() < 20) || (has_timestamp && s.len() < 27) {
            return Err(AprsError::InvalidPosition(s.to_owned()));
        };

        // Extract timestamp and remaining string
        let (timestamp, s) = if has_timestamp {
            (Some(s[1..8].parse()?), &s[8..])
        } else {
            (None, &s[1..])
        };

        // check for compressed position format
        let is_uncompressed_position = s.chars().take(1).all(|c| c.is_numeric());
        if !is_uncompressed_position {
            return Err(AprsError::UnsupportedPositionFormat(s.to_owned()));
        }

        // parse position
        let mut latitude: Latitude = s[0..8].parse()?;
        let mut longitude: Longitude = s[9..18].parse()?;

        let symbol_table = s.chars().nth(8).unwrap();
        let symbol_code = s.chars().nth(18).unwrap();

        let comment = &s[19..s.len()];

        // parse the comment
        let ogn = comment.parse::<PositionComment>().unwrap();

        // The comment may contain additional position precision information that will be added to the current position
        if let Some(additional_precision) = &ogn.additional_precision {
            *latitude += latitude.signum() * additional_precision.lat as f64 / 60_000.;
            *longitude += longitude.signum() * additional_precision.lon as f64 / 60_000.;
        }

        Ok(AprsPosition {
            timestamp,
            messaging_supported,
            latitude,
            longitude,
            symbol_table,
            symbol_code,
            comment: ogn,
        })
    }
}

impl AprsPosition {
    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        let sym = match (self.timestamp.is_some(), self.messaging_supported) {
            (true, true) => '@',
            (true, false) => '/',
            (false, true) => '=',
            (false, false) => '!',
        };

        write!(buf, "{sym}")?;

        if let Some(ts) = &self.timestamp {
            write!(buf, "{ts}")?;
        }

        write!(
            buf,
            "{}{}{}{}{:#?}",
            encode_latitude(self.latitude)?,
            self.symbol_table,
            encode_longitude(self.longitude)?,
            self.symbol_code,
            self.comment,
        )?;

        Ok(())
    }

    pub fn get_relation(&self, other: &Self) -> Relation {
        let mean_longitude: f64 = (*self.longitude + *other.longitude) / 2.0;
        let mean_latitude: f64 = (*self.latitude + *other.latitude) / 2.0;
        let flat_projection = FlatProjection::new(mean_longitude, mean_latitude);

        let p1 = flat_projection.project(*self.latitude, *self.longitude);
        let p2 = flat_projection.project(*other.latitude, *other.longitude);

        Relation {
            bearing: (450.0 - p1.bearing(&p2)) % 360.0, // convert from unit circle (0 @ east, counterclockwise) to compass rose (0 @ north, clockwise)
            distance: p1.distance(&p2) * 1000.0,        // convert from [km] to [m]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::WriterBuilder;
    use std::io::stdout;

    #[test]
    fn parse_without_timestamp_or_messaging() {
        let result = r"!4903.50N/07201.75W-".parse::<AprsPosition>().unwrap();
        assert_eq!(result.timestamp, None);
        assert!(!result.messaging_supported);
        assert_relative_eq!(*result.latitude, 49.05833333333333);
        assert_relative_eq!(*result.longitude, -72.02916666666667);
        assert_eq!(result.symbol_table, '/');
        assert_eq!(result.symbol_code, '-');
        assert_eq!(result.comment, PositionComment::default());
    }

    #[test]
    fn parse_with_comment() {
        let result = r"!4903.50N/07201.75W-Hello/A=001000"
            .parse::<AprsPosition>()
            .unwrap();
        assert_eq!(result.timestamp, None);
        assert_relative_eq!(*result.latitude, 49.05833333333333);
        assert_relative_eq!(*result.longitude, -72.02916666666667);
        assert_eq!(result.symbol_table, '/');
        assert_eq!(result.symbol_code, '-');
        assert_eq!(result.comment.unparsed.unwrap(), "Hello/A=001000");
    }

    #[test]
    fn parse_with_timestamp_without_messaging() {
        let result = r"/074849h4821.61N\01224.49E^322/103/A=003054"
            .parse::<AprsPosition>()
            .unwrap();
        assert_eq!(result.timestamp, Some(Timestamp::HHMMSS(7, 48, 49)));
        assert!(!result.messaging_supported);
        assert_relative_eq!(*result.latitude, 48.36016666666667);
        assert_relative_eq!(*result.longitude, 12.408166666666666);
        assert_eq!(result.symbol_table, '\\');
        assert_eq!(result.symbol_code, '^');
        assert_eq!(result.comment.altitude.unwrap(), 3054);
        assert_eq!(result.comment.course.unwrap(), 322);
        assert_eq!(result.comment.speed.unwrap(), 103);
    }

    #[test]
    fn parse_without_timestamp_with_messaging() {
        let result = r"=4903.50N/07201.75W-".parse::<AprsPosition>().unwrap();
        assert_eq!(result.timestamp, None);
        assert!(result.messaging_supported);
        assert_relative_eq!(*result.latitude, 49.05833333333333);
        assert_relative_eq!(*result.longitude, -72.02916666666667);
        assert_eq!(result.symbol_table, '/');
        assert_eq!(result.symbol_code, '-');
        assert_eq!(result.comment, PositionComment::default());
    }

    #[test]
    fn parse_with_timestamp_and_messaging() {
        let result = r"@074849h4821.61N\01224.49E^322/103/A=003054"
            .parse::<AprsPosition>()
            .unwrap();
        assert_eq!(result.timestamp, Some(Timestamp::HHMMSS(7, 48, 49)));
        assert!(result.messaging_supported);
        assert_relative_eq!(*result.latitude, 48.36016666666667);
        assert_relative_eq!(*result.longitude, 12.408166666666666);
        assert_eq!(result.symbol_table, '\\');
        assert_eq!(result.symbol_code, '^');
        assert_eq!(result.comment.altitude.unwrap(), 3054);
        assert_eq!(result.comment.course.unwrap(), 322);
        assert_eq!(result.comment.speed.unwrap(), 103);
    }

    #[test]
    fn test_latitude_longitude() {
        let result = r"/104337h5211.24N\00032.65W^124/081/A=004026 !W62!"
            .parse::<AprsPosition>()
            .unwrap();
        assert_relative_eq!(*result.latitude, 52.18743333333334);
        assert_relative_eq!(*result.longitude, -0.5442);
    }

    #[ignore = "position_comment serialization not implemented"]
    #[test]
    fn test_serialize() {
        let aprs_position = r"@074849h4821.61N\01224.49E^322/103/A=003054"
            .parse::<AprsPosition>()
            .unwrap();
        let mut wtr = WriterBuilder::new().from_writer(stdout());
        wtr.serialize(aprs_position).unwrap();
        wtr.flush().unwrap();
    }

    #[test]
    fn test_input_string_with_timestamp_too_short() {
        let result = r"/104337h5211.24N\00032.65W".parse::<AprsPosition>();
        assert!(result.is_err(), "Short input string should return an error");
    }

    #[test]
    fn test_input_string_without_timestamp_too_short() {
        let result = r"!4903.50N/07201.75W".parse::<AprsPosition>();
        assert!(result.is_err(), "Short input string should return an error");
    }

    #[test]
    fn test_bearing() {
        let receiver = r"!4903.50N/07201.75W-".parse::<AprsPosition>().unwrap();
        let east = r"!4903.50N/07101.75W-".parse::<AprsPosition>().unwrap();
        let north = r"!5004.50N/07201.75W-".parse::<AprsPosition>().unwrap();
        let west = r"!4903.50N/07301.75W-".parse::<AprsPosition>().unwrap();
        let south = r"!4803.50N/07201.75W-".parse::<AprsPosition>().unwrap();

        let to_north = receiver.get_relation(&north);
        assert_eq!(to_north.bearing, 0.0);

        let to_east = receiver.get_relation(&east);
        assert_eq!(to_east.bearing, 90.0);

        let to_south = receiver.get_relation(&south);
        assert_eq!(to_south.bearing, 180.0);

        let to_west = receiver.get_relation(&west);
        assert_eq!(to_west.bearing, 270.0);
    }
}
