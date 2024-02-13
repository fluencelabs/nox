use crate::function::{GetCommitmentFunction, GetStatusFunction, SubmitProofFunction};
use crate::{
    CurrentEpochFunction, DifficultyFunction, EpochDurationFunction, GetComputePeerFunction,
    GetComputeUnitsFunction, GetGlobalNonceFunction, InitTimestampFunction,
};
use chain_data::{next_opt, parse_chain_data, peer_id_to_bytes, FunctionTrait};
use chain_types::{
    Commitment, CommitmentId, CommitmentStatus, ComputePeer, ComputeUnit, GlobalNonce, Proof,
};
use clarity::{PrivateKey, Transaction};
use ethabi::ethereum_types::U256;
use ethabi::{ParamType, Token};
use eyre::eyre;
use fluence_libp2p::PeerId;
use jsonrpsee::core::client::{BatchResponse, ClientT};
use jsonrpsee::core::params::BatchRequestBuilder;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;
use serde_json::json;
use server_config::ChainListenerConfig;

const GAS_MULTIPLIER: f64 = 0.0;
pub struct ChainConnector;

pub struct ChainListenerInitParams {
    pub difficulty: Vec<u8>,
    pub init_timestamp: U256,
    pub global_nonce: Vec<u8>,
    pub current_epoch: U256,
    pub epoch_duration: U256,
}

impl ChainConnector {
    fn make_tx(
        data: Vec<u8>,
        wallet_key: PrivateKey,
        gas_limit: u64,
        nonce: u128,
        gas_price: u128,
        contract_address: &str,
        network_id: u64,
    ) -> eyre::Result<String> {
        // Create a new transaction
        let tx = Transaction::Legacy {
            nonce: nonce.into(),
            gas_price: gas_price.into(),
            gas_limit: gas_limit.into(),
            to: contract_address.parse()?,
            value: 0u32.into(),
            data,
            signature: None, // Not signed. Yet.
        };

        let tx = tx.sign(&wallet_key, Some(network_id)).to_bytes();
        let tx = hex::encode(tx);

        Ok(format!("0x{}", tx))
    }

    pub async fn get_current_commitment_id(
        endpoint: &str,
        contract_address: &str,
        host_id: PeerId,
    ) -> eyre::Result<Option<CommitmentId>> {
        let client = HttpClientBuilder::default().build(endpoint)?;

        let peer_id = Token::FixedBytes(peer_id_to_bytes(host_id));
        let data = GetComputePeerFunction::data(&[peer_id])?;
        let resp: String = client
            .request(
                "eth_call",
                rpc_params![json!({
                    "data": data,
                    "to": contract_address,
                })],
            )
            .await?;
        Ok(ComputePeer::from(&resp)?.commitment_id)
    }

