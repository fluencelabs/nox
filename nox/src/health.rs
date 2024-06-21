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

use health::HealthCheck;
use libp2p::Multiaddr;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct ConnectivityHealth {
    pub bootstrap_nodes: BootstrapNodesHealth,
    pub kademlia_bootstrap: KademliaBootstrapHealth,
}

#[derive(Clone)]
pub struct BootstrapNodesHealth {
    bootstrap_nodes_statuses: Arc<RwLock<HashMap<Multiaddr, bool>>>,
}

impl BootstrapNodesHealth {
    pub fn new(bootstrap_nodes: Vec<Multiaddr>) -> Self {
        let bootstrap_nodes_statuses = bootstrap_nodes
            .into_iter()
            .map(|addr| (addr, false))
            .collect::<HashMap<_, _>>();
        Self {
            bootstrap_nodes_statuses: Arc::new(RwLock::new(bootstrap_nodes_statuses)),
        }
    }

    pub fn on_bootstrap_disconnected(&self, addresses: Vec<Multiaddr>) {
        let mut guard = self.bootstrap_nodes_statuses.write();
        for addr in addresses {
            guard.insert(addr, false);
        }
    }

    pub fn on_bootstrap_connected(&self, addr: Multiaddr) {
        let mut guard = self.bootstrap_nodes_statuses.write();
        guard.insert(addr, true);
    }
}

impl HealthCheck for BootstrapNodesHealth {
    fn status(&self) -> eyre::Result<()> {
        let guard = self.bootstrap_nodes_statuses.read();
        for (addr, connected) in guard.iter() {
            if !connected {
                return Err(eyre::eyre!("Bootstrap {} is not connected", addr));
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct KademliaBootstrapHealth {
    status: Arc<AtomicBool>,
}

impl KademliaBootstrapHealth {
    pub fn on_boostrap_finished(&self) {
        self.status.store(true, Ordering::Release)
    }

    pub fn on_boostrap_failed(&self) {
        self.status.store(false, Ordering::Release)
    }
}

impl Default for KademliaBootstrapHealth {
    fn default() -> Self {
        Self {
            status: Arc::new(AtomicBool::default()),
        }
    }
}

impl HealthCheck for KademliaBootstrapHealth {
    fn status(&self) -> eyre::Result<()> {
        let status = self.status.load(Ordering::Acquire);
        if status {
            Ok(())
        } else {
            Err(eyre::eyre!("Kademlia bootstrap not finished"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_bootstrap_nodes_health_all_connected() {
        let bootstrap_nodes = vec![
            "/ip4/127.0.0.1/tcp/5000".parse().unwrap(),
            "/ip4/127.0.0.1/tcp/5001".parse().unwrap(),
        ];
        let bootstrap_health = BootstrapNodesHealth::new(bootstrap_nodes.clone());

        // Simulate connections to the bootstrap nodes
        bootstrap_health.on_bootstrap_connected(bootstrap_nodes[0].clone());
        bootstrap_health.on_bootstrap_connected(bootstrap_nodes[1].clone());

        let status = bootstrap_health.status();
        assert!(status.is_ok());
    }

    #[test]
    fn test_bootstrap_nodes_health_partial_connected() {
        let bootstrap_nodes = vec![
            "/ip4/127.0.0.1/tcp/5000".parse().unwrap(),
            "/ip4/127.0.0.1/tcp/5001".parse().unwrap(),
        ];
        let bootstrap_health = BootstrapNodesHealth::new(bootstrap_nodes.clone());

        // Simulate connections to only one of the bootstrap nodes
        bootstrap_health.on_bootstrap_connected(bootstrap_nodes[0].clone());

        let status = bootstrap_health.status();
        assert!(status.is_err());
    }

    #[test]
    fn test_bootstrap_nodes_health_concurrent_access() {
        let bootstrap_nodes = vec![
            "/ip4/127.0.0.1/tcp/5000".parse().unwrap(),
            "/ip4/127.0.0.1/tcp/5001".parse().unwrap(),
        ];
        let bootstrap_health = Arc::new(BootstrapNodesHealth::new(bootstrap_nodes.clone()));
        let health_clone = bootstrap_health.clone();

        let bootstrap_node = bootstrap_nodes[0].clone();
        // Simulate concurrent access by spawning multiple threads.
        let thread_handle = thread::spawn(move || {
            let health = health_clone;
            // Simulate connections to the bootstrap nodes in separate threads.
            health.on_bootstrap_connected(bootstrap_node);
        });

        // Simulate connections to the bootstrap nodes in the main thread.
        bootstrap_health.on_bootstrap_connected(bootstrap_nodes[1].clone());

        // Wait for the spawned thread to finish.
        thread_handle.join().unwrap();

        let status = bootstrap_health.status();
        assert!(status.is_ok());
    }

    #[test]
    fn new_health_instance_should_have_default_status_false() {
        let health = KademliaBootstrapHealth::default();
        assert!(!health.status.load(Ordering::Acquire));
    }

    #[test]
    fn on_bootstrap_finished_should_set_status_to_true() {
        let health = KademliaBootstrapHealth::default();
        health.on_boostrap_finished();
        assert!(health.status.load(Ordering::Acquire));
    }

    #[test]
    fn on_bootstrap_failed_should_set_status_to_false() {
        let health = KademliaBootstrapHealth::default();
        health.on_boostrap_failed();
        assert!(!health.status.load(Ordering::Acquire));
    }

    #[test]
    fn status_should_return_ok_if_bootstrap_finished() {
        let health = KademliaBootstrapHealth::default();
        health.on_boostrap_finished();
        assert!(health.status().is_ok());
    }

    #[test]
    fn status_should_return_error_if_bootstrap_failed() {
        let health = KademliaBootstrapHealth::default();
        health.on_boostrap_failed();
        assert!(health.status().is_err());
    }

    #[test]
    fn status_error_should_contain_expected_message() {
        let health = KademliaBootstrapHealth::default();
        health.on_boostrap_failed();
        let result = health.status();
        assert_eq!(
            result.err().unwrap().to_string(),
            "Kademlia bootstrap not finished"
        );
    }
}
