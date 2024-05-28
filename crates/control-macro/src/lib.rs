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

#![warn(rust_2018_idioms)]
#![deny(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

/// Takes option as an argument, unwraps if `Some`, exit function with `Ok(default)` otherwise
/// Ought to make it easier to short-circuit in functions returning `Result<Option<_>>`
#[macro_export]
macro_rules! ok_get {
    ($opt:expr) => {{
        let r = { $opt };
        match r {
            Some(r) => r,
            None => return Ok(<_>::default()),
        }
    }};
}

/// Retrieves value from `Some`, returns on `None`
#[macro_export]
macro_rules! get_return {
    ($opt:expr) => {{
        let r = { $opt };
        match r {
            Some(r) => r,
            None => return,
        }
    }};
}

/// Retrieves value from `Some`, returns on `None`
#[macro_export]
macro_rules! unwrap_return {
    ($opt:expr, $alternative:expr) => {{
        let r = { $opt };
        match r {
            Some(r) => r,
            None => {
                let alt = { $alternative };
                return alt;
            }
        }
    }};
}

#[macro_export]
macro_rules! measure {
    ($val:expr) => {{
        let start = ::std::time::Instant::now();
        match $val {
            tmp => {
                let elapsed_ms = start.elapsed().as_millis();
                if elapsed_ms > 100 {
                    println!("{} took {} ms", stringify!($val), elapsed_ms);
                }
                tmp
            }
        }
    }};
}
