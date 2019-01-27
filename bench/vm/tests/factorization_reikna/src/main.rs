pub mod bench;

pub fn main() {
    std::process::exit(bench::bench() as i32)
}