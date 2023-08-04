use health::HealthCheck;
use libp2p::Multiaddr;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConnectivityHealth {
    pub bootstrap_nodes: BootstrapNodesHealth,
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
}