    pub async fn get_commitment_status(
        endpoint: &str,
        contract_address: String,
        commitment_id: CommitmentId,
    ) -> eyre::Result<CommitmentStatus> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let data = GetStatusFunction::data(&[Token::FixedBytes(commitment_id.0)])?;
        let resp: String = client
            .request(
                "eth_call",
                rpc_params![json!({
                    "data": data,
                    "to": contract_address,
                })],
            )
            .await?;
        CommitmentStatus::from(&resp)
    }

    pub async fn get_commitment(
        endpoint: &str,
        contract_address: &str,
        commitment_id: CommitmentId,
    ) -> eyre::Result<Commitment> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let data = GetCommitmentFunction::data(&[Token::FixedBytes(commitment_id.0)])?;
        let resp: String = client
            .request(
                "eth_call",
                rpc_params![json!({
                    "data": data,
                    "to": contract_address,
                })],
            )
            .await?;
        Commitment::from(&resp)
    }

    pub async fn get_global_nonce(
        endpoint: &str,
        contract_address: &str,
    ) -> eyre::Result<GlobalNonce> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let data = GetGlobalNonceFunction::data(&[])?;
        let resp: String = client
            .request(
                "eth_call",
                rpc_params![json!({
                    "data": data,
                    "to": contract_address,
                })],
            )
            .await?;

        Ok(GlobalNonce(GetGlobalNonceFunction::decode_bytes(&resp)?))
    }

    pub async fn submit_proof(
        endpoint: &str,
        contract_address: &str,
        proof: Proof,
        private_key: PrivateKey,
    ) -> eyre::Result<String> {
        let data = SubmitProofFunction::data_bytes(&[
            Token::FixedBytes(proof.unit_id.0),
            Token::FixedBytes(proof.global_unit_nonce),
            Token::FixedBytes(proof.local_unit_nonce),
            Token::FixedBytes(proof.target_hash),
        ])?;

        let nonce = Self::get_tx_nonce(endpoint, private_key.to_address()).await?;
        let gas_price = Self::get_gas_price(endpoint).await?;
        let tx = Self::make_tx(
            data,
            private_key,
            // TODO: pass correct gas limit
            100_000,
            nonce,
            gas_price,
            &contract_address,
            // TODO: pass correct network id
            1,
        )?;

        Self::send_tx(endpoint, tx).await
    }

    async fn send_tx(endpoint: &str, tx: String) -> eyre::Result<String> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let resp: String = client
            .request("eth_sendRawTransaction", rpc_params![tx])
            .await?;
        Ok(resp)
    }

    async fn get_gas_price(endpoint: &str) -> eyre::Result<u128> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let resp: String = client.request("eth_gasPrice", rpc_params![]).await?;

        let mut tokens = parse_chain_data(&resp, &[ParamType::Uint(128)])?.into_iter();
        let price = next_opt(&mut tokens, "gas_price", Token::into_uint)?.as_u128();

        // increase price by GAS_MULTIPLIER so transaction are included faster
        let increase = (price as f64 * GAS_MULTIPLIER) as u128;
        let price = price.checked_add(increase).unwrap_or(price);

        Ok(price)
    }

    async fn get_tx_nonce(endpoint: &str, wallet: clarity::Address) -> eyre::Result<u128> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let resp: String = client
            .request(
                "eth_getTransactionCount",
                rpc_params![wallet.to_string(), "pending"],
            )
            .await?;

        let mut tokens = parse_chain_data(&resp, &[ParamType::Uint(128)])?.into_iter();
        let nonce = next_opt(&mut tokens, "nonce", Token::into_uint)?.as_u128();
        Ok(nonce)
    }

    pub async fn get_compute_units(
        endpoint: &str,
        contract_address: &str,
        peer_id: PeerId,
    ) -> eyre::Result<Vec<ComputeUnit>> {
        let client = HttpClientBuilder::default().build(endpoint)?;
        let data = GetComputeUnitsFunction::data(&[Token::FixedBytes(peer_id_to_bytes(peer_id))])?;
        let resp: String = client
            .request(
                "eth_call",
                rpc_params![json!({
                    "data": data,
                    "to": contract_address,
                })],
            )
            .await?;
        let mut tokens =
            parse_chain_data(&resp, &GetComputeUnitsFunction::signature())?.into_iter();
        let units = next_opt(&mut tokens, "units", Token::into_array)?.into_iter();
        let compute_units = units
            .map(ComputeUnit::from_token)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(compute_units)
    }

    pub async fn batch_init_request(
        config: &ChainListenerConfig,
    ) -> eyre::Result<ChainListenerInitParams> {
        let client = HttpClientBuilder::default().build(&config.http_endpoint)?;
        let mut batch = BatchRequestBuilder::new();

        append_difficulty_req(&mut batch, &config.cc_contract_address)?;
        append_init_timestamp_req(&mut batch, &config.core_contract_address)?;
        append_global_nonce_req(&mut batch, &config.cc_contract_address)?;
        append_current_epoch_req(&mut batch, &config.core_contract_address)?;
        append_epoch_duration_req(&mut batch, &config.core_contract_address)?;

        let resp: BatchResponse<String> = client.batch_request(batch).await?;
        let mut results = resp
            .into_ok()
            .map_err(|err| eyre!("Some request failed in a batch {err:?}"))?;

        // TODO remove unwraps
        let difficulty = DifficultyFunction::decode_bytes(&results.next().unwrap())?;
        let init_timestamp = InitTimestampFunction::decode_uint(&results.next().unwrap())?;
        let global_nonce = GetGlobalNonceFunction::decode_bytes(&results.next().unwrap())?;
        let current_epoch = CurrentEpochFunction::decode_uint(&results.next().unwrap())?;
        let epoch_duration = EpochDurationFunction::decode_uint(&results.next().unwrap())?;

        Ok(ChainListenerInitParams {
            difficulty,
            init_timestamp,
            global_nonce,
            current_epoch,
            epoch_duration,
        })
    }
}

fn append_difficulty_req(
    batch: &mut BatchRequestBuilder,
    cc_contract_address: &str,
) -> eyre::Result<()> {
    let data = DifficultyFunction::data(&[])?;
    batch.insert(
        "eth_call",
        rpc_params![json!({"data": data, "to": cc_contract_address})],
    )?;
    Ok(())
}

