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
use crate::chain_listener::types::ActiveCommitmentParams;
use ccp_rpc_client::{CCPRpcServer, ErrorObjectOwned};
use ccp_shared::proof::{BatchRequest, BatchResponse, ProofIdx};
use ccp_shared::types::{Difficulty, GlobalNonce, LogicalCoreId, PhysicalCoreId, CUID};
use jsonrpsee::core::async_trait;
use jsonrpsee::server::ServerHandle;
use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

pub struct CCPServer {
    pub address: String,
    _handle: ServerHandle,
    active_commitment_params: Arc<Mutex<VecDeque<ActiveCommitmentParams>>>,
}

impl CCPServer {
    pub async fn get_or_wait_active_commitment_params(&self) -> ActiveCommitmentParams {
        loop {
            let mut state = self.active_commitment_params.lock();
            match state.pop_front() {
                None => {
                    drop(state);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Some(value) => return value.clone(),
            }
        }
    }
}

impl CCPServer {
    pub async fn new() -> eyre::Result<Self> {
        let server = jsonrpsee::server::Server::builder()
            .set_message_buffer_capacity(10)
            .build("127.0.0.1:0")
            .await?;

        let addr = server.local_addr()?;

        let active_commitment_params = Arc::new(Mutex::new(VecDeque::new()));

        let ccp_rpc_impl = CCPRpcImpl {
            active_commitment_params: active_commitment_params.clone(),
        };
        let handle = server.start(ccp_rpc_impl.into_rpc());

        Ok(CCPServer {
            address: addr.to_string(),
            _handle: handle,
            active_commitment_params,
        })
    }
}

struct CCPRpcImpl {
    active_commitment_params: Arc<Mutex<VecDeque<ActiveCommitmentParams>>>,
}

#[async_trait]
impl CCPRpcServer for CCPRpcImpl {
    async fn on_active_commitment(
        &self,
        global_nonce: GlobalNonce,
        difficulty: Difficulty,
        cu_allocation: HashMap<PhysicalCoreId, CUID>,
    ) -> Result<(), ErrorObjectOwned> {
        let mut params = self.active_commitment_params.lock();
        params.push_back(ActiveCommitmentParams {
            global_nonce,
            difficulty,
            cu_allocation,
        });
        Ok(())
    }

    async fn on_no_active_commitment(&self) -> Result<(), ErrorObjectOwned> {
        Ok(())
    }

    async fn get_proofs_after(
        &self,
        _proof_idx: HashMap<CUID, ProofIdx>,
        _limit: usize,
    ) -> Result<Vec<BatchResponse>, ErrorObjectOwned> {
        Ok(vec![])
    }

    async fn get_batch_proofs_after(
        &self,
        _reqs: HashMap<CUID, BatchRequest>,
        _min_batch_count: usize,
        _max_batch_count: usize,
    ) -> Result<Vec<BatchResponse>, ErrorObjectOwned> {
        Ok(vec![])
    }

    async fn realloc_utility_cores(&self, _utility_core_ids: Vec<LogicalCoreId>) {}
}
