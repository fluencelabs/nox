use eyre::eyre;
use particle_args::{Args, JError};
use serde_json::Value as JValue;

/// Constructs a JSON object from a list of key value pairs.
pub fn obj(args: Args) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let mut object = serde_json::Map::new();

    loop {
        match (args.next(), args.next()) {
            (Some(JValue::String(name)), Some(value)) => { object.insert(name, value); },
            (Some(key), None) => return Err(JError::new(eyre!(
                "Expected odd number of arguments, got even. No value for key '{}'",
                key
            ).to_string())),
            (Some(key), Some(value)) => return Err(JError::new(eyre!(
                "All keys must be of type string. Key of the following pair is of invalid type: ({}, {})",
                key,
                value
            ).to_string())),
            (None, _) => break,
        }
    }

    Ok(JValue::Object(object))
}

/// Inserts a value into a JSON object
pub fn put(args: Args) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let mut object: serde_json::Map<String, JValue> = Args::next("object", &mut args)?;
    let key = Args::next("key", &mut args)?;
    let value = Args::next("value", &mut args)?;

    object.insert(key, value);

    Ok(JValue::Object(object))
}

pub fn parse(json: &str) -> Result<JValue, JError> {
    serde_json::from_str(json).map_err(Into::into)
}

pub fn stringify(value: JValue) -> String {
    value.to_string()
}
