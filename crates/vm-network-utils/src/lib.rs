use iptables::IPTables;
use std::net::Ipv4Addr;
use thiserror::Error;

pub struct NetworkSettings {
    pub public_ip: Ipv4Addr,
    pub vm_ip: Ipv4Addr,
    pub bridge_name: String,
    pub port_range: (u16, u16),
    pub host_ssh_port: u16,
    pub vm_ssh_port: u16,
}

// The iptables crate only return this kind of error, which can't be sent between threads
// So, the easiest solution is to store the error message as strings
#[derive(Debug, Error)]
pub enum NetworkSetupError {
    #[error("error setting up DNAT rules: {message}")]
    DNat { message: String },
    #[error("error setting up SNAT rules: {message}")]
    SNat { message: String },
    #[error("error setting up FWD rules: {message}")]
    Fwd { message: String },
    #[error("error initializing iptables: {message}")]
    Init { message: String },
    #[error("error cleaning the rules for the chain {chain_name}: {message}")]
    Clean { chain_name: String, message: String },
}

type IpTablesError = Box<dyn std::error::Error>;

pub fn setup_network(
    network_settings: &NetworkSettings,
    name: &str,
) -> Result<(), NetworkSetupError> {
    let ipt = iptables::new(false).map_err(|err| NetworkSetupError::Init {
        message: err.to_string(),
    })?;
    setup_snat(network_settings, &ipt, name).map_err(|err| NetworkSetupError::SNat {
        message: err.to_string(),
    })?;
    setup_dnat(network_settings, &ipt, name).map_err(|err| NetworkSetupError::DNat {
        message: err.to_string(),
    })?;
    setup_fwd(network_settings, &ipt, name).map_err(|err| NetworkSetupError::Fwd {
        message: err.to_string(),
    })?;
    Ok(())
}

pub fn clear_network(
    network_settings: &NetworkSettings,
    name: &str,
) -> Result<(), NetworkSetupError> {
    clear_network_inner(network_settings, name).map_err(|err| NetworkSetupError::Clean {
        chain_name: name.to_string(),
        message: err.to_string(),
    })
}

fn clear_network_inner(
    network_settings: &NetworkSettings,
    name: &str,
) -> Result<(), IpTablesError> {
    let ipt = iptables::new(false)?;
    {
        let (chain_name, rules) = snat_rules(network_settings, name);
        clear_chain(&ipt, &chain_name, &rules)?;
    }

    {
        let (chain_name, rules) = dnat_rules(network_settings, name);
        clear_chain(&ipt, &chain_name, &rules)?;
    }

    {
        let (chain_name, rules) = fwd_rules(network_settings, name);
        clear_chain(&ipt, &chain_name, &rules)?;
    }
    Ok(())
}

fn clear_chain(ipt: &IPTables, chain_name: &str, rules: &Vec<String>) -> Result<(), IpTablesError> {
    let exists = ipt.chain_exists("nat", chain_name)?;
    if !exists {
        tracing::warn!("Can't clean the chain {chain_name}: doesn't exist");
        return Ok(());
    }

    for rule in rules {
        if ipt.exists("nat", chain_name, rule)? {
            ipt.delete("nat", chain_name, rule)?;
        } else {
            tracing::warn!(
                "Can't clean a rule for the chain {chain_name}: doesn't exist; rule: {rule}"
            )
        }
    }

    ipt.delete_chain("nat", chain_name)?;
    Ok(())
}

fn setup_dnat(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), IpTablesError> {
    let (chain_name, rules) = dnat_rules(network_settings, name);
    apply_rules(ipt, &chain_name, &rules)?;
    Ok(())
}

fn setup_snat(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), IpTablesError> {
    let (chain_name, rules) = snat_rules(network_settings, name);
    apply_rules(ipt, &chain_name, &rules)?;
    Ok(())
}

fn setup_fwd(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), IpTablesError> {
    let (chain_name, rules) = fwd_rules(network_settings, name);
    apply_rules(ipt, &chain_name, &rules)?;
    Ok(())
}

fn apply_rules(ipt: &IPTables, name: &str, rules: &Vec<String>) -> Result<(), IpTablesError> {
    ipt.new_chain("nat", name)?;
    for rule in rules {
        ipt.append("nat", name, rule)?;
    }
    Ok(())
}

