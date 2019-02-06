#![feature(trace_macros)]

use proc_test::invoke_handler;

#[invoke_handler]
fn invoke(arg: ::std::string::String) -> ::std::string::String {

}


fn main() {
    tt();
}
