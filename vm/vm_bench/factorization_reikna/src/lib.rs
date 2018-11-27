mod settings;
extern crate reikna;

use settings::NUMBER;
use reikna::prime;

#[no_mangle]
pub extern "C" fn bench_test() -> u64 {
    let factors = prime::factorize(NUMBER);
    factors[0]
}