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
    let ipt = iptables::new(false).map_err(|err| NetworkSetupError::Init {
        message: err.to_string(),
    })?;
    let rule_sets = vec![
        fwd_rules(network_settings, name),
        dnat_rules(network_settings, name),
        snat_rules(network_settings, name),
    ];
    for rule_set in rule_sets {
        clear_rules(&ipt, &rule_set)?;
    }
    Ok(())
}

fn clear_rules(ipt: &IPTables, rules: &RulesSet) -> Result<(), NetworkSetupError> {
    for insert_rules in &rules.insert_rules {
        clear_existing_chain_rules(ipt, insert_rules).map_err(|err| NetworkSetupError::Clean {
            chain_name: insert_rules.chain_name.clone(),
            message: err.to_string(),
        })?;
    }

    for append_rules in &rules.append_rules {
        clear_new_chain_rules(ipt, append_rules).map_err(|err| NetworkSetupError::Clean {
            chain_name: append_rules.chain_name.clone(),
            message: err.to_string(),
        })?;
    }

    Ok(())
}

fn clear_existing_chain_rules(ipt: &IPTables, rules: &IpTablesRules) -> Result<(), IpTablesError> {
    let exists = ipt.chain_exists(rules.table_name, &rules.chain_name)?;
    if !exists {
        tracing::warn!("Can't clean the chain {}: doesn't exist", rules.chain_name);
        return Ok(());
    }

    for rule in &rules.rules {
        if ipt.exists(rules.table_name, &rules.chain_name, rule)? {
            ipt.delete(rules.table_name, &rules.chain_name, rule)?;
        } else {
            tracing::warn!(
                "Can't clean a rule for the chain {chain_name}: doesn't exist; rule: {rule}",
                chain_name = rules.chain_name,
            )
        }
    }

    Ok(())
}

fn clear_new_chain_rules(ipt: &IPTables, rules: &IpTablesRules) -> Result<(), IpTablesError> {
    clear_existing_chain_rules(ipt, rules)?;
    ipt.delete_chain(rules.table_name, &rules.chain_name)?;
    Ok(())
}

fn setup_dnat(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), IpTablesError> {
    let rules = dnat_rules(network_settings, name);
    add_rules(ipt, &rules)?;
    Ok(())
}

fn setup_snat(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), IpTablesError> {
    let rules = snat_rules(network_settings, name);
    add_rules(ipt, &rules)?;
    Ok(())
}

fn setup_fwd(
    network_settings: &NetworkSettings,
    ipt: &IPTables,
    name: &str,
) -> Result<(), IpTablesError> {
    let rules = fwd_rules(network_settings, name);
    add_rules(ipt, &rules)?;
    Ok(())
}

