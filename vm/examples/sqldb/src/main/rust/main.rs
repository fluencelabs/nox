
#[macro_use]
extern crate lazy_static;

use sql_db::Db;
use std::sync::MutexGuard;

mod sql_db;

/// Public function for export

/// `select * from Table`
#[no_mangle]
pub fn set_query_wildcard() {
    run_query(|mut db| db.set_query_wildcard())
}

/// `select * from Table where {FIELD} = {VALUE}`
#[no_mangle]
pub fn set_query_wildcard_where(field_idx: usize, field_val: i32, compare_mode: i8) {
    run_query(|mut db| db.set_query_wildcard_where(field_idx, field_val, compare_mode))
}

/// `select count(*) from Table`
#[no_mangle]
pub fn set_count_query() {
    run_query(|mut db| db.set_count_query())
}

/// `select count(*) from Table where {FIELD} = {VALUE}`
#[no_mangle]
pub fn set_count_query_where(field_idx: usize, field_val: i32, compare_mode: i8) {
    run_query(|mut db| db.set_count_query_where(field_idx, field_val, compare_mode));
}

/// `select avg({FIELD}) from Table`
#[no_mangle]
pub fn set_average_query(avg_field_idx: usize) {
    run_query(|mut db| db.set_average_query(avg_field_idx));
}

/// `select avg(*) from Table where {FIELD} = {VALUE}`
#[no_mangle]
pub fn set_average_query_where(
    avg_field_idx: usize,
    field_idx: usize,
    field_val: i32,
    compare_mode: i8
) {
    run_query(|mut db| db.set_average_query_where(avg_field_idx, field_idx, field_val, compare_mode));
}

/// Returns a serial number of the next row or -1 otherwise
#[no_mangle]
pub fn next_row() -> i32 {
    run_query(|mut db| db.next_row())
}

/// Returns a next field value for the current row
#[no_mangle]
pub fn next_field() -> f64 {
    run_query(|mut db| db.next_field())
}

// acquires lock, does query, releases lock, returns query result
fn run_query<T, F : FnOnce(MutexGuard<Db<i32>>) -> T>(query: F) -> T {
    query(sql_db::DB.lock().expect("Can't take a Database."))
}

fn main() {
    // do nothing
}
