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
use crate::ed25519::PublicKey;
use crate::public_key_hashable::PublicKeyHashable;
use crate::revoke::Revoke;
use crate::trust::Trust;
use crate::trust_node::{Auth, TrustNode};
use std::borrow::Borrow;
use std::collections::hash_map::Entry;
use std::collections::hash_map::Entry::Occupied;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::time::Duration;

/// for simplicity, we store `n` where Weight = 1/n^2
type Weight = u32;

/// Graph to efficiently calculate weights of certificates and get chains of certificates.
/// TODO serialization/deserialization
/// TODO export a certificate from graph
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct TrustGraph {
    // TODO abstract this into a trait with key access methods
    // TODO: add docs on fields
    nodes: HashMap<PublicKeyHashable, TrustNode>,
    root_weights: HashMap<PublicKeyHashable, Weight>,
}

#[allow(dead_code)]
impl TrustGraph {
    pub fn new(root_weights: Vec<(PublicKey, Weight)>) -> Self {
        Self {
            nodes: HashMap::new(),
            root_weights: root_weights
                .into_iter()
                .map(|(k, w)| (k.into(), w))
                .collect(),
        }
    }

    /// Insert new root weights
    pub fn add_root_weights<P: Into<PublicKeyHashable>>(&mut self, weights: Vec<(P, Weight)>) {
        self.root_weights
            .extend(weights.into_iter().map(|(k, w)| (k.into(), w)))
    }

    /// Get trust by public key
    pub fn get(&self, pk: PublicKey) -> Option<&TrustNode> {
        self.nodes.get(&pk.into())
    }

