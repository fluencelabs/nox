
#[macro_use]
extern crate lazy_static;

mod sql_db;

/// Public function for export


/// `select * from Table`
#[no_mangle]
pub fn set_query_wildcard() {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_query_wildcard();
}

/// `select * from Table where {FIELD} = {VALUE}`
#[no_mangle]
pub fn set_query_wildcard_where(field_idx: usize, field_val: i32, compare_mode: i8) {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_query_wildcard_where(field_idx, field_val, compare_mode);
}

/// `select count(*) from Table`
#[no_mangle]
pub fn set_count_query() {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_count_query();
}

/// `select count(*) from Table where {FIELD} = {VALUE}`
#[no_mangle]
pub fn set_count_query_where(field_idx: usize, field_val: i32, compare_mode: i8) {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_count_query_where(field_idx, field_val, compare_mode);
}

/// `select avg({FIELD}) from Table`
#[no_mangle]
pub fn set_average_query(avg_field_idx: usize) {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_average_query(avg_field_idx);
}

/// `select avg(*) from Table where {FIELD} = {VALUE}`
#[no_mangle]
pub fn set_average_query_where(
    avg_field_idx: usize,
    field_idx: usize,
    field_val: i32,
    compare_mode: i8
) {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_average_query_where(avg_field_idx, field_idx, field_val, compare_mode);
}

/// Returns an serial number of the next row or -1 otherwise
#[no_mangle]
pub fn next_row() -> i32 {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.next_row()
}

/// Returns a next field value for the current row
#[no_mangle]
pub fn next_field() -> f64 {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.next_field()
}

fn main() {
    // do nothing
}