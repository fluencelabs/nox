mod settings;
extern crate num_bigint;
extern crate num_traits;

use settings::FIB_NUMBER;
use num_bigint::BigUint;
use num_traits::One;
use std::ops::Sub;

fn fib(num: &BigUint) -> BigUint {
    if num.le(&BigUint::from(2u32)) {
        return One::one();
    }

    fib(&num.sub(1u32)) + fib(&num.sub(2u32))
}

#[no_mangle]
pub extern "C" fn main() -> u8 {
    let fib_number : BigUint = BigUint::from(FIB_NUMBER.parse::<u64>().unwrap());

    fib(&fib_number).to_bytes_le().iter().fold(0u8, |x1, x2| x1 ^ x2)
}
