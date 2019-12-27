use libp2p::Multiaddr;
use libp2p_core::Multiaddr;

pub struct Config {
    /// Local port to listen on
    pub listen_port: u16,

    /// Local ip address to listen on
    pub listen_ip: std::net::IpAddr,

    /// TODO: Key that will be used during peer id creation
    pub secret_key: Option<String>,

    /// TODO: Bootstrap nodes to join to the Fluence network
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// Topics to subscribe at the start
    pub topics: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            listen_port: 7777,
            listen_ip: "127.0.0.1".parse().unwrap(),
            secret_key: None,
            bootstrap_nodes: vec![],
            topics: vec!["churn".to_string()], // churn topic is needed for a
        }
    }
}
