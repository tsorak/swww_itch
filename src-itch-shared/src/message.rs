use serde::{Deserialize, Serialize};

pub use rearrange::Position;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Request {
    SwitchToBackground(String),
    RearrangeBackground((String, Position, String)),
    GetQueue,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Response {
    SwitchToBackground(bool),
    RearrangeBackground((bool, usize, usize)),
    GetQueue(Vec<String>),
}

impl Request {
    pub fn rearrange_background(
        bg: String,
        position: String,
        target_bg: String,
    ) -> Result<Request, &'static str> {
        let position = Position::try_from(position.as_str())?;
        Ok(Request::RearrangeBackground((bg, position, target_bg)))
    }
}

mod rearrange {
    use super::*;

    #[derive(Serialize, Deserialize, Clone)]
    #[serde(rename_all = "camelCase")]
    pub enum Position {
        Before,
        After,
    }

    impl TryFrom<&str> for Position {
        type Error = &'static str;

        fn try_from(value: &str) -> Result<Self, Self::Error> {
            match value {
                "before" => Ok(Position::Before),
                "after" => Ok(Position::After),
                _ => Err("Position must be 'before' or 'after'"),
            }
        }
    }

    impl std::fmt::Display for Position {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Position::Before => write!(f, "before"),
                Position::After => write!(f, "after"),
            }
        }
    }
}
