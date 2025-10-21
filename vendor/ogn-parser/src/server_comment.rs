use std::net::IpAddr;
use std::str::FromStr;

use chrono::prelude::*;
use serde::Serialize;

use crate::AprsError;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct ServerComment {
    pub version: String,
    pub timestamp: DateTime<Utc>,
    pub server: String,
    pub ip_address: IpAddr,
    pub port: u16,
}

impl FromStr for ServerComment {
    type Err = AprsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split_whitespace();

        // Check for "# aprsc" prefix
        if parts.next() != Some("#") || parts.next() != Some("aprsc") {
            return Err(AprsError::InvalidServerComment(s.to_string()));
        }

        // Get the software version
        let version = parts
            .next()
            .ok_or_else(|| AprsError::InvalidServerComment(s.to_string()))?;

        // Parse the timestamp
        let timestamp_str = parts.by_ref().take(5).collect::<Vec<_>>().join(" ");
        let timestamp = DateTime::parse_from_rfc2822(&timestamp_str)
            .map_err(|_| AprsError::InvalidServerComment(s.to_string()))?
            .with_timezone(&Utc);

        // Get the server name
        let server = parts
            .next()
            .ok_or_else(|| AprsError::InvalidServerComment(s.to_string()))?;

        // Get the IP address and port
        let address = parts
            .next()
            .ok_or_else(|| AprsError::InvalidServerComment(s.to_string()))?;
        let (ip_address, port) = address
            .split_once(':')
            .ok_or_else(|| AprsError::InvalidServerComment(address.to_string()))?;
        let ip_address = ip_address
            .parse::<IpAddr>()
            .map_err(|_| AprsError::InvalidServerComment(s.to_string()))?;
        let port = port
            .parse::<u16>()
            .map_err(|_| AprsError::InvalidServerComment(s.to_string()))?;

        Ok(ServerComment {
            version: version.to_string(),
            timestamp,
            server: server.to_string(),
            ip_address,
            port,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_server_comment() {
        let raw_message =
            r"# aprsc 2.1.4-g408ed49 17 Mar 2018 09:30:36 GMT GLIDERN1 37.187.40.234:10152";
        let result = raw_message.parse::<ServerComment>().unwrap();

        assert_eq!(result.version, "2.1.4-g408ed49");
        assert_eq!(
            result.timestamp.to_string(),
            "2018-03-17 09:30:36 UTC".to_string()
        );
        assert_eq!(result.server, "GLIDERN1");
        assert_eq!(
            result.ip_address,
            IpAddr::V4(Ipv4Addr::new(37, 187, 40, 234))
        );
        assert_eq!(result.port, 10152);
    }
}
