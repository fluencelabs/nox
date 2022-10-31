use serde::{Deserialize, Serialize};
use serde_json::json;

use particle_args::{Args, JError};
use particle_execution::FunctionOutcome;

pub fn unary<X, Out, F>(args: Args, f: F) -> FunctionOutcome
where
    X: for<'de> Deserialize<'de>,
    Out: Serialize,
    F: Fn(X) -> Result<Out, JError>,
{
    if args.function_args.len() != 1 {
        let err = format!("expected 1 arguments, got {}", args.function_args.len());
        return FunctionOutcome::Err(JError::new(err));
    }
    let mut args = args.function_args.into_iter();

    let x: X = Args::next("x", &mut args)?;
    let out = f(x)?;
    FunctionOutcome::Ok(json!(out))
}

pub fn binary<X, Y, Out, F>(args: Args, f: F) -> FunctionOutcome
where
    X: for<'de> Deserialize<'de>,
    Y: for<'de> Deserialize<'de>,
    Out: Serialize,
    F: Fn(X, Y) -> Result<Out, JError>,
{
    if args.function_args.len() != 2 {
        let err = format!("expected 2 arguments, got {}", args.function_args.len());
        return FunctionOutcome::Err(JError::new(err));
    }
    let mut args = args.function_args.into_iter();

    let x: X = Args::next("x", &mut args)?;
    let y: Y = Args::next("y", &mut args)?;
    let out = f(x, y)?;
    FunctionOutcome::Ok(json!(out))
}
