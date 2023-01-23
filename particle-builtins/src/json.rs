use eyre::eyre;
use particle_args::{Args, JError};
use serde_json::Value as JValue;

fn obj_from_iter(
    mut object: serde_json::Map<String, JValue>,
    args: &mut impl Iterator<Item = JValue>,
) -> Result<serde_json::Map<String, JValue>, JError> {
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

    Ok(object)
}

/// Constructs a JSON object from a list of key value pairs.
pub fn obj(args: Args) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();

    let object = obj_from_iter(<_>::default(), &mut args)?;

    Ok(JValue::Object(object))
}

/// Constructs a JSON object from a list of key value pairs.
pub fn obj_from_pairs(
    values: impl IntoIterator<Item = (String, JValue)>,
) -> Result<JValue, JError> {
    let map = values.into_iter().fold(
        <serde_json::Map<String, JValue>>::default(),
        |mut acc, (k, v)| {
            acc.insert(k, v);
            acc
        },
    );

    Ok(JValue::Object(map))
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

/// Inserts list of key value pairs into an object.
pub fn puts(args: Args) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let object = Args::next("object", &mut args)?;

    let object = obj_from_iter(object, &mut args)?;

    Ok(JValue::Object(object))
}

/// Inserts list of key value pairs into an object.
pub fn puts_from_array(
    object: JValue,
    values: impl IntoIterator<Item = JValue>,
) -> Result<JValue, JError> {
    if let JValue::Object(map) = object {
        let object = obj_from_iter(map, &mut values.into_iter())?;
        Ok(JValue::Object(object))
    } else {
        Err(JError::new(format!("expected json object, got {}", object)))
    }
}

pub fn parse(json: &str) -> Result<JValue, JError> {
    serde_json::from_str(json).map_err(Into::into)
}

pub fn stringify(value: JValue) -> String {
    value.to_string()
}

#[cfg(test)]
mod tests {
    use crate::json::parse;

    #[test]
    fn json_parse_string() {
        use serde_json::json;

        let str = json!("hellow");
        let parsed = parse(&str.to_string());
        assert_eq!(parsed.ok(), Some(str));
    }
}
