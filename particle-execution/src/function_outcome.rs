/*
 * Copyright 2021 Fluence Labs Limited
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

use std::convert::Infallible;
use std::future::Future;
use std::ops::{ControlFlow, FromResidual, Try};
use std::process::Output;

use serde_json::Value as JValue;

use json_utils::err_as_value;
use particle_args::{Args, JError};

use crate::ParticleParams;

#[derive(Debug, Clone)]
pub enum FunctionOutcome {
    NotDefined { args: Args, params: ParticleParams },
    Empty,
    Ok(JValue),
    Err(JError),
}

impl FunctionOutcome {
    /// Returns [false] if variant is [NotDefined]
    pub fn is_defined(&self) -> bool {
        !matches!(self, Self::NotDefined { .. })
    }

    /// Returns [false] if variant is [Err]
    pub fn is_err(&self) -> bool {
        !matches!(self, Self::Err { .. })
    }

    pub fn or_else(self, f: impl FnOnce(Args, ParticleParams) -> Self) -> Self {
        if let FunctionOutcome::NotDefined { args, params } = self {
            f(args, params)
        } else {
            self
        }
    }
}

impl<E: std::error::Error> From<E> for FunctionOutcome {
    fn from(err: E) -> Self {
        FunctionOutcome::Err(JError(err_as_value(err)))
    }
}

impl Try for FunctionOutcome {
    type Output = Option<JValue>;
    type Residual = JError;

    fn from_output(output: Self::Output) -> Self {
        match output {
            Some(v) => FunctionOutcome::Ok(v),
            None => FunctionOutcome::Empty,
        }
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            FunctionOutcome::Err(err) => ControlFlow::Break(err),
            FunctionOutcome::Empty | FunctionOutcome::NotDefined { .. } => {
                ControlFlow::Continue(None)
            }
            FunctionOutcome::Ok(v) => ControlFlow::Continue(Some(v)),
        }
    }
}

impl FromResidual for FunctionOutcome {
    fn from_residual(residual: JError) -> Self {
        FunctionOutcome::Err(residual)
    }
}

impl<E: Into<JError>> FromResidual<Result<Infallible, E>> for FunctionOutcome {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        let Err(e) = residual;
        FunctionOutcome::Err(e.into())
    }
}
