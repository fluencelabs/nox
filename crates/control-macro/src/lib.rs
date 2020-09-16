/// Takes option as an argument, unwraps if `Some`, exit function with `Ok(default)` otherwise
/// Ought to make it easier to short-circuit in functions returning `Result<Option<_>>`
#[macro_export]
macro_rules! ok_get {
    ($opt:expr) => {{
        let r = { $opt };
        match r {
            Some(r) => r,
            None => return Ok(Default::default()),
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
