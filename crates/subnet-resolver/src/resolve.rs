use crate::error::{ChainDataError, ResolveSubnetError};
use crate::utils::{decode_hex, next_opt, parse_peer_id};
use ethabi::ParamType::{Address, Array, FixedBytes, Tuple, Uint};
use ethabi::{Function, ParamType, StateMutability, Token};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::http_client::HttpClientBuilder;
use jsonrpsee::rpc_params;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::runtime::Handle;

/// Parse data from chain. Accepts data with and without "0x" prefix.
pub fn parse_chain_data(data: &str) -> Result<Vec<Token>, ChainDataError> {
    if data.is_empty() {
        return Err(ChainDataError::Empty);
    }
    let data = decode_hex(data).map_err(ChainDataError::DecodeHex)?;
    let signature: ParamType = Array(Box::new(Tuple(vec![
        // bytes32 id
        FixedBytes(32),
        // bytes32 peerId
        FixedBytes(32),
        // bytes32 workerId
        FixedBytes(32),
        // address owner
        Address,
        // uint256 collateral
        Uint(256),
        // uint256 created
        Uint(256),
    ])));
    Ok(ethabi::decode(&[signature], &data)?)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Worker {
    pub pat_id: String,
    pub host_id: String,
    pub worker_id: Vec<String>,
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SubnetResolveResult {
    pub success: bool,
    pub workers: Vec<Worker>,
    pub error: Vec<String>,
}

fn decode_pats(data: String) -> Result<Vec<Worker>, ResolveSubnetError> {
    let tokens = parse_chain_data(&data)?;
    let tokens = tokens.into_iter().next().ok_or(ResolveSubnetError::Empty)?;
    let tokens = tokens
        .into_array()
        .ok_or(ChainDataError::InvalidParsedToken("response"))?;
    let mut result = vec![];
    for token in tokens {
        let tuple = token
            .into_tuple()
            .ok_or(ChainDataError::InvalidParsedToken("tuple"))?;
        let mut tuple = tuple.into_iter();

        let pat_id = next_opt(&mut tuple, "pat_id", Token::into_fixed_bytes)?;
        let pat_id = hex::encode(pat_id);

        let peer_id = next_opt(&mut tuple, "compute_peer_id", Token::into_fixed_bytes)?;
        let peer_id = parse_peer_id(peer_id)
            .map_err(|e| ResolveSubnetError::InvalidPeerId(e, "compute_peer_id"))?;
        let worker_id = next_opt(&mut tuple, "compute_worker_id", Token::into_fixed_bytes)?;
        // if all bytes are 0, then worker_id is considered empty
        let all_zeros = worker_id.iter().all(|b| *b == 0);
        let worker_id = if all_zeros {
            vec![]
        } else {
            let worker_id = parse_peer_id(worker_id)
                .map_err(|e| ResolveSubnetError::InvalidPeerId(e, "worker_id"))?;
            vec![worker_id.to_string()]
        };

        let pat = Worker {
            pat_id: format!("0x{}", pat_id),
            host_id: peer_id.to_string(),
            worker_id,
        };
        result.push(pat);
    }

    Ok(result)
}

pub fn validate_deal_id(deal_id: String) -> Result<String, ResolveSubnetError> {
    // 40 hex chars + 2 for "0x" prefix
    if deal_id.len() == 42 && deal_id.starts_with("0x") {
        Ok(deal_id)
    } else if deal_id.len() == 40 {
        Ok(format!("0x{}", deal_id))
    } else {
        Err(ResolveSubnetError::InvalidDealId(deal_id))
    }
}

pub fn resolve_subnet(deal_id: String, api_endpoint: &str) -> SubnetResolveResult {
    let res: Result<_, ResolveSubnetError> = try {
        let deal_id = validate_deal_id(deal_id)?;
        // Description of the `getComputeUnits` function from the `chain.workers` smart contract on chain
        #[allow(deprecated)]
        let input = Function {
            name: String::from("getComputeUnits"),
            inputs: vec![],
            outputs: vec![],
            constant: None,
            state_mutability: StateMutability::View,
        }
        .encode_input(&[])?;
        let input = format!("0x{}", hex::encode(input));
        let client = HttpClientBuilder::default().build(api_endpoint)?;
        let params = rpc_params![json!({ "data": input, "to": deal_id }), json!("latest")];
        let response: Result<String, _> = tokio::task::block_in_place(move || {
            Handle::current().block_on(async move { client.request("eth_call", params).await })
        });

        let pats = response?;

        decode_pats(pats)?
    };

    match res {
        Ok(workers) => SubnetResolveResult {
            success: true,
            workers,
            error: vec![],
        },
        Err(err) => SubnetResolveResult {
            success: false,
            workers: vec![],
            error: vec![format!("{}", err)],
        },
    }
}
