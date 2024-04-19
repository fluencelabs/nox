use ccp_rpc_client::{CCPRpcClient, CCPRpcHttpClient, ClientError};
use ccp_shared::proof::{CCProof, ProofIdx};
use ccp_shared::types::{Difficulty, GlobalNonce, LogicalCoreId, PhysicalCoreId, CUID};
use jsonrpsee::core::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait CCPClient: Sync + Send {
    async fn on_no_active_commitment(&self) -> Result<(), ClientError>;

    async fn realloc_utility_cores(
        &self,
        utility_core_ids: Vec<LogicalCoreId>,
    ) -> Result<(), ClientError>;

    async fn on_active_commitment(
        &self,
        global_nonce: GlobalNonce,
        difficulty: Difficulty,
        cu_allocation: HashMap<PhysicalCoreId, CUID>,
    ) -> Result<(), ClientError>;

    async fn get_proofs_after(
        &self,
        proof_idx: ProofIdx,
        limit: usize,
    ) -> Result<Vec<CCProof>, ClientError>;
}

#[async_trait]
impl CCPClient for CCPRpcHttpClient {
    async fn on_no_active_commitment(&self) -> Result<(), ClientError> {
        self.on_no_active_commitment().await
    }

    async fn realloc_utility_cores(
        &self,
        utility_core_ids: Vec<LogicalCoreId>,
    ) -> Result<(), ClientError> {
        self.realloc_utility_cores(utility_core_ids).await
    }

    async fn on_active_commitment(
        &self,
        global_nonce: GlobalNonce,
        difficulty: Difficulty,
        cu_allocation: HashMap<PhysicalCoreId, CUID>,
    ) -> Result<(), ClientError> {
        self.on_active_commitment(global_nonce, difficulty, cu_allocation)
            .await
    }

    async fn get_proofs_after(
        &self,
        proof_idx: ProofIdx,
        limit: usize,
    ) -> Result<Vec<CCProof>, ClientError> {
        self.get_proofs_after(proof_idx, limit).await
    }
}
