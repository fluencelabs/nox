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
