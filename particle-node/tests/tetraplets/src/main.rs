use fluence::{fce, SecurityTetraplet};

pub fn main() {}

#[fce]
pub fn get_tetraplets(_: String) -> Vec<Vec<SecurityTetraplet>> {
    fluence::get_call_parameters().tetraplets
}
