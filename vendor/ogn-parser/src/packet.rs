use std::fmt::{Display, Write};
use std::str::FromStr;

use serde::Serialize;

use crate::AprsError;
use crate::AprsMessage;
use crate::AprsPosition;
use crate::AprsStatus;
use crate::Callsign;
use crate::EncodeError;

/// Data source types based on the TO field in APRS packets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DataSource {
    /// OGNFLR - for the flarm units - http://flarm.com
    OgnFlr,
    /// OGFLR6 - for the flarm units (old version 6) - http://flarm.com
    OgFlr6,
    /// OGFLR7 - for the flarm units (experimental) - http://flarm.com
    OgFlr7,
    /// OGFLR - for the flarm units - http://flarm.com
    OgFlr,
    /// OGNDSX - for the T_advisory - http://www.d-s-x.net/
    OgnDsx,
    /// OGNTRK - for the OGN tracker - http://wiki.glidernet.org
    OgnTrk,
    /// OGADSL - for the OGN tracker with ADS-L protocol - http://wiki.glidernet.org
    OgAdsl,
    /// OGADSB - for the ADS-B - https://www.adsbexchange.com/
    OgAdsb,
    /// OGNFNT - for the FANET - https://www.skytraxx.eu/
    OgnFnt,
    /// OGNPAW - for the PilotAware - https://www.pilotaware.com/
    OgnPaw,
    /// OGSPOT - for the SPOT - https://www.findmespot.com
    OgSpot,
    /// OGSPID - for the Spider - https://www.spidertracks.com/
    OgSpid,
    /// OGLT24 - for the LiveTrack24 - https://www.livetrack24.com/
    OgLt24,
    /// OGSKYL - for the Skyline - https://www.xcsoar.org/
    OgSkyl,
    /// OGCAPT - for the Capture - https://www.capturs.com/
    OgCapt,
    /// OGNAVI - for the Naviter devices - http://naviter.com
    OgnAvi,
    /// OGNMAV - for the MAVlink from drones - https://ardupilot.org/dev/docs/mavlink-basics.html
    OgnMav,
    /// OGFLYM - for the Flymaster devices - https://www.flymaster.net/
    OgFlym,
    /// OGNINRE - for the Garmin InReach devices - https://discover.garmin.com/en-US/inreach/personal/
    OgnInRe,
    /// OGEVARIO - for the eVario devices - https://apps.apple.com/us/app/evario-variometer-paraglider/id1243708983
    OgEvario,
    /// OGNDELAY - for the IGC sanctioned championships delayed messages using OGN/IGC approved trackers, it contains the number of seconds delayed
    OgnDelay,
    /// OGPAW - for the PilotAware devices - https://www.pilotaware.com/
    OgPaw,
    /// OGNTTN - for the The Things Network devices - https://www.thethingsnetwork.org/
    OgnTtn,
    /// OGNHEL - for the Helium LoRaWan devices - https://www.helium.com/
    OgnHel,
    /// OGAVZ - for the AVIAZE devices - https://www.aviaze.com/
    OgAvz,
    /// OGNSKY - for the SafeSky devices/app - https://www.safesky.app/en
    OgnSky,
    /// OGNMKT - for the MicroTrack devices - https://microtrak.fr/
    OgnMkt,
    /// OGNEMO - for the Canadian NEMO devices
    OgnEmo,
    /// OGNMYC - for MyCloudbase Tracker - https://mycloudbase.com/tracker/
    OgnMyc,
    /// OGSTUX - for Stratux trackers
    OgStux,
    /// OGNSXR - for moshe.braner@gmail.com - https://github.com/moshe-braner/Open-Glider-Network-Groundstation
    OgnSxr,
    /// OGAIRM - for AirMate - https://www.airmate.aero
    OgAirm,
    /// FXCAPP - for flyxc - https://flyxc.app/
    FxcApp,
    /// OGAPIK - for APIK OEM and compliant devices - https://api-k.com
    OgApik,
    /// OGMSHT - for meshtastic devices - https://meshtastic.org/
    OgMsht,
    /// OGNDVS - for weather devices like Davis - https://www.davisinstruments.com/pages/weather-stations https://www.sainlogic.com/
    OgnDvs,
    /// OGNWMN - for the Wingman - https://www.wingmanfly.app/
    OgnWmn,
    /// OGNVOL - for the Volandoo - https://www.volandoo.com/
    OgnVol,
}

