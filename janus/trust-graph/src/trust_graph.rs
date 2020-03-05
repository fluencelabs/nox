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

use crate::certificate::Certificate;
use crate::public_key_hashable::PublicKeyHashable;
use crate::revoke::Revoke;
use crate::trust_node::{Auth, TrustNode};
use libp2p_core::identity::ed25519::PublicKey;
use std::collections::hash_map::Entry;
use std::collections::hash_map::Entry::Occupied;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::time::Duration;

/// for simplicity, we store `n` where Weight = 1/n^2
type Weight = u32;

/// Graph to effectively calculate weights of certificates and get chains of certificates.
/// TODO serialization/deserialization
/// TODO export a certificate from graph
#[allow(dead_code)]
#[derive(Debug)]
struct TrustGraph {
    // TODO abstract this into a trait with key access methods
    nodes: HashMap<PublicKeyHashable, TrustNode>,
    root_weights: HashMap<PublicKeyHashable, Weight>,
}

#[allow(dead_code)]
impl TrustGraph {
    fn new(root_weights: HashMap<PublicKeyHashable, Weight>) -> Self {
        Self {
            nodes: HashMap::new(),
            root_weights,
        }
    }

    /// Get trust by public key
    pub fn get(&self, pk: PublicKey) -> Option<&TrustNode> {
        self.nodes.get(&pk.into())
    }

    /// Certificate is a chain of trusts, add this chain to graph
    pub fn add(&mut self, cert: Certificate, cur_time: Duration) -> Result<(), String> {
        let roots: Vec<PublicKey> = self.root_weights.keys().cloned().map(Into::into).collect();

        Certificate::verify(&cert, roots.as_slice(), cur_time)?;

        let chain = cert.chain;

        let root_trust = &chain[0];
        let root_pk: PublicKeyHashable = root_trust.issued_for.clone().into();

        match self.nodes.get_mut(&root_pk) {
            Some(_) => {}
            None => {
                let trust_node = TrustNode::new(root_trust.issued_for.clone(), cur_time);
                self.nodes.insert(root_pk, trust_node);
            }
        }

        let mut previous_trust = root_trust;

        for trust in chain.iter().skip(1) {
            let pk = trust.issued_for.clone().into();

            let auth = Auth {
                trust: trust.clone(),
                issued_by: previous_trust.issued_for.clone(),
            };

            match self.nodes.get_mut(&pk) {
                Some(trust_node) => {
                    trust_node.update_auth(auth);
                }
                None => {
                    let mut trust_node = TrustNode::new(root_trust.issued_for.clone(), cur_time);
                    trust_node.update_auth(auth);
                    self.nodes.insert(pk, trust_node);
                }
            }

            previous_trust = trust;
        }

        Ok(())
    }

    /// Calculates weight for a public key, ignore nodes, that already marked as passed.
    /// Returns weight (if none, this public key doesn't have a path to trusted root nodes)
    /// and a set of visited nodes.
    /// TODO do it non-recursive
    /// TODO handle non-direct revocations
    /// TODO split it with a certificate exporting and a weight calculation from certificate
    fn weight_inner(
        &self,
        pk: PublicKey,
        marked: &mut HashSet<PublicKeyHashable>,
    ) -> Option<Weight> {
        marked.insert(pk.clone().into());
        match self.nodes.get(&pk.clone().into()) {
            Some(node) => {
                let auths: Vec<Auth> = node.authorizations().collect();
                // TODO root node could be with authorizations
                if auths.is_empty() {
                    // if there is no auth for this node and this node is not a root node - it is revoked, so, no weight
                    match self.root_weights.get(&pk.into()) {
                        Some(root_weight) => Some(*root_weight),
                        None => None,
                    }
                } else {
                    let mut max_weight = None;

                    for auth_by in auths {
                        if !marked.contains(&auth_by.issued_by.clone().into()) {
                            // TODO do the search without recursion
                            let weight_op = self.weight_inner(auth_by.issued_by, marked);
                            if let Some(w) = weight_op {
                                if max_weight.is_none() || max_weight.unwrap() > w {
                                    max_weight = Some(w)
                                }
                            }
                        }
                    }
                    max_weight.map(|w| w + 1)
                }
            }
            None => None,
        }
    }

