mod settings;
extern crate reikna;

use settings::NUMBER;
use reikna::prime;

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let number : u32 = NUMBER.parse::<u64>().unwrap();
    let factors = prime::factorize(number);
    factors[0]
}