    // TODO: remove cur_time from api, leave it for tests only
    /// Certificate is a chain of trusts, add this chain to graph
    pub fn add<C>(&mut self, cert: C, cur_time: Duration) -> Result<(), String>
    where
        C: Borrow<Certificate>,
    {
        let roots: Vec<PublicKey> = self.root_weights.keys().cloned().map(Into::into).collect();
        // Check that certificate is valid and converges to one of the known roots
        Certificate::verify(cert.borrow(), roots.as_slice(), cur_time)?;

        let mut chain = cert.borrow().chain.iter();
        let root_trust = chain.next().ok_or("empty chain")?;
        let root_pk: PublicKeyHashable = root_trust.issued_for.clone().into();

        // Insert new TrustNode for this root_pk if there wasn't one
        if self.nodes.get_mut(&root_pk).is_none() {
            let mut trust_node = TrustNode::new(root_trust.issued_for.clone(), cur_time);
            let root_auth = Auth {
                trust: root_trust.clone(),
                issued_by: root_trust.issued_for.clone(),
            };
            trust_node.update_auth(root_auth);
            self.nodes.insert(root_pk, trust_node);
        }

        // Insert remaining trusts to the graph
        let mut previous_trust = root_trust;
        for trust in chain {
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

    /// Get the maximum weight of trust for one public key.
    pub fn weight<P>(&self, pk: P) -> Option<Weight>
    where
        P: Borrow<PublicKey>,
    {
        if let Some(weight) = self.root_weights.get(pk.borrow().as_ref()) {
            return Some(*weight);
        }

        let roots: Vec<PublicKey> = self
            .root_weights
            .keys()
            .map(|pk| pk.clone().into())
            .collect();

        // get all possible certificates from the given public key to all roots in the graph
        let certs = self.get_all_certs(pk, roots.as_slice());
        self.certificates_weight(certs)
    }

    /// Calculate weight from given certificates
    /// Returns None if there is no such public key
    /// or some trust between this key and a root key is revoked.
    /// TODO handle non-direct revocations
    pub fn certificates_weight<C, I>(&self, certs: I) -> Option<Weight>
    where
        C: Borrow<Certificate>,
        I: IntoIterator<Item = C>,
    {
        let mut certs = certs.into_iter().peekable();
        // if there are no certificates for the given public key, there is no info about this public key
        // or some elements of possible certificate chains was revoked
        certs.peek()?;

        let mut weight = std::u32::MAX;

        for cert in certs {
            let cert = cert.borrow();

            let root_weight = *self
                .root_weights
                .get(cert.chain.first()?.issued_for.as_ref())
                // This panic shouldn't happen // TODO: why?
                .expect("first trust in chain must be in root_weights");

            // certificate weight = root weight + 1 * every other element in the chain
            // (except root, so the formula is `root weight + chain length - 1`)
            weight = std::cmp::min(weight, root_weight + cert.chain.len() as u32 - 1)
        }

        Some(weight)
    }

    /// BF search for all converging paths (chains) in the graph
    /// TODO could be optimized with closure, that will calculate the weight on the fly
    /// TODO or store auths to build certificates
    fn bf_search_paths(
        &self,
        node: &TrustNode,
        roots: HashSet<&PublicKeyHashable>,
    ) -> Vec<Vec<Auth>> {
        // queue to collect all chains in the trust graph (each chain is a path in the trust graph)
        let mut chains_queue: VecDeque<Vec<Auth>> = VecDeque::new();

        let node_auths: Vec<Auth> = node.authorizations().cloned().collect();
        // put all auth in the queue as the first possible paths through the graph
        for auth in node_auths {
            chains_queue.push_back(vec![auth]);
        }

        // List of all chains that converge (terminate) to known roots
        let mut terminated_chains: Vec<Vec<Auth>> = Vec::new();

        while !chains_queue.is_empty() {
            let cur_chain = chains_queue
                .pop_front()
                .expect("`chains_queue` always has at least one element");

            let last = cur_chain
                .last()
                .expect("`cur_chain` always has at least one element");

            let auths: Vec<Auth> = self
                .nodes
                .get(&last.issued_by.clone().into())
                .expect(
                    "there cannot be paths without any nodes after adding verified certificates",
                )
                .authorizations()
                .cloned()
                .collect();

            for auth in auths {
                // if there is auth, that we not visited in the current chain, copy chain and append this auth
                if !cur_chain
                    .iter()
                    .any(|a| a.trust.issued_for == auth.issued_by)
                {
                    let mut new_chain = cur_chain.clone();
                    new_chain.push(auth);
                    chains_queue.push_back(new_chain);
                }
            }

            // to be considered a valid chain, the chain must:
            // - end with a self-signed trust
            // - that trust must converge to one of the root weights
            // - there should be more than 1 trust in the chain
            let self_signed = last.issued_by == last.trust.issued_for;
            let converges_to_root = roots.contains(last.issued_by.as_ref());

            if self_signed && converges_to_root && cur_chain.len() > 1 {
                terminated_chains.push(cur_chain);
            }
        }

        terminated_chains
    }

    // TODO: remove `roots` argument from api, leave it for tests and internal usage only
    /// Get all possible certificates where `issued_for` will be the last element of the chain
    /// and one of the destinations is the root of this chain.
    pub fn get_all_certs<P>(&self, issued_for: P, roots: &[PublicKey]) -> Vec<Certificate>
    where
        P: Borrow<PublicKey>,
    {
        // get all auths (edges) for issued public key
        let issued_for_node = self.nodes.get(issued_for.borrow().as_ref());

        let roots = roots.iter().map(|pk| pk.as_ref());
        let roots = self.root_weights.keys().chain(roots).collect();

        match issued_for_node {
            Some(node) => self
                .bf_search_paths(node, roots)
                .iter()
                .map(|auths| {
                    // TODO: can avoid cloning here by returning &Certificate
                    let trusts: Vec<Trust> =
                        auths.iter().map(|auth| auth.trust.clone()).rev().collect();
                    Certificate::new_unverified(trusts)
                })
                .filter(|c| {
                    // Certificate with one trust means nothing, gotta be a bug. Checking for it here.
                    debug_assert!(
                        c.chain.len() > 1,
                        "certificate with chain of len 1 arose: {:#?}",
                        c
                    );
                    c.chain.len() > 1
                })
                .collect(),
            None => Vec::new(),
        }
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

    fn generate_cert_with(
        len: usize,
        // Map of index to keypair. These key pairs will be used in trust chains at the given indexes
        keys: HashMap<usize, KeyPair>,
        expires_at: Duration,
        issued_at: Duration,
    ) -> (Vec<KeyPair>, Certificate) {
        assert!(len > 2);

        let root_kp = KeyPair::generate();
        let second_kp = KeyPair::generate();

        let mut cert =
            Certificate::issue_root(&root_kp, second_kp.public_key(), expires_at, issued_at);

        let mut key_pairs = vec![root_kp, second_kp];

        for idx in 2..len {
            let kp = keys.get(&idx).unwrap_or(&KeyPair::generate()).clone();
            let previous_kp = &key_pairs[idx - 1];
            cert = Certificate::issue(
                &previous_kp,
                kp.public_key(),
                &cert,
                expires_at,
                // TODO: why `issued_at = issued_at - 60 seconds`?
                issued_at.checked_sub(Duration::from_secs(60)).unwrap(),
                current_time(),
            )
            .unwrap();
            key_pairs.push(kp);
        }

        (key_pairs, cert)
    }

    fn generate_cert_with_len(
        len: usize,
        keys: HashMap<usize, KeyPair>,
    ) -> (Vec<KeyPair>, Certificate) {
        let cur_time = current_time();
        let far_future = cur_time.checked_add(one_minute()).unwrap();

        generate_cert_with(len, keys, far_future, cur_time)
    }

    #[test]
    fn test_add_cert_without_trusted_root() {
        let (_, _, cert) = generate_root_cert();

        let cur_time = current_time();

        let mut graph = TrustGraph::default();
        let addition = graph.add(cert, cur_time);
        assert_eq!(addition.is_ok(), false);
    }

    #[test]
    fn test_add_cert() {
        let (root, _, cert) = generate_root_cert();

        let mut graph = TrustGraph::default();
        graph.root_weights.insert(root.key_pair.public().into(), 0);

        let addition = graph.add(cert, current_time());
        assert_eq!(addition.is_ok(), true);
    }

    #[test]
    fn test_add_certs_with_same_trusts_and_different_expirations() {
        let cur_time = current_time();
        let far_future = cur_time + Duration::from_secs(10);
        let far_far_future = cur_time + Duration::from_secs(900);
        let key_pair1 = KeyPair::generate();
        let key_pair2 = KeyPair::generate();

        // Use key_pair1 and key_pair2 for 5th and 6th trust in the cert chain
        let mut chain_keys = HashMap::new();
        chain_keys.insert(5, key_pair1.clone());
        chain_keys.insert(6, key_pair2.clone());

        let (key_pairs1, cert1) = generate_cert_with(10, chain_keys, far_future * 2, far_future);

        // Use key_pair1 and key_pair2 for 7th and 8th trust in the cert chain
        let mut chain_keys = HashMap::new();
        chain_keys.insert(7, key_pair1.clone());
        chain_keys.insert(8, key_pair2.clone());

        let (key_pairs2, cert2) =
            generate_cert_with(10, chain_keys, far_far_future * 2, far_far_future);

        let mut graph = TrustGraph::default();
        let root1_pk = key_pairs1[0].public_key();
        let root2_pk = key_pairs2[0].public_key();
        graph.root_weights.insert(root1_pk.into(), 1);
        graph.root_weights.insert(root2_pk.into(), 0);
        graph.add(cert1, cur_time).unwrap();

        let node2 = graph.get(key_pair2.public_key()).unwrap();
        let auth_by_kp1 = node2
            .authorizations()
            .find(|a| a.issued_by == key_pair1.public_key())
            .unwrap();

        assert_eq!(auth_by_kp1.trust.expires_at, far_future * 2);

        graph.add(cert2, cur_time).unwrap();

        let node2 = graph.get(key_pair2.public_key()).unwrap();
        let auth_by_kp1 = node2
            .authorizations()
            .find(|a| a.issued_by == key_pair1.public_key())
            .unwrap();

        assert_eq!(auth_by_kp1.trust.expires_at, far_far_future * 2);
    }

    #[test]
    fn test_one_cert_in_graph() {
        let (key_pairs, cert1) = generate_cert_with_len(10, HashMap::new());
        let last_trust = cert1.chain[9].clone();

        let mut graph = TrustGraph::default();

        let root_pk = key_pairs[0].public_key();
        graph.root_weights.insert(root_pk.into(), 1);

        graph.add(cert1, current_time()).unwrap();

        let w1 = graph.weight(key_pairs[0].public_key()).unwrap();
        assert_eq!(w1, 1);

        let w2 = graph.weight(key_pairs[1].public_key()).unwrap();
        assert_eq!(w2, 2);

        let w3 = graph.weight(key_pairs[9].public_key()).unwrap();
        assert_eq!(w3, 10);

        let node = graph.get(key_pairs[9].public_key()).unwrap();
        let auths: Vec<&Auth> = node.authorizations().collect();

        assert_eq!(auths.len(), 1);
        assert_eq!(auths[0].trust, last_trust);
    }

    #[test]
    fn test_cycles_in_graph() {
        let key_pair1 = KeyPair::generate();
        let key_pair2 = KeyPair::generate();
        let key_pair3 = KeyPair::generate();

        let mut chain_keys = HashMap::new();
        chain_keys.insert(3, key_pair1.clone());
        chain_keys.insert(5, key_pair2.clone());
        chain_keys.insert(7, key_pair3.clone());

        let (key_pairs1, cert1) = generate_cert_with_len(10, chain_keys);

        let mut chain_keys = HashMap::new();
        chain_keys.insert(7, key_pair1.clone());
        chain_keys.insert(6, key_pair2.clone());
        chain_keys.insert(5, key_pair3.clone());

        let (key_pairs2, cert2) = generate_cert_with_len(10, chain_keys);

        let mut graph = TrustGraph::default();
        let root1_pk = key_pairs1[0].public_key();
        let root2_pk = key_pairs2[0].public_key();
        graph.root_weights.insert(root1_pk.into(), 1);
        graph.root_weights.insert(root2_pk.into(), 0);

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

    #[test]
    fn test_get_one_cert() {
        let (key_pairs, cert) = generate_cert_with_len(5, HashMap::new());

        let mut graph = TrustGraph::default();
        let root1_pk = key_pairs[0].public_key();
        graph.root_weights.insert(root1_pk.clone().into(), 1);

        graph.add(cert.clone(), current_time()).unwrap();

        let certs = graph.get_all_certs(key_pairs.last().unwrap().public_key(), &[root1_pk]);

        assert_eq!(certs.len(), 1);
        assert_eq!(certs[0], cert);
    }

    #[test]
    fn test_chain_from_root_to_another_root() {
        let (_, cert) = generate_cert_with_len(6, HashMap::new());

        let mut graph = TrustGraph::default();
        // add first and last trusts as roots
        graph
            .root_weights
            .insert(cert.chain[0].clone().issued_for.into(), 1);
        graph
            .root_weights
            .insert(cert.chain[3].clone().issued_for.into(), 1);
        graph
            .root_weights
            .insert(cert.chain[5].clone().issued_for.into(), 1);

        graph.add(cert.clone(), current_time()).unwrap();

        let t = cert.chain[5].clone();
        let certs = graph.get_all_certs(t.issued_for, &[]);

        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn test_find_certs() {
        let key_pair1 = KeyPair::generate();
        let key_pair2 = KeyPair::generate();
        let key_pair3 = KeyPair::generate();

        let mut chain_keys = HashMap::new();
        chain_keys.insert(2, key_pair1.clone());
        chain_keys.insert(3, key_pair2.clone());
        chain_keys.insert(4, key_pair3.clone());

        let (key_pairs1, cert1) = generate_cert_with_len(5, chain_keys);

        let mut chain_keys = HashMap::new();
        chain_keys.insert(4, key_pair1.clone());
        chain_keys.insert(3, key_pair2.clone());
        chain_keys.insert(2, key_pair3.clone());

        let (key_pairs2, cert2) = generate_cert_with_len(5, chain_keys);

        let mut chain_keys = HashMap::new();
        chain_keys.insert(3, key_pair1.clone());
        chain_keys.insert(4, key_pair2.clone());
        chain_keys.insert(2, key_pair3.clone());

        let (key_pairs3, cert3) = generate_cert_with_len(5, chain_keys);

        let mut graph = TrustGraph::default();
        let root1_pk = key_pairs1[0].public_key();
        let root2_pk = key_pairs2[0].public_key();
        let root3_pk = key_pairs3[0].public_key();
        graph.root_weights.insert(root1_pk.clone().into(), 1);
        graph.root_weights.insert(root2_pk.clone().into(), 0);
        graph.root_weights.insert(root3_pk.clone().into(), 0);

        graph.add(cert1, current_time()).unwrap();
        graph.add(cert2, current_time()).unwrap();
        graph.add(cert3, current_time()).unwrap();

        let roots_values = [root1_pk, root2_pk, root3_pk];

        let certs1 = graph.get_all_certs(key_pair1.public_key(), &roots_values);
        let lenghts1: Vec<usize> = certs1.iter().map(|c| c.chain.len()).collect();
        let check_lenghts1: Vec<usize> = vec![3, 4, 4, 5, 5];
        assert_eq!(lenghts1, check_lenghts1);

        let certs2 = graph.get_all_certs(key_pair2.public_key(), &roots_values);
        let lenghts2: Vec<usize> = certs2.iter().map(|c| c.chain.len()).collect();
        let check_lenghts2: Vec<usize> = vec![4, 4, 4, 5, 5];
        assert_eq!(lenghts2, check_lenghts2);

        let certs3 = graph.get_all_certs(key_pair3.public_key(), &roots_values);
        let lenghts3: Vec<usize> = certs3.iter().map(|c| c.chain.len()).collect();
        let check_lenghts3: Vec<usize> = vec![3, 3, 5];
        assert_eq!(lenghts3, check_lenghts3);
    }
}
