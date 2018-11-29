mod settings;
extern crate reikna;

use settings::FACTORIZED_NUMBER;
use reikna::prime;

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let factorized_number : u64 = FACTORIZED_NUMBER.parse::<u64>().unwrap();
    // reikna uses Atkin or Eratosthenes seive to factorize given number
    let factors = prime::factorize(factorized_number);

    factors[0]
}
