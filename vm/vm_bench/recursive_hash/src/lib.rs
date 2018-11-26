mod settings;

use settings::{INITIAL_VALUE, ITERATIONS_COUNT};
use sha3::{Digest, Sha3_256};

#[no_mangle]
pub fn bench_test() {
    let mut hasher = Sha3_256::new();
    hasher.input(b"asd");

//    for _ in 1..ITERATIONS_COUNT {
//        result = hasher.input(result);
//    }

    let result = hasher.result();
}
