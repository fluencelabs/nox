extern crate fibonacci_bigint_wasm;

pub fn main() {
    std::process::exit(fibonacci_bigint_wasm::main() as i32)
}