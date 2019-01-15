extern crate fluence;

mod utils;

use utils::generate_register;
use fluence::credentials::Credentials;

#[test]
fn publish_pinned() -> () {
    let reg = generate_register(Credentials::No);
}
