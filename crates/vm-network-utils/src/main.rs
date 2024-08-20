use vm_network_utils::{clear_network, setup_network, NetworkSettings};

fn main() {
    let settings = NetworkSettings {
        public_ip: "0.0.0.0".to_string(),
        vm_ip: "127.0.0.1".to_string(),
        bridge_name: "br0".to_string(),
        port_range: (1000, 65535),
        host_ssh_port: 2222,
        vm_ssh_port: 22,
    };
    let test_name = "test-name";
    println!("Clear: {:?}", clear_network(&settings, test_name));
    //let result = setup_network(settings, test_name);
    //println!("Setup: {result:?}");
}