fn append_init_timestamp_req(
    batch: &mut BatchRequestBuilder,
    core_contract_address: &str,
) -> eyre::Result<()> {
    let data = InitTimestampFunction::data(&[])?;
    batch.insert(
        "eth_call",
        rpc_params![json!({"data": data, "to": core_contract_address})],
    )?;
    Ok(())
}

fn append_global_nonce_req(
    batch: &mut BatchRequestBuilder,
    cc_contract_address: &str,
) -> eyre::Result<()> {
    let data = GetGlobalNonceFunction::data(&[])?;
    batch.insert(
        "eth_call",
        rpc_params![json!({"data": data, "to": cc_contract_address})],
    )?;
    Ok(())
}

fn append_current_epoch_req(
    batch: &mut BatchRequestBuilder,
    core_contract_address: &str,
) -> eyre::Result<()> {
    let data = CurrentEpochFunction::data(&[])?;
    batch.insert(
        "eth_call",
        rpc_params![json!({"data": data, "to": core_contract_address})],
    )?;
    Ok(())
}

fn append_epoch_duration_req(
    batch: &mut BatchRequestBuilder,
    core_contract_address: &str,
) -> eyre::Result<()> {
    let data = EpochDurationFunction::data(&[])?;
    batch.insert(
        "eth_call",
        rpc_params![json!({"data": data, "to": core_contract_address})],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::ChainConnector;
    use chain_data::peer_id_from_hex;
    use chain_types::CommitmentId;

    #[tokio::test]
    async fn test_get_compute_units() {
        let peer_id_hex = "0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b";
        let peer_id = peer_id_from_hex(peer_id_hex).unwrap();
        let contract_address = "0x68B1D87F95878fE05B998F19b66F4baba5De1aed";

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
        let units = ChainConnector::get_compute_units(&url, &contract_address, peer_id)
            .await
            .unwrap();

        mock.assert();
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].start_epoch, 0.into());
        assert!(units[0].deal.is_none());
        assert_eq!(units[1].start_epoch, 0.into());
        assert!(units[1].deal.is_none());
    }

    #[tokio::test]
    async fn test_get_current_commitment_id_none() {
        let peer_id_hex = "0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b";
        let peer_id = peer_id_from_hex(peer_id_hex).unwrap();
        let contract_address = "0x3Aa5ebB10DC797CAC828524e59A333d0A371443c";

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
        let commitment_id =
            ChainConnector::get_current_commitment_id(&url, &contract_address, peer_id)
                .await
                .unwrap();

        mock.assert();
        assert!(commitment_id.is_none());
    }

    #[tokio::test]
    async fn test_get_current_commitment_id_some() {
        let peer_id_hex = "0x6497db93b32e4cdd979ada46a23249f444da1efb186cd74b9666bd03f710028b";
        let peer_id = peer_id_from_hex(peer_id_hex).unwrap();
        let contract_address = "0x3Aa5ebB10DC797CAC828524e59A333d0A371443c";

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
        let commitment_id =
            ChainConnector::get_current_commitment_id(&url, &contract_address, peer_id)
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
        let contract_address = "0x3Aa5ebB10DC797CAC828524e59A333d0A371443c";

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
        let commitment = ChainConnector::get_commitment(&url, &contract_address, commitment_id)
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
        let contract_address = "0x3Aa5ebB10DC797CAC828524e59A333d0A371443c";

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
            .with_body(expected_response)
            .create();
        let commitment_id = CommitmentId(hex::decode(&commitment_id[2..]).unwrap());
        let status = ChainConnector::get_commitment_status(
            &url,
            contract_address.to_string(),
            commitment_id,
        )
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
        let url = "http://localhost:8545".to_string();
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
        let config = server_config::ChainListenerConfig {
            http_endpoint: url,
            ws_endpoint: "".to_string(),
            cc_contract_address: "0x3Aa5ebB10DC797CAC828524e59A333d0A371443c".to_string(),
            core_contract_address: "0x0B306BF915C4d645ff596e518fAf3F9669b97016".to_string(),
            market_contract_address: "0x68B1D87F95878fE05B998F19b66F4baba5De1aed".to_string(),
        };
        let init_params = ChainConnector::batch_init_request(&config).await.unwrap();

        mock.assert();
        assert_eq!(
            hex::encode(&init_params.difficulty),
            "76889c92f61b9c5df216e048df56eb8f4eb02f172ab0d5b04edb9190ab9c9eec"
        );
        assert_eq!(init_params.init_timestamp, 1707760129.into());
        assert_eq!(
            hex::encode(init_params.global_nonce),
            "0000000000000000000000000000000000000000000000000000000000000005"
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
}
