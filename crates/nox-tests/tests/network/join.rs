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

use fstrings::f;

pub fn join_stream(stream: &str, relay: &str, length: &str, result: &str) -> String {
    f!(r#"
        (new $monotonic_stream
            (seq
                (fold ${stream} elem
                    (seq
                        (ap elem $monotonic_stream)
                        (seq
                            (canon {relay} $monotonic_stream #canon_stream)
                            (xor
                                (match #canon_stream.length {length} 
                                    (null) ;; fold ends if there's no `next`
                                )
                                (next elem)
                            )
                        )
                    )
                )
                (canon {relay} ${stream} #{result})
            )
        )
    "#)
}
