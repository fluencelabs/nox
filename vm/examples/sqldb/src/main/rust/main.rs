#![feature(type_ascription)]

#[macro_use]
extern crate lazy_static;

mod sql_db;

use sql_db::Db;

/// Public function for export


/// `select * from Table`
#[no_mangle]
pub fn set_query_wildcard() {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_query_wildcard();
}

/// `select * from Table where {FIELD} = {VALUE}`
#[no_mangle]
#[feature(type_ascription)]
pub fn set_query_wildcard_where(field_idx: usize, field_val: i32) {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.set_query_wildcard_where(field_idx: usize, field_val: i32);
}

/// Returns an serial number of the next row or -1 otherwise
#[no_mangle]
pub fn next_row() -> i32 {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.next_row()
}

/// Returns a next field value for the current row
#[no_mangle]
pub fn next_field() -> i32 {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.next_field()
}

fn main() {
    // do nothing
}