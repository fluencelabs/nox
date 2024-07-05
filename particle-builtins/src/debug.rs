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

use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::builtins::CustomService;

pub fn fmt_custom_services(
    services: &RwLock<HashMap<String, CustomService>>,
    fmt: &mut std::fmt::Formatter<'_>,
) -> Result<(), std::fmt::Error> {
    fmt.debug_map()
        .entries(
            services
                .blocking_read()
                .iter()
                .map(|(sid, fs)| (sid, fs.functions.keys().collect::<Vec<_>>())),
        )
        .finish()
}
