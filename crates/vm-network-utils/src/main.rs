use std::net::Ipv4Addr;
use std::str::FromStr;
use vm_network_utils::{clear_network, setup_network, NetworkSettings};

fn main() {
    let settings = NetworkSettings {
        public_ip: Ipv4Addr::from_str("0.0.0.0").unwrap(),
        vm_ip: Ipv4Addr::from_str("127.0.0.1").unwrap(),
        bridge_name: "br0".to_string(),
        port_range: (1000, 65535),
        host_ssh_port: 2222,
        vm_ssh_port: 22,
    };
    let test_name = "test-name";
    println!("Clear: {:?}", clear_network(&settings, test_name));
    // let result = setup_network(settings, test_name);
    //println!("Setup: {result:?}");
}
