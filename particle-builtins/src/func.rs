/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

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
