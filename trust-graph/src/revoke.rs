/*
 *   MIT License
 *
 *   Copyright (c) 2020 Fluence Labs Limited
 *
 *   Permission is hereby granted, free of charge, to any person obtaining a copy
 *   of this software and associated documentation files (the "Software"), to deal
 *   in the Software without restriction, including without limitation the rights
 *   to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 *   copies of the Software, and to permit persons to whom the Software is
 *   furnished to do so, subject to the following conditions:
 *
 *   The above copyright notice and this permission notice shall be included in all
 *   copies or substantial portions of the Software.
 *
 *   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 *   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 *   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 *   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 *   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 *   OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 *   SOFTWARE.
 */

use crate::ed25519::PublicKey;
use crate::key_pair::KeyPair;
use crate::key_pair::Signature;
use crate::trust::{EXPIRATION_LEN, PUBLIC_KEY_LEN};
use std::time::Duration;

/// "A document" that cancels trust created before.
/// TODO delete pk from Revoke (it is already in a trust node)
#[derive(Debug, Clone)]
pub struct Revoke {
    /// who is revoked
    pub pk: PublicKey,
    /// date when revocation was created
    pub revoked_at: Duration,
    /// the issuer of this revocation
    pub revoked_by: PublicKey,
    /// proof of this revocation
    signature: Signature,
}

impl Revoke {
    #[allow(dead_code)]
    fn new(
        pk: PublicKey,
        revoked_by: PublicKey,
        revoked_at: Duration,
        signature: Signature,
    ) -> Self {
        Self {
            pk,
            revoked_at,
            revoked_by,
            signature,
        }
    }

    /// Creates new revocation signed by a revoker.
    #[allow(dead_code)]
    pub fn create(revoker: &KeyPair, to_revoke: PublicKey, revoked_at: Duration) -> Self {
        let msg = Revoke::signature_bytes(&to_revoke, revoked_at);
        let signature = revoker.sign(&msg);

        Revoke::new(to_revoke, revoker.public_key(), revoked_at, signature)
    }

    fn signature_bytes(pk: &PublicKey, revoked_at: Duration) -> Vec<u8> {
        let mut msg = Vec::with_capacity(PUBLIC_KEY_LEN + EXPIRATION_LEN);
        msg.extend_from_slice(&pk.encode());
        msg.extend_from_slice(&(revoked_at.as_millis() as u64).to_le_bytes());

        msg
    }

    /// Verifies that revocation is cryptographically correct.
    pub fn verify(revoke: &Revoke) -> Result<(), String> {
        let msg = Revoke::signature_bytes(&revoke.pk, revoke.revoked_at);

        if !revoke
            .revoked_by
            .verify(msg.as_slice(), revoke.signature.as_slice())
        {
            return Err("Revoke has incorrect signature.".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_gen_revoke_and_validate() {
        let revoker = KeyPair::generate();
        let to_revoke = KeyPair::generate();

        let duration = Duration::new(100, 0);

        let revoke = Revoke::create(&revoker, to_revoke.public_key(), duration);

        assert_eq!(Revoke::verify(&revoke).is_ok(), true);
    }

    #[test]
    fn test_validate_corrupted_revoke() {
        let revoker = KeyPair::generate();
        let to_revoke = KeyPair::generate();

        let duration = Duration::new(100, 0);

        let revoke = Revoke::create(&revoker, to_revoke.public_key(), duration);

        let duration2 = Duration::new(95, 0);
        let corrupted_revoke = Revoke::new(
            to_revoke.public_key(),
            revoker.public_key(),
            duration2,
            revoke.signature,
        );

        assert_eq!(Revoke::verify(&corrupted_revoke).is_ok(), false);
    }
}
