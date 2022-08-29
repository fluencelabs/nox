use std::collections::HashSet;
use std::ops::Mul;

use itertools::Itertools;

use particle_args::JError;

/// x + y
pub fn add(x: i64, y: i64) -> Result<i64, JError> {
    x.checked_add(y)
        .ok_or_else(|| JError::new("i64 add overflow"))
}

/// x - y
pub fn sub(x: i64, y: i64) -> Result<i64, JError> {
    let y = y
        .checked_neg()
        .ok_or_else(|| JError::new("i64 neg overflow"))?;
    x.checked_add(y)
        .ok_or_else(|| JError::new("i64 add overflow"))
}

/// x * y
pub fn mul(x: i64, y: i64) -> Result<i64, JError> {
    x.checked_mul(y)
        .ok_or_else(|| JError::new("i64 mul overflow"))
}

/// floor(x * y) (x and y can be float)
pub fn fmul_floor(x: f64, y: f64) -> Result<i64, JError> {
    Ok(x.mul(y).floor() as i64)
}

/// x / y
pub fn div(x: i64, y: i64) -> Result<i64, JError> {
    x.checked_div(y)
        .ok_or_else(|| JError::new("i64 div overflow"))
}

/// x % y (remainder)
pub fn rem(x: i64, y: i64) -> Result<i64, JError> {
    x.checked_rem(y)
        .ok_or_else(|| JError::new("i64 rem overflow"))
}

/// x ^ y
pub fn pow(x: i64, y: u32) -> Result<i64, JError> {
    x.checked_pow(y)
        .ok_or_else(|| JError::new("i64 pow overflow"))
}

/// log_x(y) (logarithm of base x)
pub fn log(x: i64, y: i64) -> Result<u32, JError> {
    y.checked_ilog(x)
        .ok_or_else(|| JError::new("i64 log overflow"))
}

/// x > y
pub fn gt(x: i64, y: i64) -> Result<bool, JError> {
    Ok(x.gt(&y))
}

/// x >= y
pub fn gte(x: i64, y: i64) -> Result<bool, JError> {
    Ok(x.ge(&y))
}

/// x < y
pub fn lt(x: i64, y: i64) -> Result<bool, JError> {
    Ok(x.lt(&y))
}

/// x <= y
pub fn lte(x: i64, y: i64) -> Result<bool, JError> {
    Ok(x.le(&y))
}

/// compare x and y
/// Less = -1
/// Equal = 0
/// Greater = 1
pub fn cmp(x: i64, y: i64) -> Result<i8, JError> {
    let ord = x.cmp(&y);
    Ok(ord as i8)
}

/// fold(_ + _) (sum of all numbers in array)
pub fn array_sum(xs: Vec<i64>) -> Result<i64, JError> {
    xs.into_iter()
        .try_fold(0, i64::checked_add)
        .ok_or_else(|| JError::new("i64 add overflow"))
}

/// remove duplicates, not stable
pub fn dedup(xs: Vec<String>) -> Result<Vec<String>, JError> {
    Ok(xs.into_iter().unique().collect())
}

/// set-intersection of two arrays, not stable, deduplicates
pub fn intersect(xs: HashSet<String>, ys: HashSet<String>) -> Result<Vec<String>, JError> {
    Ok(xs.intersection(&ys).cloned().collect())
}

/// set-difference of two arrays, not stable, deduplicates
pub fn diff(xs: HashSet<String>, ys: HashSet<String>) -> Result<Vec<String>, JError> {
    Ok(xs.difference(&ys).cloned().collect())
}

/// symmetric difference of two arrays, not stable, deduplicates
pub fn sdiff(xs: HashSet<String>, ys: HashSet<String>) -> Result<Vec<String>, JError> {
    Ok(xs.symmetric_difference(&ys).cloned().collect())
}
