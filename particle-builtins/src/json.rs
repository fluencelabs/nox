use eyre::eyre;
use particle_args::{Args, JError};
use serde_json::Value as JValue;

fn insert_pairs(
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

    let object = insert_pairs(<_>::default(), &mut args)?;

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

/// Inserts list of key value pairs into an object.
pub fn puts(args: Args) -> Result<JValue, JError> {
    let mut args = args.function_args.into_iter();
    let object = Args::next("object", &mut args)?;

    let object = insert_pairs(object, &mut args)?;

    Ok(JValue::Object(object))
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
