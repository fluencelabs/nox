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

use super::address_protocol::Protocol;
use libp2p::kad::record;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::convert::{TryFrom, TryInto};
use std::iter::FromIterator;
use std::ops::Div;
use std::str::FromStr;
use url::Url;

static SCHEME: Lazy<Url> = Lazy::new(|| Url::parse("fluence:/").unwrap());

#[derive(Debug, Clone)]
pub enum AddressError {
    Empty,
    UnknownProtocol,
    InvalidPeerId,
    InvalidUrl,
    InvalidProtocol,
    InvalidUtf8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Address(Url);

// TODO: validate address on creation and building
impl Address {
    // Creates empty address, containing only SCHEMA
    pub fn empty() -> Self {
        Address(SCHEME.clone())
    }

    // Creates address from a protocol and tail of protocols
    pub fn cons<P, I>(proto: P, protos: I) -> Self
    where
        P: Borrow<Protocol>,
        I: IntoIterator,
        I::Item: Borrow<Protocol>,
    {
        Address::from(proto.borrow()).append_protos(protos)
    }

    // Appends path of `other` to `self`
    pub fn extend<A>(mut self, other: A) -> Self
    where
        A: Borrow<Address>,
    {
        let other = &other.borrow().0;

        let path = other.path_segments().unwrap();
        self = self.extend_path(path);

        if let Some(fragment) = other.fragment() {
            self = self.set_fragment(fragment)
        }

        self
    }

    // Appends all given protocols to `self`
    pub fn append_protos<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator,
        I::Item: Borrow<Protocol>,
    {
        for p in iter {
            self = self.append(p.borrow())
        }
        self
    }

    // Prepends given protocol to the beginning of the current path
    pub fn prepend<P>(mut self, protocol: P) -> Self
    where
        P: Borrow<Protocol>,
    {
        match protocol.borrow() {
            // override fragment (even though it's strange to prepend hashtag, i'd avoid throwing error here)
            Protocol::Hashtag(h) => self.set_fragment(h.as_str()),
            protocol => {
                // concatenate new with existing
                let path = protocol.to_string() + self.0.path();
                self.0.set_path(path.as_str());
                self
            }
        }
    }

    // Appends given protocol to the end of the current path
    pub fn append<P>(self, protocol: P) -> Self
    where
        P: Borrow<Protocol>,
    {
        match protocol.borrow() {
            Protocol::Hashtag(h) => self.set_fragment(h.as_str()),
            other => {
                let (p, v) = other.components();
                self.extend_path([p, v.as_str()].iter())
            }
        }
    }

    // Returns protocols in the address
    pub fn protocols(&self) -> Vec<Protocol> {
        self.iter().collect()
    }

    // Builds iterator over protocols in the address
    pub fn iter(&self) -> ProtocolsIter<'_> {
        ProtocolsIter(self.0.path_segments().unwrap(), self.0.fragment())
    }

    // Returns true if address empty (i.e., contains only schema)
    pub fn is_empty(&self) -> bool {
        let path = self.0.path();
        (path.is_empty() || path == "/") && self.0.fragment().is_none()
    }

    // Returns true if address contains given protocol
    pub fn contains(&self, proto: &Protocol) -> bool {
        self.iter().any(|p| p.eq(proto))
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    /// Appends all given path segments to `self`
    fn extend_path<I>(mut self, segments: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        // It's ok to call .expect() since Address always contains SCHEME
        self.0
            .path_segments_mut()
            .expect("url contains scheme")
            .extend(segments);
        self
    }

    /// Sets fragment (hashtag) of the underlying address
    fn set_fragment<'a>(mut self, fragment: &'a str) -> Self {
        self.0.set_fragment(Some(fragment));
        self
    }
}

impl From<Protocol> for Address {
    fn from(p: Protocol) -> Self {
        Address::from(&p)
    }
}

