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