fn add_rules(ipt: &IPTables, rules_set: &RulesSet) -> Result<(), IpTablesError> {
    for append_rules in &rules_set.append_rules {
        ipt.new_chain(append_rules.table_name, &append_rules.chain_name)?;
        for r in &append_rules.rules {
            ipt.append(append_rules.table_name, &append_rules.chain_name, r)?;
        }
    }

    for rule in &rules_set.insert_rules {
        for r in &rule.rules {
            // 1 is a default position when inserting a rule
            ipt.insert(rule.table_name, &rule.chain_name, r, 1)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct RulesSet {
    append_rules: Vec<IpTablesRules>,
    insert_rules: Vec<IpTablesRules>,
}

#[derive(Debug)]
struct IpTablesRules {
    table_name: &'static str,
    chain_name: String,
    rules: Vec<String>,
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
//          -d ${VM_IP}
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
//          -d ${VM_IP}
//          -j MASQUERADE
// ```
fn snat_rules(network_settings: &NetworkSettings, name: &str) -> RulesSet {
    let public_ip = network_settings.public_ip;
    let vm_ip = network_settings.vm_ip;
    let vm_ssh_port = network_settings.vm_ssh_port;
    let host_ssh_port = network_settings.host_ssh_port;
    let port_start = network_settings.port_range.0;
    let port_end = network_settings.port_range.1;

    let name = cut_chain_name(format!("SNAT-{name}"));

    let ports_rule = format!(
        "-s {vm_ip} -p tcp -m tcp --dport {port_start}:{port_end} -j SNAT --to-source {public_ip}"
    );

    let ssh_ports_rule =
        format!("-s {vm_ip} -p tcp -m tcp --dport {vm_ssh_port} -j SNAT --to-source {public_ip}");

    let masquerade_ports_rule = format!(
        "-s {vm_ip} -p tcp -m tcp --dport {port_start}:{port_end} -d {vm_ip} -j MASQUERADE"
    );

    let masquerade_ssh_ports_rule =
        format!("-s {vm_ip} -p tcp -m tcp --dport {host_ssh_port} -d {vm_ip} -j MASQUERADE");

    let append_rules = IpTablesRules {
        table_name: "nat",
        chain_name: name.clone(),
        rules: vec![
            ports_rule,
            ssh_ports_rule,
            masquerade_ports_rule,
            masquerade_ssh_ports_rule,
        ],
    };

    let postrouting_rule = IpTablesRules {
        table_name: "nat",
        chain_name: "POSTROUTING".to_string(),
        rules: vec![format!("-s {vm_ip} -d {vm_ip} -j {name}")],
    };

    RulesSet {
        append_rules: vec![append_rules],
        insert_rules: vec![postrouting_rule],
    }
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
//
// iptables -t nat
//          -I OUTPUT
//          -d ${PUBLIC_IP}
//          -j DNAT-${VM_NAME}
// iptables -t nat
//          -I PREROUTING
//          -d ${PUBLIC_IP}
//          -j DNAT-${VM_NAME}
// ```
fn dnat_rules(network_settings: &NetworkSettings, name: &str) -> RulesSet {
    let public_ip = network_settings.public_ip;
    let vm_ip = network_settings.vm_ip;
    let vm_ssh_port = network_settings.vm_ssh_port;
    let host_ssh_port = network_settings.host_ssh_port;
    let port_start = network_settings.port_range.0;
    let port_end = network_settings.port_range.1;

    let name = cut_chain_name(format!("DNAT-{name}"));

    let port_rule = format!(
        "-d {public_ip} -p tcp -m tcp --dport {port_start}:{port_end} -j DNAT \
        --to-destination {vm_ip}:{port_start}-{port_end}",
    );

    let ssh_port_rule = format!(
        "-d {public_ip} -p tcp -m tcp --dport {host_ssh_port} -j DNAT \
        --to-destination {vm_ip}:{vm_ssh_port}",
    );
    let append_rules = IpTablesRules {
        table_name: "nat",
        chain_name: name.clone(),
        rules: vec![port_rule, ssh_port_rule],
    };

    let prerouting_rule = IpTablesRules {
        table_name: "nat",
        chain_name: "PREROUTING".to_string(),
        rules: vec![format!("-d {public_ip} -j {name}")],
    };

    let output_rule = IpTablesRules {
        table_name: "nat",
        chain_name: "OUTPUT".to_string(),
        rules: vec![format!("-d {public_ip} -j {name}")],
    };

    RulesSet {
        append_rules: vec![append_rules],
        insert_rules: vec![output_rule, prerouting_rule],
    }
}

// ```
// iptables -t filter -N FWD-${VM_NAME}
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
// iptables -t filter
//          -I FORWARD
//          -d {VM_IP}
//          -j FWD-${VM_NAME}
// ```
fn fwd_rules(network_settings: &NetworkSettings, name: &str) -> RulesSet {
    let vm_ip = network_settings.vm_ip;
    let vm_ssh_port = network_settings.vm_ssh_port;
    let port_start = network_settings.port_range.0;
    let port_end = network_settings.port_range.1;
    let bridge_name = &network_settings.bridge_name;

    let chain_name = cut_chain_name(format!("FWD-{name}"));

    let port_rule = format!(
        "-d {vm_ip} -o {bridge_name} -p tcp -m tcp --dport {port_start}:{port_end} -j ACCEPT"
    );

    let ssh_port_rule =
        format!("-d {vm_ip} -o {bridge_name} -p tcp -m tcp --dport {vm_ssh_port} -j ACCEPT");

    let fwd_rule = format!("-d {vm_ip} -j {chain_name}");

    let append_rules = IpTablesRules {
        table_name: "filter",
        chain_name,
        rules: vec![port_rule, ssh_port_rule],
    };

    let insert_rules = IpTablesRules {
        table_name: "filter",
        chain_name: "FORWARD".to_string(),
        rules: vec![fwd_rule],
    };

    RulesSet {
        append_rules: vec![append_rules],
        insert_rules: vec![insert_rules],
    }
}

// iptables allows only 29 characters for the chain name
fn cut_chain_name(mut name: String) -> String {
    name.truncate(28);
    name
}

#[test]
fn test() {
    use std::str::FromStr;
    fn to_string(r: &RulesSet) -> String {
        let mut rules = Vec::new();
        for a in &r.append_rules {
            let fmt = format!(
                "-t {table_name} -N {chain_name}",
                table_name = a.table_name,
                chain_name = a.chain_name
            );
            rules.push(fmt);
            for rule in &a.rules {
                let fmt = format!(
                    "-t {table_name} -A {chain_name} {rule}",
                    table_name = a.table_name,
                    chain_name = a.chain_name
                );
                rules.push(fmt);
            }
        }

        for i in &r.insert_rules {
            for rule in &i.rules {
                let fmt = format!(
                    "-t {table_name} -I {chain_name} {rule}",
                    table_name = i.table_name,
                    chain_name = i.chain_name
                );
                rules.push(fmt);
            }
        }
        rules.join("\n")
    }

    let ns = NetworkSettings {
        public_ip: Ipv4Addr::from_str("1.1.1.1").unwrap(),
        vm_ip: Ipv4Addr::from_str("2.2.2.2").unwrap(),
        bridge_name: "br0".to_string(),
        port_range: (1000, 65535),
        host_ssh_port: 2222,
        vm_ssh_port: 22,
    };

    {
        let test = fwd_rules(&ns, "test");
        let result = to_string(&test);
        let expected = r#"-t filter -N FWD-test
-t filter -A FWD-test -d 2.2.2.2 -o br0 -p tcp -m tcp --dport 1000:65535 -j ACCEPT
-t filter -A FWD-test -d 2.2.2.2 -o br0 -p tcp -m tcp --dport 22 -j ACCEPT
-t filter -I FORWARD -d 2.2.2.2 -j FWD-test"#;
        assert_eq!(expected, result);
    }

    {
        let test = dnat_rules(&ns, "test");
        let result = to_string(&test);
        let expected = r#"-t nat -N DNAT-test
-t nat -A DNAT-test -d 1.1.1.1 -p tcp -m tcp --dport 1000:65535 -j DNAT --to-destination 2.2.2.2:1000-65535
-t nat -A DNAT-test -d 1.1.1.1 -p tcp -m tcp --dport 2222 -j DNAT --to-destination 2.2.2.2:22
-t nat -I OUTPUT -d 1.1.1.1 -j DNAT-test
-t nat -I PREROUTING -d 1.1.1.1 -j DNAT-test"#;
        assert_eq!(expected, result);
    }

    {
        let test = snat_rules(&ns, "test");
        let result = to_string(&test);
        let expected = r#"-t nat -N SNAT-test
-t nat -A SNAT-test -s 2.2.2.2 -p tcp -m tcp --dport 1000:65535 -j SNAT --to-source 1.1.1.1
-t nat -A SNAT-test -s 2.2.2.2 -p tcp -m tcp --dport 22 -j SNAT --to-source 1.1.1.1
-t nat -A SNAT-test -s 2.2.2.2 -p tcp -m tcp --dport 1000:65535 -d 2.2.2.2 -j MASQUERADE
-t nat -A SNAT-test -s 2.2.2.2 -p tcp -m tcp --dport 2222 -d 2.2.2.2 -j MASQUERADE
-t nat -I POSTROUTING -s 2.2.2.2 -d 2.2.2.2 -j SNAT-test"#;
        assert_eq!(expected, result);
    }
}
