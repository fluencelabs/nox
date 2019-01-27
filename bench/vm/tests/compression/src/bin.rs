extern crate compression_wasm;

pub fn main() {
    std::process::exit(compression_wasm::main() as i32)
}