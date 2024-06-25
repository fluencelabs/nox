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

use std::convert::Infallible;
use std::ops::{ControlFlow, FromResidual, Try};

use serde_json::Value as JValue;

use json_utils::err_as_value;
use particle_args::{Args, JError};

use crate::ParticleParams;

#[allow(clippy::large_enum_variant)]
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
    pub fn not_err(&self) -> bool {
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
