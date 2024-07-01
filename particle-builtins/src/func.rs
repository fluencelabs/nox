/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
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
