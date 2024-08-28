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

#![cfg_attr(target_os = "linux", allow(unused_imports))]

#[cfg(not(target_os = "linux"))]
pub type IPTables = ();

use crate::{IpTablesError, IpTablesRules, NetworkSettings, NetworkSetupError, RulesSet};

#[cfg(not(target_os = "linux"))]
pub fn clear_rules(_: &IPTables, _: &RulesSet) -> Result<(), NetworkSetupError> { Ok(()) }
#[cfg(not(target_os = "linux"))]
pub fn clear_network(_: &NetworkSettings, _: &str) -> Result<(), NetworkSetupError> { Ok(())}
#[cfg(not(target_os = "linux"))]
pub fn setup_network(_: &NetworkSettings, _: &str) -> Result<(), NetworkSetupError> { Ok(()) }
#[cfg(not(target_os = "linux"))]
pub fn clear_existing_chain_rules(_: &IPTables, _: &IpTablesRules) -> Result<(), IpTablesError> { Ok (()) }
#[cfg(not(target_os = "linux"))]
pub fn clear_new_chain_rules(_: &IPTables, _: &IpTablesRules) -> Result<(), IpTablesError> { Ok (()) }
#[cfg(not(target_os = "linux"))]
pub fn add_rules(_: &IPTables, _: &crate::RulesSet) -> Result<(), IpTablesError> { Ok(()) }
