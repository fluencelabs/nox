use fluence;
use proc_test::invoke_handler;
use std::str::FromStr;

#[invoke_handler]
fn tt(arg: Vec<u8>) -> String {
    String::from("hello world")
}

fn main() {
    tt(vec![1, 2]);
}
