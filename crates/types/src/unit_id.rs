use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct UnitId(String);

impl From<String> for UnitId {
    fn from(value: String) -> Self {
        UnitId(value)
    }
}

impl From<&str> for UnitId {
    fn from(value: &str) -> Self {
        UnitId(value.to_string())
    }
}
