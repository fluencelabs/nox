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
