use std::collections::HashMap;
use std::sync::Arc;

use ccp_shared::proof::CCProof;
use ccp_shared::types::{Difficulty, GlobalNonce};
use clarity::Transaction;
use ethabi::ethereum_types::U256;
use ethabi::Token;
use eyre::eyre;
use futures::FutureExt;
use jsonrpsee::core::client::{BatchResponse, ClientT};
use jsonrpsee::core::params::{ArrayParams, BatchRequestBuilder};
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use jsonrpsee::tracing::log;
use serde_json::Value as JValue;
use serde_json::{json, Value};
use tokio::sync::Mutex;

use chain_data::ChainDataError::InvalidTokenSize;
use chain_data::{next_opt, parse_chain_data, peer_id_to_bytes, ChainFunction};
use chain_types::{Commitment, CommitmentId, CommitmentStatus, ComputePeer, ComputeUnit};
use fluence_libp2p::PeerId;
use particle_args::{Args, JError};
use particle_builtins::{wrap, CustomService};
use particle_execution::{ParticleParams, ServiceFunction};
use server_config::ChainConfig;

use crate::error::{process_response, ConnectorError};
use crate::function::{GetCommitmentFunction, GetStatusFunction, SubmitProofFunction};
use crate::ConnectorError::InvalidBaseFeePerGas;
use crate::{
    CurrentEpochFunction, DifficultyFunction, EpochDurationFunction, GetComputePeerFunction,
    GetComputeUnitsFunction, GetGlobalNonceFunction, InitTimestampFunction,
};

const BASE_FEE_MULTIPLIER: f64 = 0.125;

pub struct ChainConnector {
    client: Arc<jsonrpsee::http_client::HttpClient>,
    config: ChainConfig,
    tx_nonce_mutex: Arc<Mutex<()>>,
    host_id: PeerId,
}

pub struct CCInitParams {
    pub difficulty: Difficulty,
    pub init_timestamp: U256,
    pub global_nonce: GlobalNonce,
    pub current_epoch: U256,
    pub epoch_duration: U256,
}

impl ChainConnector {
    pub fn new(
        config: ChainConfig,
        host_id: PeerId,
    ) -> eyre::Result<(Arc<Self>, HashMap<String, CustomService>)> {
        log::info!("Connecting to chain via {}", config.http_endpoint);

        let connector = Arc::new(Self {
            client: Arc::new(HttpClientBuilder::default().build(&config.http_endpoint)?),
            config,
            tx_nonce_mutex: Arc::new(Default::default()),
            host_id,
        });

        let builtins = Self::make_connector_builtins(connector.clone());
        Ok((connector, builtins))
    }

    fn make_connector_builtins(connector: Arc<Self>) -> HashMap<String, CustomService> {
        let mut builtins = HashMap::new();
        builtins.insert(
            "connector".to_string(),
            CustomService::new(
                vec![("send_tx", Self::make_send_tx_closure(connector.clone()))],
                None,
            ),
        );
        builtins
    }

    fn make_send_tx_closure(connector: Arc<Self>) -> ServiceFunction {
        ServiceFunction::Immut(Box::new(move |args, params| {
            let connector = connector.clone();
            async move { wrap(connector.send_tx_builtin(args, params).await) }.boxed()
        }))
    }

    async fn send_tx_builtin(&self, args: Args, params: ParticleParams) -> Result<JValue, JError> {
        if params.init_peer_id != self.host_id {
            return Err(JError::new("Only the root worker can send transactions"));
        }

        let mut args = args.function_args.into_iter();
        let data: Vec<u8> = Args::next("data", &mut args)?;
        let to: String = Args::next("to", &mut args)?;
        let tx_hash = self
            .send_tx(data, &to)
            .await
            .map_err(|err| JError::new(format!("Failed to send tx: {err}")))?;
        Ok(json!(tx_hash))
    }

    async fn get_base_fee_per_gas(&self) -> Result<U256, ConnectorError> {
        let block: Value = process_response(
            self.client
                .request("eth_getBlockByNumber", rpc_params!["pending", false])
                .await,
        )?;

        let fee = block
            .as_object()
            .and_then(|o| o.get("baseFeePerGas"))
            .and_then(Value::as_str)
            .ok_or(InvalidBaseFeePerGas(block.to_string()))?
            .to_string();

        let base_fee_per_gas =
            U256::from_str_radix(&fee, 16).map_err(|_| InvalidBaseFeePerGas(fee))?;

        Ok(base_fee_per_gas)
    }

