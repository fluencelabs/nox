#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;

use rocket::local::{Client, LocalRequest, LocalResponse};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

fn main() {
    let rocket = rocket::ignite();
    let client = Client::new(rocket).expect("valid rocket instance");
    let req = client.get("/");
    let response = req.dispatch();
}