impl FromStr for DataSource {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OGNFLR" => Ok(DataSource::OgnFlr),
            "OGFLR6" => Ok(DataSource::OgFlr6),
            "OGFLR7" => Ok(DataSource::OgFlr7),
            "OGFLR" => Ok(DataSource::OgFlr),
            "OGNDSX" => Ok(DataSource::OgnDsx),
            "OGNTRK" => Ok(DataSource::OgnTrk),
            "OGADSL" => Ok(DataSource::OgAdsl),
            "OGADSB" => Ok(DataSource::OgAdsb),
            "OGNFNT" => Ok(DataSource::OgnFnt),
            "OGNPAW" => Ok(DataSource::OgnPaw),
            "OGSPOT" => Ok(DataSource::OgSpot),
            "OGSPID" => Ok(DataSource::OgSpid),
            "OGLT24" => Ok(DataSource::OgLt24),
            "OGSKYL" => Ok(DataSource::OgSkyl),
            "OGCAPT" => Ok(DataSource::OgCapt),
            "OGNAVI" => Ok(DataSource::OgnAvi),
            "OGNMAV" => Ok(DataSource::OgnMav),
            "OGFLYM" => Ok(DataSource::OgFlym),
            "OGNINRE" => Ok(DataSource::OgnInRe),
            "OGEVARIO" => Ok(DataSource::OgEvario),
            "OGNDELAY" => Ok(DataSource::OgnDelay),
            "OGPAW" => Ok(DataSource::OgPaw),
            "OGNTTN" => Ok(DataSource::OgnTtn),
            "OGNHEL" => Ok(DataSource::OgnHel),
            "OGAVZ" => Ok(DataSource::OgAvz),
            "OGNSKY" => Ok(DataSource::OgnSky),
            "OGNMKT" => Ok(DataSource::OgnMkt),
            "OGNEMO" => Ok(DataSource::OgnEmo),
            "OGNMYC" => Ok(DataSource::OgnMyc),
            "OGSTUX" => Ok(DataSource::OgStux),
            "OGNSXR" => Ok(DataSource::OgnSxr),
            "OGAIRM" => Ok(DataSource::OgAirm),
            "FXCAPP" => Ok(DataSource::FxcApp),
            "OGAPIK" => Ok(DataSource::OgApik),
            "OGMSHT" => Ok(DataSource::OgMsht),
            "OGNDVS" => Ok(DataSource::OgnDvs),
            "OGNWMN" => Ok(DataSource::OgnWmn),
            "OGNVOL" => Ok(DataSource::OgnVol),
            _ => Err(()),
        }
    }
}

impl Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DataSource::OgnFlr => "OGNFLR",
            DataSource::OgFlr6 => "OGFLR6",
            DataSource::OgFlr7 => "OGFLR7",
            DataSource::OgFlr => "OGFLR",
            DataSource::OgnDsx => "OGNDSX",
            DataSource::OgnTrk => "OGNTRK",
            DataSource::OgAdsl => "OGADSL",
            DataSource::OgAdsb => "OGADSB",
            DataSource::OgnFnt => "OGNFNT",
            DataSource::OgnPaw => "OGNPAW",
            DataSource::OgSpot => "OGSPOT",
            DataSource::OgSpid => "OGSPID",
            DataSource::OgLt24 => "OGLT24",
            DataSource::OgSkyl => "OGSKYL",
            DataSource::OgCapt => "OGCAPT",
            DataSource::OgnAvi => "OGNAVI",
            DataSource::OgnMav => "OGNMAV",
            DataSource::OgFlym => "OGFLYM",
            DataSource::OgnInRe => "OGNINRE",
            DataSource::OgEvario => "OGEVARIO",
            DataSource::OgnDelay => "OGNDELAY",
            DataSource::OgPaw => "OGPAW",
            DataSource::OgnTtn => "OGNTTN",
            DataSource::OgnHel => "OGNHEL",
            DataSource::OgAvz => "OGAVZ",
            DataSource::OgnSky => "OGNSKY",
            DataSource::OgnMkt => "OGNMKT",
            DataSource::OgnEmo => "OGNEMO",
            DataSource::OgnMyc => "OGNMYC",
            DataSource::OgStux => "OGSTUX",
            DataSource::OgnSxr => "OGNSXR",
            DataSource::OgAirm => "OGAIRM",
            DataSource::FxcApp => "FXCAPP",
            DataSource::OgApik => "OGAPIK",
            DataSource::OgMsht => "OGMSHT",
            DataSource::OgnDvs => "OGNDVS",
            DataSource::OgnWmn => "OGNWMN",
            DataSource::OgnVol => "OGNVOL",
        };
        write!(f, "{s}")
    }
}

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct AprsPacket {
    pub from: Callsign,
    pub to: Callsign,
    pub via: Vec<Callsign>,
    #[serde(flatten)]
    pub data: AprsData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw: Option<String>,
}

