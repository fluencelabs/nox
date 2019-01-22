extern crate compression_wasm;
use compression_wasm::main;

pub fn main() {
    std::process::exit(main() as i32)
}