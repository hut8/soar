use std::str::FromStr;

use serde::Serialize;

use crate::AprsError;

#[derive(PartialEq, Debug, Clone, Serialize)]
pub struct Comment {
    pub comment: String,
}

impl FromStr for Comment {
    type Err = AprsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Comment {
            comment: s.to_string(),
        })
    }
}