impl FromStr for AprsPacket {
    type Err = AprsError;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let header_delimiter = s
            .find(':')
            .ok_or_else(|| AprsError::InvalidPacket(s.to_owned()))?;
        let (header, rest) = s.split_at(header_delimiter);
        let body = &rest[1..];

        let from_delimiter = header
            .find('>')
            .ok_or_else(|| AprsError::InvalidPacket(s.to_owned()))?;
        let (from, rest) = header.split_at(from_delimiter);
        let from = Callsign::from_str(from)?;

        let to_and_via = &rest[1..];
        let to_and_via: Vec<_> = to_and_via.split(',').collect();

        let to = to_and_via
            .first()
            .ok_or_else(|| AprsError::InvalidPacket(s.to_owned()))?;
        let to = Callsign::from_str(to)?;

        let mut via = vec![];
        for v in to_and_via.iter().skip(1) {
            via.push(Callsign::from_str(v)?);
        }

        let data = AprsData::from_str(body)?;

        Ok(AprsPacket {
            from,
            to,
            via,
            data,
            raw: Some(s.to_owned()),
        })
    }
}

impl AprsPacket {
    /// Get the source type based on the via field and packet type
    pub fn position_source_type(&self) -> crate::position::PositionSourceType {
        match &self.data {
            AprsData::Position(_) => crate::position::PositionSourceType::from_packet(self),
            _ => crate::position::PositionSourceType::NotPosition,
        }
    }

    pub fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        write!(buf, "{}>{}", self.from, self.to)?;
        for v in &self.via {
            write!(buf, ",{v}").unwrap();
        }
        write!(buf, ":")?;
        self.data.encode(buf)?;

        Ok(())
    }

    /// Parse the TO field and return the data source type if it matches a known variant
    pub fn data_source(&self) -> Option<DataSource> {
        self.to.0.parse().ok()
    }
}

#[derive(PartialEq, Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AprsData {
    Position(AprsPosition),
    Message(AprsMessage),
    Status(AprsStatus),
    Unknown,
}

impl FromStr for AprsData {
    type Err = AprsError;

    fn from_str(s: &str) -> Result<Self, AprsError> {
        Ok(match s.chars().next().unwrap_or(0 as char) {
            ':' => AprsData::Message(AprsMessage::from_str(&s[1..])?),
            '!' | '/' | '=' | '@' => AprsData::Position(AprsPosition::from_str(s)?),
            '>' => AprsData::Status(AprsStatus::from_str(&s[1..])?),
            _ => AprsData::Unknown,
        })
    }
}

