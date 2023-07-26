use health::HealthCheck;
use libp2p::Multiaddr;
use parking_lot::{RwLock};
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