// ```
// iptables -t nat -N SNAT-${VM_NAME}
// # Map the port range
// iptables -A SNAT-${VM_NAME}
//          -s ${VM_IP}
//          -p tcp -m tcp
//           --dport ${RNG_START}:${RNG_END}
//          -j SNAT
//          --to-source ${PUBLIC_IP}
// iptables -A SNAT-${VM_NAME}
//          -s ${VM_IP}
//          -p tcp -m tcp
//          --dport ${RNG_START}:${RNG_END}
//          -d ${PUBLIC_IP}
//          -j MASQUERADE
// # Map SSH ports
// iptables -A SNAT-${VM_NAME}
//          -s ${VM_IP}
//          -p tcp -m tcp
//          --dport ${MAP_VM}
//          -j SNAT
//          --to-source ${PUBLIC_IP}
// iptables -A SNAT-${VM_NAME}
//          -s ${VM_IP}
//          -p tcp -m tcp
//          --dport ${MAP_HOST}
//          -d ${PUBLIC_IP}
//          -j MASQUERADE
// ```
fn snat_rules(network_settings: &NetworkSettings, name: &str) -> (String, Vec<String>) {
    let public_ip = network_settings.public_ip;
    let vm_ip = network_settings.vm_ip;
    let vm_ssh_port = network_settings.vm_ssh_port;
    let host_ssh_port = network_settings.host_ssh_port;
    let port_start = network_settings.port_range.0;
    let port_end = network_settings.port_range.1;

    let name = format!("SNAT-{name}");

    let ports_rules = format!(
        "-s {vm_ip} -p tcp -m tcp --dport {port_start}:{port_end} -j SNAT --to-source {public_ip}"
    );

    let ssh_ports_rules =
        format!("-s {vm_ip} -p tcp -m tcp --dport {vm_ssh_port} -j SNAT --to-source {public_ip}");

    let masquerade_ports_rules = format!(
        "-s {vm_ip} -p tcp -m tcp --dport {port_start}:{port_end} -d {public_ip} -j MASQUERADE"
    );

    let masquerade_ssh_ports_rules =
        format!("-s {vm_ip} -p tcp -m tcp --dport {host_ssh_port} -d {public_ip} -j MASQUERADE");

    let rules = vec![
        ports_rules,
        ssh_ports_rules,
        masquerade_ports_rules,
        masquerade_ssh_ports_rules,
    ];

    (name, rules)
}

// The corresponding iptables commands:
// ```bash
// iptables -t nat -N DNAT-${VM_NAME}
// # Map the port range
// iptables -A DNAT-${VM_NAME}  # --append chain
//          -d ${PUBLIC_IP}     # --destination
//          -p tcp              # --protocol
//          -m tcp              # --match
//          --dport ${RNG_START}:${RNG_END} # --destination-port (I don't have it my manual)
//          --to-destination ${VM_IP}:${RNG_START}-${RNG_END}
//          -j DNAT             # --jump
// # Map the SSH ports
// iptables -A DNAT-${VM_NAME}
//          -d ${PUBLIC_IP}
//          -p tcp -m tcp
//          --dport ${MAP_HOST}
//           --to-destination ${VM_IP}:${MAP_VM}
//           -j DNAT
// ```
fn dnat_rules(network_settings: &NetworkSettings, name: &str) -> (String, Vec<String>) {
    let public_ip = network_settings.public_ip;
    let vm_ip = network_settings.vm_ip;
    let vm_ssh_port = network_settings.vm_ssh_port;
    let host_ssh_port = network_settings.host_ssh_port;
    let port_start = network_settings.port_range.0;
    let port_end = network_settings.port_range.1;

    let name = format!("DNAT-{name}");

    let port_rules = format!(
        "-d {public_ip} -p tcp -m tcp --dport {port_start}:{port_end} -j DNAT \
        --to-destination {vm_ip}:{port_start}-{port_end}",
    );

    let ssh_port_rules = format!(
        "-d {public_ip} -p tcp -m tcp --dport {host_ssh_port} -j DNAT \
        --to-destination {vm_ip}:{vm_ssh_port}",
    );
    let rules = vec![port_rules, ssh_port_rules];
    (name, rules)
}

// ```
// iptables -t nat -N FWD-${VM_NAME}
// iptables -A FWD-${VM_NAME}
//          -d ${VM_IP}
//          -o ${BRIDGE_NAME}   # --out-interface
//          -p tcp -m tcp
//          --dport ${RNG_START}:${RNG_END}
//          -j ACCEPT
// iptables -A FWD-${VM_NAME}
//          -d ${VM_IP}
//          -o ${BRIDGE_NAME}
//          -p tcp -m tcp
//          --dport ${MAP_VM}
//          -j ACCEPT
// ```
fn fwd_rules(network_settings: &NetworkSettings, name: &str) -> (String, Vec<String>) {
    let vm_ip = network_settings.vm_ip;
    let vm_ssh_port = network_settings.vm_ssh_port;
    let port_start = network_settings.port_range.0;
    let port_end = network_settings.port_range.1;
    let bridge_name = &network_settings.bridge_name;

    let name = format!("FWD-{name}");

    let port_rules = format!(
        "-d {vm_ip} -o {bridge_name} -p tcp -m tcp --dport {port_start}:{port_end} -j ACCEPT"
    );

    let ssh_port_rules =
        format!("-d {vm_ip} -o {bridge_name} -p tcp -m tcp --dport {vm_ssh_port} -j ACCEPT");

    let rules = vec![port_rules, ssh_port_rules];
    (name, rules)
}