impl AprsData {
    fn encode<W: Write>(&self, buf: &mut W) -> Result<(), EncodeError> {
        match self {
            Self::Position(p) => {
                p.encode(buf)?;
            }
            Self::Message(m) => {
                write!(buf, "{m}")?;
            }
            Self::Status(s) => {
                write!(buf, "{s}")?;
            }
            Self::Unknown => return Err(EncodeError::InvalidData),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Timestamp;
    use rust_decimal::prelude::*;

    #[test]
    fn parse() {
        let result = r"ICA3D17F2>APRS,qAS,dl4mea:/074849h4821.61N\01224.49E^322/103/A=003054 !W46! id213D17F2 -039fpm +0.0rot 2.5dB 3e -0.0kHz gps1x1".parse::<AprsPacket>().unwrap();
        assert_eq!(result.from, Callsign::new("ICA3D17F2"));
        assert_eq!(result.to, Callsign::new("APRS"));
        assert_eq!(
            result.via,
            vec![Callsign::new("qAS"), Callsign::new("dl4mea"),]
        );

        match result.data {
            AprsData::Position(position) => {
                assert_eq!(position.timestamp, Some(Timestamp::HHMMSS(7, 48, 49)));
                assert_relative_eq!(*position.latitude, 48.36023333333334);
                assert_relative_eq!(*position.longitude, 12.408266666666666);
                /*assert_eq!(
                    position.comment,
                    "322/103/A=003054 !W09! id213D17F2 -039fpm +0.0rot 2.5dB 3e -0.0kHz gps1x1"
                );*/
            }
            _ => panic!("Unexpected data type"),
        }
    }

    #[test]
    fn parse_non_ascii_message() {
        let result =
            r#"FNTFC070C>OGNFNT,qAS,LSXI1:>231159h Name="Höhematte holfuy" sF1 cr4 6.2dB -12.9kHz"#
                .parse::<AprsPacket>();
        assert!(result.is_ok());

        let packet = result.unwrap();
        assert_eq!(packet.from, Callsign::new("FNTFC070C"));
        assert_eq!(packet.to, Callsign::new("OGNFNT"));
        assert_eq!(packet.data_source(), Some(DataSource::OgnFnt));
        assert_eq!(
            packet.via,
            vec![Callsign::new("qAS"), Callsign::new("LSXI1")]
        );

        match packet.data {
            AprsData::Status(status) => {
                assert_eq!(status.timestamp, Some(Timestamp::HHMMSS(23, 11, 59)));
                assert_eq!(status.comment.name, Some("Höhematte holfuy".to_string()));
            }
            _ => panic!("Expected Status data type"),
        }
    }

    #[test]
    fn parse_message() {
        let result =
            r"ICA3D17F2>Aprs,qAS,dl4mea::DEST     :Hello World! This msg has a : colon {32975"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(result.from, Callsign::new("ICA3D17F2"));
        assert_eq!(result.to, Callsign::new("Aprs"));
        assert_eq!(
            result.via,
            vec![Callsign::new("qAS"), Callsign::new("dl4mea"),]
        );

        match result.data {
            AprsData::Message(msg) => {
                assert_eq!(msg.addressee, "DEST");
                assert_eq!(msg.text, "Hello World! This msg has a : colon ");
                assert_eq!(msg.id, Some(32975));
            }
            _ => panic!("Unexpected data type"),
        }
    }

    #[test]
    fn parse_status() {
        let result = r"ICA3D17F2>APRS,qAS,dl4mea:>312359zStatus seems okay!"
            .parse::<AprsPacket>()
            .unwrap();
        assert_eq!(result.from, Callsign::new("ICA3D17F2"));
        assert_eq!(result.to, Callsign::new("APRS"));
        assert_eq!(
            result.via,
            vec![Callsign::new("qAS"), Callsign::new("dl4mea"),]
        );

        match result.data {
            AprsData::Status(msg) => {
                assert_eq!(msg.timestamp, Some(Timestamp::DDHHMM(31, 23, 59)));
                assert_eq!(msg.comment.unparsed.unwrap(), "Status seems okay!");
            }
            _ => panic!("Unexpected data type"),
        }
    }

    #[ignore = "status_comment and position_comment serialization not implemented"]
    #[test]
    fn e2e_serialize_deserialize() {
        let valids = vec![
            r"ICA3D17F2>APRS,qAS,dl4mea:/074849h4821.61N\01224.49E^322/103/A=003054 !W09! id213D17F2 -039fpm +0.0rot 2.5dB 3e -0.0kHz gps1x1",
            r"ICA3D17F2>APRS,qAS,dl4mea:@074849h4821.61N\01224.49E^322/103/A=003054 !W09! id213D17F2 -039fpm +0.0rot 2.5dB 3e -0.0kHz gps1x1",
            r"ICA3D17F2>APRS,qAS,dl4mea:!4821.61N\01224.49E^322/103/A=003054 !W09! id213D17F2 -039fpm +0.0rot 2.5dB 3e -0.0kHz gps1x1",
            r"ICA3D17F2>APRS,qAS,dl4mea:=4821.61N\01224.49E^322/103/A=003054 !W09! id213D17F2 -039fpm +0.0rot 2.5dB 3e -0.0kHz gps1x1",
            r"ICA3D17F2>Aprs,qAS,dl4mea::DEST     :Hello World! This msg has a : colon {32975",
            r"ICA3D17F2>Aprs,qAS,dl4mea::DESTINATI:Hello World! This msg has a : colon ",
            r"ICA3D17F2>APRS,qAS,dl4mea:>312359zStatus seems okay!",
            r"ICA3D17F2>APRS,qAS,dl4mea:>184050hAlso with HMS format...",
        ];

        for v in valids {
            let mut buf = String::new();
            v.parse::<AprsPacket>().unwrap().encode(&mut buf).unwrap();
            assert_eq!(buf, v)
        }
    }

    #[test]
    fn test_data_source_from_str() {
        // Test a few key data sources
        assert_eq!("OGNFLR".parse::<DataSource>().unwrap(), DataSource::OgnFlr);
        assert_eq!("OGFLR6".parse::<DataSource>().unwrap(), DataSource::OgFlr6);
        assert_eq!("OGFLR7".parse::<DataSource>().unwrap(), DataSource::OgFlr7);
        assert_eq!("OGNTRK".parse::<DataSource>().unwrap(), DataSource::OgnTrk);
        assert_eq!("OGADSB".parse::<DataSource>().unwrap(), DataSource::OgAdsb);
        assert_eq!("FXCAPP".parse::<DataSource>().unwrap(), DataSource::FxcApp);
        assert_eq!("OGNVOL".parse::<DataSource>().unwrap(), DataSource::OgnVol);

        // Test unknown data source
        assert!("UNKNOWN".parse::<DataSource>().is_err());
        assert!("".parse::<DataSource>().is_err());
    }

    #[test]
    fn test_data_source_display() {
        // Test display formatting
        assert_eq!(format!("{}", DataSource::OgnFlr), "OGNFLR");
        assert_eq!(format!("{}", DataSource::OgFlr6), "OGFLR6");
        assert_eq!(format!("{}", DataSource::OgFlr7), "OGFLR7");
        assert_eq!(format!("{}", DataSource::OgnTrk), "OGNTRK");
        assert_eq!(format!("{}", DataSource::OgAdsb), "OGADSB");
        assert_eq!(format!("{}", DataSource::FxcApp), "FXCAPP");
        assert_eq!(format!("{}", DataSource::OgnVol), "OGNVOL");
    }

    #[test]
    fn test_data_source_roundtrip() {
        // Test that all variants can roundtrip through string conversion
        let test_cases = [
            DataSource::OgnFlr,
            DataSource::OgFlr6,
            DataSource::OgFlr7,
            DataSource::OgFlr,
            DataSource::OgnDsx,
            DataSource::OgnTrk,
            DataSource::OgAdsl,
            DataSource::OgAdsb,
            DataSource::OgnFnt,
            DataSource::OgnPaw,
            DataSource::OgSpot,
            DataSource::OgSpid,
            DataSource::OgLt24,
            DataSource::OgSkyl,
            DataSource::OgCapt,
            DataSource::OgnAvi,
            DataSource::OgnMav,
            DataSource::OgFlym,
            DataSource::OgnInRe,
            DataSource::OgEvario,
            DataSource::OgnDelay,
            DataSource::OgPaw,
            DataSource::OgnTtn,
            DataSource::OgnHel,
            DataSource::OgAvz,
            DataSource::OgnSky,
            DataSource::OgnMkt,
            DataSource::OgnEmo,
            DataSource::OgnMyc,
            DataSource::OgStux,
            DataSource::OgnSxr,
            DataSource::OgAirm,
            DataSource::FxcApp,
            DataSource::OgApik,
            DataSource::OgMsht,
            DataSource::OgnDvs,
            DataSource::OgnWmn,
            DataSource::OgnVol,
        ];

        for data_source in test_cases {
            let string_repr = format!("{data_source}");
            let parsed = string_repr.parse::<DataSource>().unwrap();
            assert_eq!(data_source, parsed, "Failed roundtrip for {string_repr}");
        }
    }

    #[test]
    fn test_device_type_method() {
        // Test with known data sources
        let packet_ognflr =
            "ICA3D17F2>OGNFLR,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(packet_ognflr.data_source(), Some(DataSource::OgnFlr));

        let packet_ogadsb =
            "ICA3D17F2>OGADSB,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(packet_ogadsb.data_source(), Some(DataSource::OgAdsb));

        let packet_fxcapp =
            "ICA3D17F2>FXCAPP,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(packet_fxcapp.data_source(), Some(DataSource::FxcApp));

        // Test with unknown data source
        let packet_unknown =
            "ICA3D17F2>APRS,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(packet_unknown.data_source(), None);

        // Test with completely unknown TO field
        let packet_custom =
            "ICA3D17F2>CUSTOM123,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(packet_custom.data_source(), None);
    }

    #[test]
    fn test_ognsdr_status_packet() {
        let result = r"Herborn>OGNSDR,TCPIP*,qAC,GLIDERN5:>225942h v0.3.2.arm64 CPU:0.7 RAM:790.3/3976.3MB NTP:0.3ms/-15.3ppm +61.3C EGM96:+49m 0/0Acfts[1h] RF:+0+0.0ppm/+4.10dB/+2.9dB@10km[751698]/+10.1dB@10km[1/2]"
            .parse::<AprsPacket>()
            .unwrap();

        assert_eq!(result.from, Callsign::new("Herborn"));
        assert_eq!(result.to, Callsign::new("OGNSDR"));
        assert_eq!(
            result.via,
            vec![
                Callsign::new("TCPIP*"),
                Callsign::new("qAC"),
                Callsign::new("GLIDERN5")
            ]
        );

        match result.data {
            AprsData::Status(status) => {
                assert_eq!(status.timestamp, Some(Timestamp::HHMMSS(22, 59, 42)));
                assert_eq!(status.comment.version, Some("0.3.2".to_string()));
                assert_eq!(status.comment.platform, Some("arm64".to_string()));
                assert_eq!(status.comment.geoid_offset, Some(49));
                assert_eq!(status.comment.unparsed, None);
            }
            _ => panic!("Expected Status data type"),
        }
    }

    #[test]
    fn test_ognsky_negative_altitude_packet() {
        let result = r"SKYF63E59>OGNSKY,qAS,SafeSky:/183123h4547.25N/01250.21E'288/044/A=-00006 !W25! id20F63E59 +000fpm gps4x3"
            .parse::<AprsPacket>()
            .unwrap();

        assert_eq!(result.from, Callsign::new("SKYF63E59"));
        assert_eq!(result.to, Callsign::new("OGNSKY"));
        assert_eq!(result.data_source(), Some(DataSource::OgnSky));
        assert_eq!(
            result.via,
            vec![Callsign::new("qAS"), Callsign::new("SafeSky")]
        );

        match result.data {
            AprsData::Position(position) => {
                assert_eq!(position.timestamp, Some(Timestamp::HHMMSS(18, 31, 23)));
                assert_relative_eq!(*position.latitude, 45.787533333333336);
                assert_relative_eq!(*position.longitude, 12.836916666666667);
                assert_eq!(position.comment.course, Some(288));
                assert_eq!(position.comment.speed, Some(44));
                assert_eq!(position.comment.altitude, Some(-6)); // Negative altitude!
                assert_eq!(position.comment.climb_rate, Some(0));
                assert_eq!(position.comment.gps_quality, Some("4x3".to_string()));
                assert_eq!(position.comment.gnss_horizontal_resolution, Some(4));
                assert_eq!(position.comment.gnss_vertical_resolution, Some(3));
                assert_eq!(position.comment.unparsed, None);
            }
            _ => panic!("Expected Position data type"),
        }
    }

    #[test]
    fn test_ognpur_decimal_course_packet() {
        let result = r"PUR64020B>OGNPUR,qAS,PureTrk23:/142436h4546.60N/01146.10Eg166.56186289668/018/A=002753 !W64! id1E64020B +000fpm +0.0rot 0.0dB 0e +0.0kHz gps2x3"
            .parse::<AprsPacket>()
            .unwrap();

        assert_eq!(result.from, Callsign::new("PUR64020B"));
        assert_eq!(result.to, Callsign::new("OGNPUR"));
        assert_eq!(result.data_source(), None); // OGNPUR is not a recognized data source
        assert_eq!(
            result.via,
            vec![Callsign::new("qAS"), Callsign::new("PureTrk23")]
        );

        match result.data {
            AprsData::Position(position) => {
                assert_eq!(position.timestamp, Some(Timestamp::HHMMSS(14, 24, 36)));
                assert_relative_eq!(*position.latitude, 45.77676666666667);
                assert_relative_eq!(*position.longitude, 11.768400000000002);
                assert_eq!(position.comment.course, Some(167)); // Decimal 166.56186289668 rounded to 167
                assert_eq!(position.comment.speed, Some(18));
                assert_eq!(position.comment.altitude, Some(2753));
                assert_eq!(position.comment.climb_rate, Some(0));
                assert_eq!(position.comment.turn_rate, Decimal::from_f32(0.0));
                assert_eq!(position.comment.signal_quality, Decimal::from_f32(0.0));
                assert_eq!(position.comment.error, Some(0));
                assert_eq!(position.comment.frequency_offset, Decimal::from_f32(0.0));
                assert_eq!(position.comment.gps_quality, Some("2x3".to_string()));
                assert_eq!(position.comment.unparsed, None);
            }
            _ => panic!("Expected Position data type"),
        }
    }

    #[test]
    fn test_position_source_type() {
        use crate::position::PositionSourceType;

        // Test receiver packet
        let receiver_packet =
            "AVX1053>OGNSDR,TCPIP*,qAC,GLIDERN3:/190916h6022.40NI00512.27E&/A=000049"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(
            receiver_packet.position_source_type(),
            PositionSourceType::Receiver
        );

        // Test aircraft packet
        let aircraft_packet =
            "ICA3D17F2>APRS,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(
            aircraft_packet.position_source_type(),
            PositionSourceType::Aircraft
        );

        // Test unknown packet
        let unknown_packet =
            "TEST123>APRS,WIDE1-1,WIDE2-1:/074849h4821.61N\\01224.49E^322/103/A=003054"
                .parse::<AprsPacket>()
                .unwrap();
        assert_eq!(
            unknown_packet.position_source_type(),
            PositionSourceType::Unknown
        );

        // Test non-position packet
        let status_packet = "TEST123>APRS,WIDE1-1:>Status message"
            .parse::<AprsPacket>()
            .unwrap();
        assert_eq!(
            status_packet.position_source_type(),
            PositionSourceType::NotPosition
        );
    }

    #[test]
    fn test_device_type_comprehensive() {
        // Demonstrate the comprehensive functionality with various data sources
        let test_packets = vec![
            (
                "ICA3D17F2>OGNFLR,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                Some(DataSource::OgnFlr),
            ),
            (
                "ICA3D17F2>OGFLR6,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                Some(DataSource::OgFlr6),
            ),
            (
                "ICA3D17F2>OGADSB,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                Some(DataSource::OgAdsb),
            ),
            (
                "ICA3D17F2>OGNTRK,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                Some(DataSource::OgnTrk),
            ),
            (
                "ICA3D17F2>FXCAPP,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                Some(DataSource::FxcApp),
            ),
            (
                "ICA3D17F2>OGNVOL,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                Some(DataSource::OgnVol),
            ),
            (
                "ICA3D17F2>APRS,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                None,
            ),
            (
                "ICA3D17F2>UNKNOWN,qAS,dl4mea:/074849h4821.61N\\01224.49E^322/103/A=003054",
                None,
            ),
        ];

        for (packet_str, expected) in test_packets {
            let packet = packet_str.parse::<AprsPacket>().unwrap();
            assert_eq!(
                packet.data_source(),
                expected,
                "Failed for packet with TO field: {}",
                packet.to
            );
        }
    }

    #[test]
    fn test_packet_with_malformed_name_field() {
        // Test the problematic message that caused the panic
        let result = r#"FNT4A08CC>OGNFNT,qAS,Innichen:>215504h Name=""#.parse::<AprsPacket>();

        assert!(result.is_ok());
        let packet = result.unwrap();

        assert_eq!(packet.from, Callsign::new("FNT4A08CC"));
        assert_eq!(packet.to, Callsign::new("OGNFNT"));
        assert_eq!(packet.data_source(), Some(DataSource::OgnFnt));
        assert_eq!(
            packet.via,
            vec![Callsign::new("qAS"), Callsign::new("Innichen")]
        );

        match packet.data {
            AprsData::Status(status) => {
                assert_eq!(status.timestamp, Some(Timestamp::HHMMSS(21, 55, 4)));
                assert_eq!(status.comment.name, None); // Name should not be parsed due to malformed quotes
                assert_eq!(status.comment.unparsed, Some("Name=\"".to_string()));
            }
            _ => panic!("Expected Status data type"),
        }
    }
}
