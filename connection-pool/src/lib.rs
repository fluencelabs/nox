/*
    Contact {
        pub_key: bytes
        addr: Multiaddr?
    }

    ConnPool {
        // aqua-accessible (for `is_connected` builtin + xor)
        is_connected: PubKey -> Bool

        // Contact algebra
        get_contact: PubKey -> Contact?
        is_connected: Contact -> Bool
        connect: Contact -> async Bool // Bool or Either
        disconnect: Contact -> async Bool // Bool or Either

        // number of active (existing) connections
        count_connections: {clients: Int, peers: Int}
        //conn lifecycle
        lifecycle_events: (  Connected(Contact) | Disconnected(Contact)  )*
        // receive msg (particle)
        source: (Contact, msg)*
        // send msg (particle)
        sink: (Contact, msg) -> async Bool // Bool or Either
    }

    // Kademlia algebra
    - neighborhood
    - resolve?
*/

#![allow(unused_imports, dead_code)]

mod api;
mod behaviour;
mod connection_pool;

pub use ::connection_pool::ConnectionPoolT;
pub use ::connection_pool::Contact;
pub use ::connection_pool::LifecycleEvent;
pub use api::{ConnectionPoolApi, ConnectionPoolInlet};
pub use behaviour::ConnectionPoolBehaviour;