impl From<&Protocol> for Address {
    fn from(p: &Protocol) -> Self {
        Address::empty().append(p)
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> core::result::Result<Self, Self::Err> {
        let url = SCHEME.join(s).map_err(|_| AddressError::InvalidUrl)?;
        Ok(Address(url))
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = AddressError;

    fn try_from(v: &[u8]) -> core::result::Result<Self, Self::Error> {
        let utf8 = String::from_utf8_lossy(v);
        utf8.parse()
    }
}

impl TryFrom<&record::Key> for Address {
    type Error = AddressError;

    fn try_from(key: &record::Key) -> core::result::Result<Self, Self::Error> {
        key.as_ref().try_into()
    }
}

impl Into<record::Key> for &Address {
    fn into(self) -> record::Key {
        Into::<Vec<u8>>::into(self).into()
    }
}

impl Into<Vec<u8>> for &Address {
    fn into(self) -> Vec<u8> {
        self.to_string().into_bytes()
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromIterator<Protocol> for Address {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Protocol>,
    {
        let vec = iter.into_iter().collect::<Vec<_>>();
        vec.iter().collect()
    }
}

impl<'a> FromIterator<&'a Protocol> for Address {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = &'a Protocol>,
    {
        let path = iter.into_iter().map(|p| p.to_string()).collect::<String>();
        let url = SCHEME
            .join(path.as_str())
            .expect("can't fail joining protocols");
        Address(url)
    }
}

pub struct ProtocolsIter<'a>(std::str::Split<'a, char>, Option<&'a str>);
impl<'a> ProtocolsIter<'a> {
    fn take_fragment(&mut self) -> Option<Protocol> {
        let hashtag = std::iter::once("#");
        let fragment = self.1.take().into_iter();
        Protocol::from_iter(hashtag.chain(fragment)).ok()
    }

    fn next_protocol(&mut self) -> Option<Protocol> {
        Protocol::from_iter(&mut self.0).ok()
    }
}

impl<'a> Iterator for ProtocolsIter<'a> {
    type Item = Protocol;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_protocol().or(self.take_fragment())
    }
}

impl Default for Address {
    fn default() -> Self {
        Address::empty()
    }
}

impl Div<Address> for Protocol {
    type Output = Address;

    fn div(self, rhs: Address) -> Self::Output {
        rhs.prepend(self)
    }
}

impl Div<Protocol> for Address {
    type Output = Address;

    fn div(self, rhs: Protocol) -> Self::Output {
        self.append(rhs)
    }
}

impl Div<&Protocol> for Address {
    type Output = Address;

    fn div(self, rhs: &Protocol) -> Self::Output {
        self.append(rhs)
    }
}

impl<I> Div<I> for Address
where
    I: IntoIterator,
    I::Item: Borrow<Protocol>,
{
    type Output = Address;

    fn div(self, rhs: I) -> Self::Output {
        self.append_protos(rhs)
    }
}

impl Div<Address> for Address {
    type Output = Address;

    fn div(self, rhs: Address) -> Self::Output {
        self.extend(rhs)
    }
}

impl Div<Protocol> for Protocol {
    type Output = Address;

    fn div(self, rhs: Protocol) -> Self::Output {
        Address::from(self).append(rhs)
    }
}

// Builds relay address which looks like this: "/peer/QmRelay/client/QmClient"
#[macro_export]
macro_rules! relay {
    ($relay:expr,$client:expr$(,$kp:expr)?) => {{
        let relay = $crate::Address::from($crate::Protocol::Peer($relay));
        let relay = relay.append($crate::Protocol::Client($client));
        // Optional line. If kp isn't passed, no signature will be generated
        $(let relay = {
            let signature = $kp.sign(relay.path().as_bytes());
            relay.append($crate::Protocol::Signature(signature))
        };)?
        relay
    }};
}

// Builds service address which looks like this: "/providers/Key"
#[macro_export]
macro_rules! provider {
    ($service_id:expr) => {{
        let id = $service_id;
        // TODO: Will usually clone here, is it ok?
        $crate::Address::from($crate::Protocol::Providers(id.into()))
    }};
}

/// Used in tests, but can't be marked as `#[cfg(tests)]` because it'll not be possible to export
impl Address {
    pub fn random_relay_unsigned() -> Self {
        use fluence_libp2p::RandomPeerId;

        relay!(RandomPeerId::random(), RandomPeerId::random())
    }

    pub fn random_relay() -> Self {
        use fluence_libp2p::RandomPeerId;
        use libp2p::identity::{ed25519::Keypair, PublicKey::Ed25519};

        let node = RandomPeerId::random();
        let client_kp = Keypair::generate();
        let client = Ed25519(client_kp.public()).into_peer_id();

        relay!(node, client, client_kp)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn route_and_resolve() {
        #[allow(clippy::ptr_arg)]
        fn imitate_resolve(_service_id: &String) -> Address {
            let relay = Protocol::Peer(
                "Qmay8oMmnDmfLpmZtNwisEcmReVVqzvm2vcTc9rPzxeS3x"
                    .parse()
                    .unwrap(),
            );
            let client = Protocol::Client(
                "QmWsPEib1mbGSxdqtDGnrqXiZFfVEbvicKS6cf5JfTtaZU"
                    .parse()
                    .unwrap(),
            );

            vec![relay, client].iter().collect()
        }

        let service = Address("fluence:/service/IPFS.get_QmFile".parse().unwrap());

        let mut iter = service.iter().peekable();

        let expected: Address = "fluence:/peer/Qmay8oMmnDmfLpmZtNwisEcmReVVqzvm2vcTc9rPzxeS3x/client/QmWsPEib1mbGSxdqtDGnrqXiZFfVEbvicKS6cf5JfTtaZU/service/IPFS.get_QmFile".parse().unwrap();

        if let Some(Protocol::Providers(id)) = iter.peek() {
            let resolved = imitate_resolve(id).extend(&service);
            assert_eq!(expected, resolved);

            let protos = service.iter();
            let resolved = imitate_resolve(id).append_protos(protos);
            assert_eq!(expected, resolved);
        } else {
            unreachable!()
        };
    }

    #[test]
    fn try_hashtag() {
        let _u: Url = "fluence:/peer/QmPeer/#service.call".parse().unwrap();

        let u = SCHEME.join("#w.tf").unwrap();
        println!("{:?} {:?} {:?}", u, u.fragment(), u.path());
        let u = _u.join("fluence:/client/Cl/#test.call").unwrap();
        println!("{:?} {:?} {:?}", u, u.fragment(), u.path());
    }
}
