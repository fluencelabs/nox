use iptables::IPTables;
use thiserror::Error;

pub struct NetworkSettings {
    pub public_ip: String,
    pub vm_ip: String,
    pub bridge_name: String,
    pub port_range: (u16, u16),
    pub host_ssh_port: u16,
    pub vm_ssh_port: u16,
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct DNatSetupError(#[from] Box<dyn std::error::Error>);

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SNatSetupError(#[from] Box<dyn std::error::Error>);

#[derive(Debug, Error)]
#[error(transparent)]
pub struct FwdSetupError(#[from] Box<dyn std::error::Error>);

#[derive(Debug, Error)]
pub enum NetworkSetupError {
    #[error("error setting up DNAT rules: {0}")]
    DNat(#[from] DNatSetupError),
    #[error("error setting up SNAT rules: {0}")]
    SNat(#[from] SNatSetupError),
    #[error("error setting up FWD rules: {0}")]
    Fwd(#[from] FwdSetupError),
    #[error("error initializing iptables: {0}")]
    Init(#[from] Box<dyn std::error::Error>),
}

pub fn setup_network(
    network_settings: NetworkSettings,
    name: &str,
) -> Result<(), NetworkSetupError> {
    let ipt = iptables::new(false)?;
    setup_snat(&network_settings, &ipt, name)?;
    setup_dnat(&network_settings, &ipt, name)?;
    setup_fwd(&network_settings, &ipt, name)?;
    Ok(())
}

pub fn clear_network(
    network_settings: &NetworkSettings,
    name: &str,
) -> Result<(), NetworkSetupError> {
    let ipt = iptables::new(false)?;
    let snat_result = {
        let (chain_name, rules) = snat_rules(network_settings, name);
        clear_chain(&ipt, &chain_name, &rules)
    };

    let dnat_result = {
        let (chain_name, rules) = dnat_rules(network_settings, name);
        clear_chain(&ipt, &chain_name, &rules)
    };

    let fwd_result = {
        let (chain_name, rules) = fwd_rules(network_settings, name);
        clear_chain(&ipt, &chain_name, &rules)
    };

    // Try to clean as much as possible and only then fail
    snat_result?;
    dnat_result?;
    fwd_result?;

    Ok(())
}

fn clear_chain(
    ipt: &IPTables,
    chain_name: &str,
    rules: &Vec<String>,
) -> Result<(), NetworkSetupError> {
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
) -> Result<(), DNatSetupError> {
    let (chain_name, rules) = dnat_rules(network_settings, name);
    apply_rules(ipt, &chain_name, &rules)?;
    Ok(())
}

fn setup_snat(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), SNatSetupError> {
    let (chain_name, rules) = snat_rules(network_settings, name);
    apply_rules(ipt, &chain_name, &rules)?;
    Ok(())
}

fn setup_fwd(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), FwdSetupError> {
    let (chain_name, rules) = fwd_rules(network_settings, name);
    apply_rules(ipt, &chain_name, &rules)?;
    Ok(())
}

fn apply_rules(
    ipt: &IPTables,
    name: &str,
    rules: &Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    ipt.new_chain("nat", &name)?;
    for rule in rules {
        ipt.append("nat", &name, &rule)?;
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
    let name = format!("SNAT-{name}");
    let common_prefix = &[
        format!("-s {}", network_settings.vm_ip),
        "-p tcp".to_string(),
        "-m tcp".to_string(),
    ]
    .join(" ");

    let ports_rules = &[
        common_prefix.clone(),
        format!(
            "--dport {}:{}",
            network_settings.port_range.0, network_settings.port_range.1
        ),
        "-j SNAT".to_string(),
        format!("--to-source {}", network_settings.public_ip),
    ]
    .join(" ");

    let ssh_ports_rules = &[
        common_prefix.clone(),
        format!("--dport {}", network_settings.vm_ssh_port),
        "-j SNAT".to_string(),
        format!("--to-source {}", network_settings.public_ip),
    ]
    .join(" ");

    let masquerade_ports_rules = &[
        common_prefix.clone(),
        format!(
            "--dport {}:{}",
            network_settings.port_range.0, network_settings.port_range.1
        ),
        format!("-d {}", network_settings.public_ip),
        "-j MASQUERADE".to_string(),
    ]
    .join(" ");
    let masquerade_ssh_ports_rules = &[
        common_prefix.clone(),
        format!("--dport {}", network_settings.host_ssh_port),
        format!("-d {}", network_settings.public_ip),
        "-j MASQUERADE".to_string(),
    ]
    .join(" ");

    let rules = vec![
        ports_rules.to_string(),
        ssh_ports_rules.to_string(),
        masquerade_ports_rules.to_string(),
        masquerade_ssh_ports_rules.to_string(),
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
    let name = format!("DNAT-{name}");
    let common_prefix = &[
        format!("-d {}", network_settings.public_ip),
        "-p tcp".to_string(),
        "-m tcp".to_string(),
        format!(
            "--dport {}:{}",
            network_settings.port_range.0, network_settings.port_range.1
        ),
        "-j DNAT".to_string(),
    ]
    .join(" ");

    let port_rules = &[
        common_prefix.clone(),
        format!(
            "--to-destination {}:{}-{}",
            network_settings.vm_ip, network_settings.port_range.0, network_settings.port_range.1
        ),
    ]
    .join(" ");
    let ssh_port_rules = &[
        common_prefix.clone(),
        format!(
            "--to-destination {}:{}",
            network_settings.vm_ip, network_settings.vm_ssh_port
        ),
    ]
    .join(" ");
    let rules = vec![port_rules.to_string(), ssh_port_rules.to_string()];
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
    let name = format!("FWD-{name}");
    let common_prefix = &[
        format!("-d {}", network_settings.public_ip),
        "-p tcp".to_string(),
        "-m tcp".to_string(),
        format!("-o {}", network_settings.bridge_name),
    ]
    .join(" ");
    let port_rules = &[
        common_prefix.clone(),
        format!(
            "--dport {}:{}",
            network_settings.port_range.0, network_settings.port_range.1
        ),
        "-j ACCEPT".to_string(),
    ]
    .join(" ");
    let ssh_rules = &[
        common_prefix.clone(),
        format!("--dport {}", network_settings.vm_ssh_port),
        "-j ACCEPT".to_string(),
    ]
    .join(" ");
    let rules = vec![port_rules.to_string(), ssh_rules.to_string()];
    (name, rules)
}
