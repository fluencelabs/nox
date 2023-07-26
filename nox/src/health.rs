use health::HealthCheck;
use libp2p::Multiaddr;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct ConnectivityHealthChecks {
    pub bootstrap_nodes: BootstrapNodesHealthCheck,
}

#[derive(Clone)]
pub struct BootstrapNodesHealthCheck {
    bootstrap_nodes_statuses: Arc<Mutex<HashMap<Multiaddr, bool>>>,
}

impl BootstrapNodesHealthCheck {
    pub fn new(bootstrap_nodes: Vec<Multiaddr>) -> Self {
        let bootstrap_nodes_statuses = bootstrap_nodes
            .into_iter()
            .map(|addr| (addr, false))
            .collect::<HashMap<_, _>>();
        Self {
            bootstrap_nodes_statuses: Arc::new(Mutex::new(bootstrap_nodes_statuses)),
        }
    }

    pub fn on_bootstrap_disconnected(&self, addresses: Vec<Multiaddr>) {
        let mut guard = self.bootstrap_nodes_statuses.lock();
        for addr in addresses {
            guard.insert(addr, false);
        }
    }

    pub fn on_bootstrap_connected(&self, addr: Multiaddr) {
        let mut guard = self.bootstrap_nodes_statuses.lock();
        guard.insert(addr, true);
    }
}

impl HealthCheck for BootstrapNodesHealthCheck {
    fn check(&self) -> eyre::Result<()> {
        let guard = self.bootstrap_nodes_statuses.lock();
        for (addr, connected) in guard.iter() {
            if !connected {
                return Err(eyre::eyre!("Bootstrap {} is not connected", addr));
            }
        }
        Ok(())
    }
}
