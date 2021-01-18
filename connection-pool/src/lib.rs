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

mod behaviour;
mod connection_pool;

pub use behaviour::ConnectionPoolBehaviour;
pub use connection_pool::ConnectionPool;
