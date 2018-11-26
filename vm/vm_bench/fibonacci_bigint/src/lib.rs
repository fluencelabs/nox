mod settings;
extern crate num_bigint;
extern crate num_traits;

use settings::{FIB_NUMBER};
use num_bigint::{BigInt};
use num_traits::{One, Zero};

#[no_mangle]
pub extern "C" fn fib(num: u64) -> BigInt {
    let mut f_0: BigInt = Zero::zero();
    let mut f_1: BigInt = One::one();

    for _ in 0..num {
        let f_2 = f_0 + &f_1;
        f_0 = f_1;
        f_1 = f_2;
    }

    f_0
}

#[no_mangle]
pub extern "C" fn bench_test() -> u8 {
    let fib_number : u64 = FIB_NUMBER.parse::<u64>().unwrap();

    fib(fib_number).to_bytes_le().1.iter().fold(0, |x1, x2| x1 ^ x2)
}
