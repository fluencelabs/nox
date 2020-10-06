use serde_json::Value;
use std::fmt::Debug;

pub fn into_string(v: Value) -> Option<String> {
    if let Value::String(s) = v {
        return Some(s);
    }

    None
}

/// Converts an error into IValue::String
pub fn as_value<E: Debug>(err: E) -> Value {
    Value::String(format!("Error: {:?}", err))
}
