extern crate fluence;

mod utils;

use fluence::credentials::Credentials;
use utils::generate_register;

#[test]
fn publish_pinned() -> () {
    let reg = generate_register(Credentials::No);
}
