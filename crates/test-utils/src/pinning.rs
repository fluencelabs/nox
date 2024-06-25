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

use cpu_utils::pinning::ThreadPinner;
use cpu_utils::LogicalCoreId;

pub const DUMMY: DummyThreadPinner = DummyThreadPinner {};

pub struct DummyThreadPinner;

impl ThreadPinner for DummyThreadPinner {
    #[inline]
    fn pin_current_thread_to(&self, _core_id: LogicalCoreId) -> bool {
        true
    }

    #[inline]
    fn pin_current_thread_to_cpuset(&self, _core_ids: &[LogicalCoreId]) -> bool {
        true
    }
}