    /// Get the maximum weight of trust for one public key.
    /// Returns None if there is no such public key
    /// or some trust between this key and a root key is revoked.
    pub fn weight(&self, pk: PublicKey) -> Option<Weight> {
        let mut visited = HashSet::new();
        self.weight_inner(pk, &mut visited)
    }

    /// Mark public key as revoked.
    pub fn revoke(&mut self, revoke: Revoke) -> Result<(), String> {
        Revoke::verify(&revoke)?;

        let pk: PublicKeyHashable = revoke.pk.clone().into();

        match self.nodes.entry(pk) {
            Occupied(mut entry) => {
                entry.get_mut().update_revoke(revoke);
                Ok(())
            }
            Entry::Vacant(_) => Err("There is no trust with such PublicKey".to_string()),
        }
    }

    /// Check information about new certificates and about revoked certificates.
    /// Do it once per some time
    // TODO
    fn maintain() {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key_pair::KeyPair;
    use crate::misc::current_time;
    use failure::_core::time::Duration;

    pub fn one_minute() -> Duration {
        Duration::new(60, 0)
    }

    fn generate_root_cert() -> (KeyPair, KeyPair, Certificate) {
        let root_kp = KeyPair::generate();
        let second_kp = KeyPair::generate();

        let cur_time = current_time();

        (
            root_kp.clone(),
            second_kp.clone(),
            Certificate::issue_root(
                &root_kp,
                second_kp.public_key(),
                cur_time.checked_add(one_minute()).unwrap(),
                cur_time,
            ),
        )
    }

    fn generate_cert_with_len_and_expiration(
        len: usize,
        predefined: HashMap<usize, KeyPair>,
        expiration: Duration,
    ) -> (Vec<KeyPair>, Certificate) {
        assert!(len > 2);

        let root_kp = KeyPair::generate();
        let second_kp = KeyPair::generate();

        let mut cert =
            Certificate::issue_root(&root_kp, second_kp.public_key(), expiration, current_time());

        let mut key_pairs = vec![root_kp, second_kp];

        for idx in 2..len {
            let kp = predefined.get(&idx).unwrap_or(&KeyPair::generate()).clone();
            let previous_kp = &key_pairs[idx - 1];
            cert = Certificate::issue(
                &previous_kp,
                kp.public_key(),
                &cert,
                expiration,
                current_time().checked_sub(Duration::new(60, 0)).unwrap(),
                current_time(),
            )
            .unwrap();
            key_pairs.push(kp);
        }

        (key_pairs, cert)
    }

    #[allow(dead_code)]
    fn generate_cert_with_len(
        len: usize,
        predefined: HashMap<usize, KeyPair>,
    ) -> (Vec<KeyPair>, Certificate) {
        let cur_time = current_time();
        let far_future = cur_time.checked_add(one_minute()).unwrap();

        generate_cert_with_len_and_expiration(len, predefined, far_future)
    }

    #[test]
    fn test_add_cert_without_trusted_root() {
        let (_, _, cert) = generate_root_cert();

        let cur_time = current_time();

        let mut graph = TrustGraph::new(HashMap::new());
        let addition = graph.add(cert, cur_time);
        assert_eq!(addition.is_ok(), false);
    }

    #[test]
    fn test_add_cert() {
        let (root, _, cert) = generate_root_cert();

        let mut roots = HashMap::new();
        roots.insert(root.key_pair.public().into(), 0);

        let cur_time = current_time();

        let mut graph = TrustGraph::new(roots);
        let addition = graph.add(cert, cur_time);
        assert_eq!(addition.is_ok(), true);
    }

    #[test]
    fn test_add_certs_with_same_trusts_and_different_expirations() {
        let cur_time = current_time();
        let far_future = cur_time.checked_add(Duration::new(10, 0)).unwrap();
        let far_far_future = cur_time.checked_add(Duration::new(900, 0)).unwrap();
        let key_pair1 = KeyPair::generate();
        let key_pair2 = KeyPair::generate();

        let mut predefined1 = HashMap::new();
        predefined1.insert(5, key_pair1.clone());
        predefined1.insert(6, key_pair2.clone());

        let (key_pairs1, cert1) =
            generate_cert_with_len_and_expiration(10, predefined1, far_future);

        let mut predefined2 = HashMap::new();
        predefined2.insert(7, key_pair1.clone());
        predefined2.insert(8, key_pair2.clone());

        let (key_pairs2, cert2) =
            generate_cert_with_len_and_expiration(10, predefined2, far_far_future);

        let mut roots = HashMap::new();
        let root1_pk = key_pairs1[0].public_key();
        let root2_pk = key_pairs2[0].public_key();
        roots.insert(root1_pk.into(), 1);
        roots.insert(root2_pk.into(), 0);

        let mut graph = TrustGraph::new(roots);
        graph.add(cert1, cur_time).unwrap();

        let node2 = graph.get(key_pair2.public_key()).unwrap();
        let auth_by_kp1 = node2
            .authorizations()
            .find(|a| a.issued_by == key_pair1.public_key())
            .unwrap();

        assert_eq!(auth_by_kp1.trust.expires_at, far_future);

        graph.add(cert2, cur_time).unwrap();

        let node2 = graph.get(key_pair2.public_key()).unwrap();
        let auth_by_kp1 = node2
            .authorizations()
            .find(|a| a.issued_by == key_pair1.public_key())
            .unwrap();

        assert_eq!(auth_by_kp1.trust.expires_at, far_far_future);
    }

    #[test]
    fn test_one_cert_in_graph() {
        let (key_pairs, cert1) = generate_cert_with_len(10, HashMap::new());
        let last_trust = cert1.chain[9].clone();

        let mut roots = HashMap::new();
        let root_pk = key_pairs[0].public_key();
        roots.insert(root_pk.into(), 1);

        let mut graph = TrustGraph::new(roots);
        graph.add(cert1, current_time()).unwrap();

        let w1 = graph.weight(key_pairs[0].public_key()).unwrap();
        assert_eq!(w1, 1);

        let w2 = graph.weight(key_pairs[1].public_key()).unwrap();
        assert_eq!(w2, 2);

        let w3 = graph.weight(key_pairs[9].public_key()).unwrap();
        assert_eq!(w3, 10);

        let node = graph.get(key_pairs[9].public_key()).unwrap();
        let auths: Vec<Auth> = node.authorizations().collect();

        assert_eq!(auths.len(), 1);
        assert_eq!(auths[0].trust, last_trust);
    }

    #[test]
    fn test_cycles_in_graph() {
        let key_pair1 = KeyPair::generate();
        let key_pair2 = KeyPair::generate();
        let key_pair3 = KeyPair::generate();

        let mut predefined1 = HashMap::new();
        predefined1.insert(3, key_pair1.clone());
        predefined1.insert(5, key_pair2.clone());
        predefined1.insert(7, key_pair3.clone());

        let (key_pairs1, cert1) = generate_cert_with_len(10, predefined1);

        let mut predefined2 = HashMap::new();
        predefined2.insert(7, key_pair1.clone());
        predefined2.insert(6, key_pair2.clone());
        predefined2.insert(5, key_pair3.clone());

        let (key_pairs2, cert2) = generate_cert_with_len(10, predefined2);

        let mut roots = HashMap::new();
        let root1_pk = key_pairs1[0].public_key();
        let root2_pk = key_pairs2[0].public_key();
        roots.insert(root1_pk.into(), 1);
        roots.insert(root2_pk.into(), 0);
        let mut graph = TrustGraph::new(roots);

        let last_pk1 = cert1.chain[9].issued_for.clone();
        let last_pk2 = cert2.chain[9].issued_for.clone();

        graph.add(cert1, current_time()).unwrap();
        graph.add(cert2, current_time()).unwrap();

        let revoke1 = Revoke::create(&key_pairs1[3], key_pairs1[4].public_key(), current_time());
        graph.revoke(revoke1).unwrap();
        let revoke2 = Revoke::create(&key_pairs2[5], key_pairs2[6].public_key(), current_time());
        graph.revoke(revoke2).unwrap();

        let w1 = graph.weight(key_pair1.public_key()).unwrap();
        // all upper trusts are revoked for this public key
        let w2 = graph.weight(key_pair2.public_key());
        let w3 = graph.weight(key_pair3.public_key()).unwrap();
        let w_last1 = graph.weight(last_pk1).unwrap();
        let w_last2 = graph.weight(last_pk2).unwrap();

        assert_eq!(w1, 4);
        assert_eq!(w2.is_none(), true);
        assert_eq!(w3, 5);
        assert_eq!(w_last1, 7);
        assert_eq!(w_last2, 6);
    }
}
