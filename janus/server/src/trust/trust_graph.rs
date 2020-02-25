/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::trust::certificate::Certificate;
use crate::trust::trust::Trust;

#[allow(dead_code)]
type PublicKey = String;
#[allow(dead_code)]
type Revoke = String;
#[allow(dead_code)]
type FailureReason = String;

/// graph to effectively calculate weights of certificates and get chains of certificates
#[allow(dead_code)]
struct TrustGraph {}

/// Element of graph
#[allow(dead_code)]
struct TrustNode {
    /// identity key of this element
    pk: PublicKey,
    /// one public key could be revoked by multiple certificates
    revoked_by: Vec<Revoke>,
    /// one public key could be authorized by multiple certificates
    authorized_by: Vec<Auth>,
    /// for maintain
    verified_at: u64,
}

#[allow(dead_code)]
impl TrustNode {
    /// list of certificates that revoked this trust
    fn revoked_by(&self) -> &[Revoke] {
        &[]
    }

    /// list of certificates that authorized this trust
    fn authorized_by(&self) -> &[Auth] {
        &[]
    }

    /// last check
    fn verified_at(&self) -> u64 {
        1
    }
}

/// represents who give a certificate
#[allow(dead_code)]
struct Auth {
    ///
    trust: Trust,
    /// previous pk
    issued_by: PublicKey,
}
#[allow(dead_code)]
impl TrustGraph {
    /// Storing trust in graph
    fn store(&self, _node: TrustNode) {}

    /// Get trust by public key
    fn load(&self, _pk: PublicKey) -> TrustNode {
        TrustNode {
            pk: "".to_string(),
            revoked_by: Vec::new(),
            authorized_by: Vec::new(),
            verified_at: 0,
        }
    }

    /// Stream all trusts?
    fn stream(&self) -> &[TrustNode] {
        &[]
    }

    /// Certificate is a chain of trusts, add this chain to graph
    fn add(&self, _cert: Certificate) {}

    /// You have certificates that you trust.
    /// Get maximum weight (minimal length of chain) for a public key from these certificates
    fn weight(&self, _pk: PublicKey) -> u32 {
        1
    }

    /// Mark public key as revoked
    fn revoke(&self, _time: u64, _pk: PublicKey) -> Revoke {
        "".to_string()
    }

    /// Check if certificate is revoked?
    fn revoked(_revoke: Revoke) {}

    /// Check information about new certificates and about revoked certificates.
    /// Do it once per some time
    fn maintain() {}
}