    async fn get_tx_nonce(&self) -> Result<U256, ConnectorError> {
        let address = self.config.wallet_key.to_address().to_string();
        let resp: String = process_response(
            self.client
                .request("eth_getTransactionCount", rpc_params![address, "pending"])
                .await,
        )?;

        let nonce =
            U256::from_str_radix(&resp, 16).map_err(|_| ConnectorError::InvalidNonce(resp))?;
        Ok(nonce)
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U256, ConnectorError> {
        let resp: String = process_response(
            self.client
                .request("eth_maxPriorityFeePerGas", rpc_params![])
                .await,
        )?;
        let max_priority_fee_per_gas =
            U256::from_str_radix(&resp, 16).map_err(|_| ConnectorError::InvalidGasLimit(resp))?;
        Ok(max_priority_fee_per_gas)
    }

    async fn estimate_gas_limit(&self, data: &[u8], to: &str) -> Result<U256, ConnectorError> {
        let resp: String = process_response(
            self.client
                .request(
                    "eth_estimateGas",
                    rpc_params![json!({
                        "from": self.config.wallet_key.to_address().to_string(),
                        "to": to,
                        "data": format!("0x{}", hex::encode(data)),
                    })],
                )
                .await,
        )?;
        let limit =
            U256::from_str_radix(&resp, 16).map_err(|_| ConnectorError::InvalidGasLimit(resp))?;
        Ok(limit)
    }

    pub async fn send_tx(&self, data: Vec<u8>, to: &str) -> Result<String, ConnectorError> {
        let base_fee_per_gas = self.get_base_fee_per_gas().await?;
        let gas_limit = self.estimate_gas_limit(&data, to).await?;
        let max_priority_fee_per_gas = self.max_priority_fee_per_gas().await?;

        let increase = (base_fee_per_gas.as_u64() as f64 * BASE_FEE_MULTIPLIER) as u128;
        let base_fee = base_fee_per_gas
            .checked_add(increase.into())
            .ok_or(InvalidBaseFeePerGas("AAAA".to_string()))?;
        // (base fee + priority fee).
        let max_fee_per_gas = base_fee + max_priority_fee_per_gas;

        // We use this lock no ensure that we don't send two transactions with the same nonce
        let _lock = self.tx_nonce_mutex.lock().await;
        let nonce = self.get_tx_nonce().await?;

        // Create a new transaction
        let tx = Transaction::Eip1559 {
            chain_id: self.config.network_id.into(),
            nonce: nonce.as_u128().into(),
            max_priority_fee_per_gas: max_priority_fee_per_gas.as_u128().into(),
            gas_limit: gas_limit.as_u128().into(),
            to: to.parse()?,
            value: 0u32.into(),
            data,
            signature: None, // Not signed. Yet.
            max_fee_per_gas: max_fee_per_gas.as_u128().into(),
            access_list: vec![],
        };

        let tx = tx
            .sign(&self.config.wallet_key, Some(self.config.network_id))
            .to_bytes();
        let tx = hex::encode(tx);

        let resp: String = process_response(
            self.client
                .request("eth_sendRawTransaction", rpc_params![format!("0x{}", tx)])
                .await,
        )?;
        Ok(resp)
    }

    pub async fn get_current_commitment_id(&self) -> Result<Option<CommitmentId>, ConnectorError> {
        let peer_id = Token::FixedBytes(peer_id_to_bytes(self.host_id));
        let data = GetComputePeerFunction::data(&[peer_id])?;
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.market_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        Ok(ComputePeer::from(&resp)?.commitment_id)
    }

    pub async fn get_commitment_status(
        &self,
        commitment_id: CommitmentId,
    ) -> Result<CommitmentStatus, ConnectorError> {
        let data = GetStatusFunction::data(&[Token::FixedBytes(commitment_id.0)])?;
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.cc_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        Ok(CommitmentStatus::from(&resp)?)
    }

    pub async fn get_commitment(
        &self,
        commitment_id: CommitmentId,
    ) -> Result<Commitment, ConnectorError> {
        let data = GetCommitmentFunction::data(&[Token::FixedBytes(commitment_id.0)])?;
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.cc_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        Ok(Commitment::from(&resp)?)
    }

    pub async fn get_global_nonce(&self) -> Result<GlobalNonce, ConnectorError> {
        let data = GetGlobalNonceFunction::data(&[])?;
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.cc_contract_address
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;

        let bytes = GetGlobalNonceFunction::decode_fixed_bytes(&resp)?;
        Ok(GlobalNonce::new(
            bytes.try_into().map_err(|_| InvalidTokenSize)?,
        ))
    }

    pub async fn submit_proof(&self, proof: CCProof) -> Result<String, ConnectorError> {
        let data = SubmitProofFunction::data_bytes(&[
            Token::FixedBytes(proof.cu_id.as_ref().to_vec()),
            Token::FixedBytes(proof.local_nonce.as_ref().to_vec()),
            Token::FixedBytes(proof.result_hash.as_ref().to_vec()),
        ])?;

        self.send_tx(data, &self.config.cc_contract_address).await
    }

    pub async fn get_compute_units(&self) -> Result<Vec<ComputeUnit>, ConnectorError> {
        let data =
            GetComputeUnitsFunction::data(&[Token::FixedBytes(peer_id_to_bytes(self.host_id))])?;
        let resp: String = process_response(
            self.client
                .request(
                    "eth_call",
                    rpc_params![
                        json!({
                            "data": data,
                            "to": self.config.market_contract_address,
                        }),
                        "latest"
                    ],
                )
                .await,
        )?;
        let mut tokens =
            parse_chain_data(&resp, &GetComputeUnitsFunction::signature())?.into_iter();
        let units = next_opt(&mut tokens, "units", Token::into_array)?.into_iter();
        let compute_units = units
            .map(ComputeUnit::from_token)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(compute_units)
    }

    pub async fn get_cc_init_params(&self) -> eyre::Result<CCInitParams> {
        let mut batch = BatchRequestBuilder::new();

        batch.insert("eth_call", self.difficulty_params()?)?;
        batch.insert("eth_call", self.init_timestamp_params()?)?;
        batch.insert("eth_call", self.global_nonce_params()?)?;
        batch.insert("eth_call", self.current_epoch_params()?)?;
        batch.insert("eth_call", self.epoch_duration_params()?)?;

        let resp: BatchResponse<String> = self.client.batch_request(batch).await?;
        let mut results = resp
            .into_ok()
            .map_err(|err| eyre!("Some request failed in a batch {err:?}"))?;

        let difficulty = DifficultyFunction::decode_fixed_bytes(
            &results.next().ok_or(eyre!("No response for difficulty"))?,
        )?;
        let init_timestamp = InitTimestampFunction::decode_uint(
            &results
                .next()
                .ok_or(eyre!("No response for init_timestamp"))?,
        )?;
        let global_nonce = GetGlobalNonceFunction::decode_fixed_bytes(
            &results
                .next()
                .ok_or(eyre!("No response for global_nonce"))?,
        )?;
        let current_epoch = CurrentEpochFunction::decode_uint(
            &results
                .next()
                .ok_or(eyre!("No response for current_epoch"))?,
        )?;
        let epoch_duration = EpochDurationFunction::decode_uint(
            &results
                .next()
                .ok_or(eyre!("No response for epoch_duration"))?,
        )?;

        Ok(CCInitParams {
            difficulty: Difficulty::new(
                difficulty
                    .try_into()
                    .map_err(|_| eyre!("Failed to convert difficulty"))?,
            ),
            init_timestamp,
            global_nonce: GlobalNonce::new(
                global_nonce
                    .try_into()
                    .map_err(|_| eyre!("Failed to convert global_nonce"))?,
            ),
            current_epoch,
            epoch_duration,
        })
    }

    fn difficulty_params(&self) -> eyre::Result<ArrayParams> {
        let data = DifficultyFunction::data(&[])?;
        Ok(rpc_params![
            json!({"data": data, "to": self.config.cc_contract_address}),
            "latest"
        ])
    }

    fn init_timestamp_params(&self) -> eyre::Result<ArrayParams> {
        let data = InitTimestampFunction::data(&[])?;
        Ok(rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ])
    }
    fn global_nonce_params(&self) -> eyre::Result<ArrayParams> {
        let data = GetGlobalNonceFunction::data(&[])?;
        Ok(rpc_params![
            json!({"data": data, "to": self.config.cc_contract_address}),
            "latest"
        ])
    }
    fn current_epoch_params(&self) -> eyre::Result<ArrayParams> {
        let data = CurrentEpochFunction::data(&[])?;
        Ok(rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ])
    }
    fn epoch_duration_params(&self) -> eyre::Result<ArrayParams> {
        let data = EpochDurationFunction::data(&[])?;
        Ok(rpc_params![
            json!({"data": data, "to": self.config.core_contract_address}),
            "latest"
        ])
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use std::str::FromStr;
    use std::sync::Arc;

    use ccp_shared::proof::{CCProof, CCProofId, ProofIdx};
    use ccp_shared::types::{Difficulty, GlobalNonce, LocalNonce, ResultHash, CUID};
    use clarity::PrivateKey;
    use hex::FromHex;
    use mockito::Matcher;
    use serde_json::json;

    use chain_data::peer_id_from_hex;
    use chain_types::{CommitmentId, COMMITMENT_IS_NOT_ACTIVE};

    use crate::{ChainConnector, ConnectorError};

    fn get_connector(url: &str) -> Arc<ChainConnector> {
        let (connector, _) = ChainConnector::new(
            server_config::ChainConfig {
                http_endpoint: url.to_string(),
                cc_contract_address: "0x8dc7d48492b9fD2519b65A54816be03758742c60".to_string(),
                core_contract_address: "0x0B306BF915C4d645ff596e518fAf3F9669b97016".to_string(),
                market_contract_address: "0x68B1D87F95878fE05B998F19b66F4baba5De1aed".to_string(),
                network_id: 3525067388221321,
                wallet_key: PrivateKey::from_str(
                    "0x97a2456e78c4894c62eef6031972d1ca296ed40bf311ab54c231f13db59fc428",
                )
                .unwrap(),
            },
            peer_id_from_hex("0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b")
                .unwrap(),
        )
        .unwrap();

        connector
    }

    #[tokio::test]
    async fn test_get_compute_units() {
        let expected_data = "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000025d204dcc21f59c2a2098a277e48879207f614583e066654ad6736d36815ebb9e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000450e2f2a5bdb528895e9005f67e70fe213b9b822122e96fd85d2238cae55b6f900000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();

        let units = get_connector(&url).get_compute_units().await.unwrap();

        mock.assert();
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].start_epoch, 0.into());
        assert!(units[0].deal.is_none());
        assert_eq!(units[1].start_epoch, 0.into());
        assert!(units[1].deal.is_none());
    }

    #[tokio::test]
    async fn test_get_current_commitment_id_none() {
        let expected_data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();
        let commitment_id = get_connector(&url)
            .get_current_commitment_id()
            .await
            .unwrap();

        mock.assert();
        assert!(commitment_id.is_none());
    }

    #[tokio::test]
    async fn test_get_current_commitment_id_some() {
        let expected_data = "0xaa3046a12a1aac6e840625e6329d70b427328fec36dc8d273e5e6454b85633d5aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000005b73c5498c1e3b4dba84de0f1833c4a029d90519";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");

        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();
        let commitment_id = get_connector(&url)
            .get_current_commitment_id()
            .await
            .unwrap();

        mock.assert();
        assert!(commitment_id.is_some());
        assert_eq!(
            hex::encode(commitment_id.unwrap().0),
            "aa3046a12a1aac6e840625e6329d70b427328feceedc8d273e5e6454b85633b5"
        );
    }

    #[tokio::test]
    async fn test_get_commitment() {
        let commitment_id = "0xa98dc43600773b162bcdb8175eadc037412cd7ad83555fafa507702011a53992";

        let expected_data = "0x00000000000000000000000000000000000000000000000000000000000000016497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000012c00000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");
        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();
        let commitment_id = CommitmentId(hex::decode(&commitment_id[2..]).unwrap());
        let commitment = get_connector(&url)
            .get_commitment(commitment_id)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(
            commitment.status,
            chain_types::CommitmentStatus::WaitDelegation
        );
        assert_eq!(commitment.start_epoch, 0.into());
        assert_eq!(commitment.end_epoch, 300.into());
    }

    #[tokio::test]
    async fn get_commitment_status() {
        let commitment_id = "0xa98dc43600773b162bcdb8175eadc037412cd7ad83555fafa507702011a53992";

        let expected_data = "0x0000000000000000000000000000000000000000000000000000000000000001";
        let expected_response =
            format!("{{\"jsonrpc\":\"2.0\",\"result\":\"{expected_data}\",\"id\":0}}");
        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .match_body(Matcher::PartialJson(json!({
                "method": "eth_call",
            })))
            .with_body(expected_response)
            .create();
        let commitment_id = CommitmentId(hex::decode(&commitment_id[2..]).unwrap());
        let status = get_connector(&url)
            .get_commitment_status(commitment_id)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(status, chain_types::CommitmentStatus::WaitDelegation);
    }

    #[tokio::test]
    async fn test_batch_init_request() {
        let expected_response = r#"[
          {
            "jsonrpc": "2.0",
            "result": "0x76889c92f61b9c5df216e048df56eb8f4eb02f172ab0d5b04edb9190ab9c9eec",
            "id": 0
          },
          {
            "jsonrpc": "2.0",
            "result": "0x0000000000000000000000000000000000000000000000000000000065ca5a01",
            "id": 1
          },
          {
            "jsonrpc": "2.0",
            "result": "0x0000000000000000000000000000000000000000000000000000000000000005",
            "id": 2
          },
          {
            "jsonrpc": "2.0",
            "result": "0x00000000000000000000000000000000000000000000000000000000000016be",
            "id": 3
          },
          {
            "jsonrpc": "2.0",
            "result": "0x000000000000000000000000000000000000000000000000000000000000000f",
            "id": 4
          }
        ]"#;
        let mut server = mockito::Server::new();
        let url = server.url();
        let mock = server
            .mock("POST", "/")
            // expect exactly 1 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(expected_response)
            .create();

        let init_params = get_connector(&url).get_cc_init_params().await.unwrap();

        mock.assert();
        assert_eq!(
            init_params.difficulty,
            <Difficulty>::from_hex(
                "76889c92f61b9c5df216e048df56eb8f4eb02f172ab0d5b04edb9190ab9c9eec"
            )
            .unwrap()
        );
        assert_eq!(init_params.init_timestamp, 1707760129.into());
        assert_eq!(
            init_params.global_nonce,
            <GlobalNonce>::from_hex(
                "0000000000000000000000000000000000000000000000000000000000000005"
            )
            .unwrap()
        );
        assert_eq!(
            init_params.current_epoch,
            0x00000000000000000000000000000000000000000000000000000000000016be.into()
        );
        assert_eq!(
            init_params.epoch_duration,
            0x000000000000000000000000000000000000000000000000000000000000000f.into()
        );
    }

    #[tokio::test]
    async fn submit_proof_not_active() {
        let get_block_by_number = r#"{"jsonrpc":"2.0","id":0,"result":{"hash":"0xcbe8d90665392babc8098738ec78009193c99d3cc872a6657e306cfe8824bef9","parentHash":"0x15e767118a3e2d7545fee290b545faccd4a9eff849ac1057ce82cab7100c0c52","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":"0xa2","gasLimit":"0x1c9c380","gasUsed":"0x0","timestamp":"0x65d88f76","extraData":"0x","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000","baseFeePerGas":"0x7","totalDifficulty":"0x0","uncles":[],"transactions":[],"size":"0x220"}}"#;
        let estimate_gas = r#"
        {
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32000,
                "message": "execution reverted: revert: Capacity commitment is not active",
                "data": "0x08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000021436170616369747920636f6d6d69746d656e74206973206e6f742061637469766500000000000000000000000000000000000000000000000000000000000000"
            }
        }
        "#;

        let mut server = mockito::Server::new();
        let url = server.url();
        server
            .mock("POST", "/")
            // expect exactly 4 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: serde_json::Value =
                    serde_json::from_slice(body).expect("mock: parse request body");
                let method = body.get("method").expect("get method");
                let method = method.as_str().expect("as str").trim_matches(|c| c == '\"');

                match method {
                    "eth_getBlockByNumber" => get_block_by_number.into(),
                    "eth_estimateGas" => estimate_gas.into(),
                    method => format!("'{}' not supported", method).into(),
                }
            })
            .create();

        let proof = CCProof::new(
            CCProofId::new(
                GlobalNonce::new([0u8; 32].into()),
                Difficulty::new([0u8; 32].into()),
                ProofIdx::zero(),
            ),
            LocalNonce::new([0u8; 32].into()),
            CUID::new([0u8; 32].into()),
            ResultHash::from_slice([0u8; 32].into()),
        );
        let result = get_connector(&url).submit_proof(proof).await;

        assert!(result.is_err());

        assert_matches!(
            result.unwrap_err(),
            ConnectorError::RpcCallError {
                code: _,
                message: _,
                data,
            } if data.contains(COMMITMENT_IS_NOT_ACTIVE)
        );
    }

    #[tokio::test]
    async fn submit_proof() {
        let get_block_by_number = r#"{"jsonrpc":"2.0","id":0,"result":{"hash":"0xcbe8d90665392babc8098738ec78009193c99d3cc872a6657e306cfe8824bef9","parentHash":"0x15e767118a3e2d7545fee290b545faccd4a9eff849ac1057ce82cab7100c0c52","sha3Uncles":"0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347","miner":"0x0000000000000000000000000000000000000000","stateRoot":"0x0000000000000000000000000000000000000000000000000000000000000000","transactionsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","receiptsRoot":"0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421","logsBloom":"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000","difficulty":"0x0","number":"0xa2","gasLimit":"0x1c9c380","gasUsed":"0x0","timestamp":"0x65d88f76","extraData":"0x","mixHash":"0x0000000000000000000000000000000000000000000000000000000000000000","nonce":"0x0000000000000000","baseFeePerGas":"0x7","totalDifficulty":"0x0","uncles":[],"transactions":[],"size":"0x220"}}"#;
        let estimate_gas = r#"{"jsonrpc":"2.0","id": 1,"result": "0x5208"}"#;
        let max_priority_fee = r#"{"jsonrpc":"2.0","id": 2,"result": "0x5208"}"#;
        let nonce = r#"{"jsonrpc":"2.0","id":3,"result":"0x20"}"#;
        let send_tx_response = r#"
            {
                "jsonrpc": "2.0",
                "id": 4,
                "result": "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
            }
            "#;
        let mut server = mockito::Server::new();
        let url = server.url();
        server
            .mock("POST", "/")
            // expect exactly 4 POST request
            .expect(1)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body_from_request(move |req| {
                let body = req.body().expect("mock: get request body");
                let body: serde_json::Value =
                    serde_json::from_slice(body).expect("mock: parse request body");
                let method = body.get("method").expect("get method");
                let method = method.as_str().expect("as str").trim_matches(|c| c == '\"');

                match method {
                    "eth_getBlockByNumber" => get_block_by_number.into(),
                    "eth_estimateGas" => estimate_gas.into(),
                    "eth_maxPriorityFeePerGas" => max_priority_fee.into(),
                    "eth_getTransactionCount" => nonce.into(),
                    "eth_sendRawTransaction" => send_tx_response.into(),
                    method => format!("'{}' not supported", method).into(),
                }
            })
            .create();

        let proof = CCProof::new(
            CCProofId::new(
                GlobalNonce::new([0u8; 32].into()),
                Difficulty::new([0u8; 32].into()),
                ProofIdx::zero(),
            ),
            LocalNonce::new([0u8; 32].into()),
            CUID::new([0u8; 32].into()),
            ResultHash::from_slice([0u8; 32].into()),
        );
        let result = get_connector(&url).submit_proof(proof).await.unwrap();

        assert_eq!(
            result,
            "0x55bfec4a4400ca0b09e075e2b517041cd78b10021c51726cb73bcba52213fa05"
        );
    }
}
