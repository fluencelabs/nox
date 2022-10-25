use particle_args::JError;
use particle_execution::FunctionOutcome;
use serde_json::Value as JValue;

pub fn ok(v: JValue) -> FunctionOutcome {
    FunctionOutcome::Ok(v)
}

pub fn wrap(r: Result<JValue, JError>) -> FunctionOutcome {
    match r {
        Ok(v) => FunctionOutcome::Ok(v),
        Err(err) => FunctionOutcome::Err(err),
    }
}

pub fn wrap_unit(r: Result<(), JError>) -> FunctionOutcome {
    match r {
        Ok(_) => FunctionOutcome::Empty,
        Err(err) => FunctionOutcome::Err(err),
    }
}
