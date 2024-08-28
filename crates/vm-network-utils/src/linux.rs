/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

#![cfg(target_os = "linux")]

pub use iptables::IPTables;
use crate::{IpTablesError, IpTablesRules, NetworkSettings, NetworkSetupError, RulesSet, setup_snat, setup_dnat, setup_fwd, fwd_rules, dnat_rules, snat_rules};

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

pub fn clear_rules(ipt: &IPTables, rules: &RulesSet) -> Result<(), NetworkSetupError> {
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

pub fn clear_existing_chain_rules(ipt: &IPTables, rules: &IpTablesRules) -> Result<(), IpTablesError> {
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

pub fn clear_new_chain_rules(ipt: &IPTables, rules: &IpTablesRules) -> Result<(), IpTablesError> {
    clear_existing_chain_rules(ipt, rules)?;
    ipt.delete_chain(rules.table_name, &rules.chain_name)?;
    Ok(())
}

pub fn add_rules(ipt: &IPTables, rules_set: &RulesSet) -> Result<(), IpTablesError> {
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
