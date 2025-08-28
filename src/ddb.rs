use serde_derive::Deserialize;
use serde_derive::Serialize;

const DDB_URL: &str = "http://ddb.glidernet.org/download/?j=1";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub device_type: String,
    pub device_id: String,
    pub aircraft_model: String,
    pub registration: String,
    pub cn: String,
    pub tracked: String,
    pub identified: String,
}
