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

#[no_mangle]
pub fn next_row() -> i32 {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.next_row()
}

#[no_mangle]
pub fn next_field() -> i32 {
    let mut db = sql_db::DB.lock().expect("Can't take a Database.");
    db.next_field()
}

fn main() {
    // do nothing
}