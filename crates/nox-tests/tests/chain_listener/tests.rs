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

use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use crate::chain_listener::ccp_server::CCPServer;
use crate::chain_listener::chain_server::ChainServer;
use crate::chain_listener::types::{ChainState, CreateDealParams, PeerState, RegisterPeerParams};
use alloy_primitives::fixed_bytes;
use alloy_primitives::Address;
use alloy_primitives::FixedBytes;
use alloy_primitives::U256;
use ccp_shared::types::PhysicalCoreId;
use ccp_shared::types::CUID;
use chain_connector::CommitmentId;
use chain_connector::Deal::Status;
use chain_connector::Offer::ComputePeer;
use chain_connector::Offer::ComputeUnit;
use clarity::PrivateKey;
use created_swarm::make_swarms_with_cfg;
use created_swarm::CreatedSwarm;
use eyre::eyre;
use itertools::Itertools;
use log_utils::enable_logs;
use maplit::hashmap;
use server_config::ChainConfig;
use server_config::ChainListenerConfig;
use tempfile::TempDir;

struct ChainListenerTestEntities {
    ccp_server: CCPServer,
    chain_server: ChainServer,
    swarms: Vec<CreatedSwarm>,
    _events_dir: TempDir,
}

impl ChainListenerTestEntities {
    pub async fn new() -> eyre::Result<Self> {
        let chain_server = ChainServer::new(
            "0x73e491D86aDBc4504156Ad04254F2Ba7Eb5935cA".to_string(),
            ChainState::default(),
        )
        .await?;

        let ccp_server = CCPServer::new().await?;
        let events_dir = TempDir::new()?;
        let cc_events_dir = events_dir.path().to_path_buf();
        let chain_server_address = chain_server.address.clone();
        let ccp_server_address = ccp_server.address.clone();
        let swarms = make_swarms_with_cfg(1, move |mut cfg| {
            cfg.chain_config = Some(ChainConfig {
                http_endpoint: format!("http://{}", chain_server_address),
                diamond_contract_address: "0x73e491D86aDBc4504156Ad04254F2Ba7Eb5935cA".to_string(),
                network_id: 1,
                wallet_key: PrivateKey::from_str(
                    "0xfdc4ba94809c7930fe4676b7d845cbf8fa5c1beae8744d959530e5073004cf3f",
                )
                .unwrap(),
                default_base_fee: None,
                default_priority_fee: None,
            });
            cfg.chain_listener_config = Some(ChainListenerConfig {
                ws_endpoint: format!("ws://{}", chain_server_address),
                ccp_endpoint: Some(format!("http://{}", ccp_server_address)),
                proof_poll_period: Duration::from_secs(5),
                min_batch_count: 0,
                max_batch_count: 0,
                max_proof_batch_size: 0,
                epoch_end_window: Default::default(),
                ws_ping_period: Default::default(),
            });

            cfg.cc_events_dir = Some(cc_events_dir.clone());
            cfg.cpu_cores_count = Some(128);
            cfg.metrics_enabled = true;
            cfg
        })
        .await;
        Ok(Self {
            ccp_server,
            chain_server,
            swarms,
            _events_dir: events_dir,
        })
    }
}
const WAIT_BLOCK_POLLING_INTERVAL: Duration = Duration::from_millis(100);

async fn wait_block_process_on_nox(swarm: &CreatedSwarm, block_number: U256) -> eyre::Result<()> {
    let http_endpoint = swarm.http_listen_addr;
    let http_client = &reqwest::Client::new();
    loop {
        let response = http_client
            .get(format!("http://{}/metrics", http_endpoint))
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::OK {
            let body = response.text().await?;
            let lines: Vec<_> = body.lines().map(|s| Ok(s.to_owned())).collect();
            let metrics = prometheus_parse::Scrape::parse(lines.into_iter())?;
            let metric = metrics
                .samples
                .into_iter()
                .find(|el| el.metric.as_str() == "chain_listener_last_process_block")
                .map(|el| el.value);

            let metric = metric.ok_or(eyre!("metric not found"))?;
            let value = match metric {
                prometheus_parse::Value::Gauge(value) => U256::try_from(value)?,
                _ => return Err(eyre!("wrong metric format")),
            };

            if value == block_number {
                return Ok(());
            }
        }

        tokio::time::sleep(WAIT_BLOCK_POLLING_INTERVAL).await
    }
}

#[tokio::test]
async fn test_cc_activation_flow() {
    enable_logs();
    let test_entities = ChainListenerTestEntities::new().await.unwrap();

    let owner_address = Address::from(fixed_bytes!("9b5baa60fa4004a0f72b65416fc1dae6d3e88611"));
    let offer_id = fixed_bytes!("17376d336cc28febe907ebf32629d63717eb5d4b0e1bb0e3d5e7c72db6ad58e5");

    let commitment_id = CommitmentId::new(
        fixed_bytes!("58e279641b29d191462381628d407f2a5a08fa0620b9b12b45aecbbdb7aa0d20").0,
    )
    .unwrap();

    let compute_unit_id_1 =
        fixed_bytes!("ad13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95");

    let compute_unit_1 = ComputeUnit {
        id: compute_unit_id_1,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
        onchainWorkerId: FixedBytes::<32>::ZERO,
    };

    let compute_peer_1 = ComputePeer {
        offerId: offer_id,
        commitmentId: FixedBytes::<32>::ZERO,
        unitCount: U256::from(1),
        owner: owner_address,
    };

    test_entities
        .chain_server
        .register_peer(RegisterPeerParams {
            state: hashmap! {
               test_entities.swarms[0].peer_id => PeerState {
                    units: vec![compute_unit_1],
                    compute_peer: compute_peer_1
                },
            },
        })
        .await;

    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    // Create commitment with status wait delegation
    test_entities
        .chain_server
        .create_commitment(test_entities.swarms[0].peer_id, &commitment_id)
        .await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    let activation_epoch = test_entities
        .chain_server
        .activate_commitment(test_entities.swarms[0].peer_id, &commitment_id, 100)
        .await;
    // active commitment on epoch 3
    let block_number = test_entities
        .chain_server
        .move_to_epoch(activation_epoch)
        .await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    // assert CCP
    // Active Commitment Params are pushed to CCP Mock in-memory stack on New Active Commitment event, and then popped on get_or_wait_active_commitment_params call
    // This way we assert that CCP receives correct Active Commitment Params when Commitment is Activated
    let result = test_entities
        .ccp_server
        .get_or_wait_active_commitment_params()
        .await;

    assert_eq!(
        result.difficulty.to_string(),
        "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    assert_eq!(
        result.global_nonce.to_string(),
        "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a"
    );

    assert_allocation(
        result.cu_allocation,
        vec![(PhysicalCoreId::new(127), CUID::new(compute_unit_id_1.0))],
    );
}

#[tokio::test]
async fn test_deal_insufficient_funds_flow() {
    enable_logs();
    let test_entities = ChainListenerTestEntities::new().await.unwrap();

    let deal_address = Address::from(fixed_bytes!("9c5baa60fa4004a0f72b65416fc1dae6d3e88611"));
    let owner_address = Address::from(fixed_bytes!("8a5baa60fa4004a0f72b65416fc1dae6d3e88611"));
    let offer_id = fixed_bytes!("17376d336cc28febe907ebf32629d63717eb5d4b0e1bb0e3d5e7c72db6ad58e5");

    let commitment_id = CommitmentId::new(
        fixed_bytes!("58e279641b29d191462381628d407f2a5a08fa0620b9b12b45aecbbdb7aa0d20").0,
    )
    .unwrap();

    let compute_unit_id_1 =
        fixed_bytes!("ad13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce95");
    let cu_id_1 = CUID::new(compute_unit_id_1.0);
    let compute_unit_id_2 =
        fixed_bytes!("bd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce96");
    let cu_id_2 = CUID::new(compute_unit_id_2.0);
    let compute_unit_id_3 =
        fixed_bytes!("cd13defccc6aaee67e8df9acea926fb37d9057275d5300f7e20ec8f1f8f7ce97");
    let cu_id_3 = CUID::new(compute_unit_id_3.0);

    let compute_unit_1 = ComputeUnit {
        id: compute_unit_id_1,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
        onchainWorkerId: FixedBytes::<32>::ZERO,
    };

    let compute_unit_2 = ComputeUnit {
        id: compute_unit_id_2,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
        onchainWorkerId: FixedBytes::<32>::ZERO,
    };

    let compute_unit_3 = ComputeUnit {
        id: compute_unit_id_3,
        deal: Address::ZERO,
        startEpoch: U256::from(2),
        onchainWorkerId: FixedBytes::<32>::ZERO,
    };

    let compute_peer_1 = ComputePeer {
        offerId: offer_id,
        commitmentId: FixedBytes::<32>::ZERO,
        unitCount: U256::from(1),
        owner: owner_address,
    };

    test_entities
        .chain_server
        .register_peer(RegisterPeerParams {
            state: hashmap! {
               test_entities.swarms[0].peer_id => PeerState {
                    units: vec![compute_unit_1.clone(), compute_unit_2.clone(), compute_unit_3],
                    compute_peer: compute_peer_1
                },
            },
        })
        .await;

    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    // Create commitment with status wait delegation
    test_entities
        .chain_server
        .create_commitment(test_entities.swarms[0].peer_id, &commitment_id)
        .await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    let epoch = test_entities
        .chain_server
        .activate_commitment(test_entities.swarms[0].peer_id, &commitment_id, 100)
        .await;
    // active commitment on epoch 3
    test_entities.chain_server.move_to_epoch(epoch).await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    test_entities
        .chain_server
        .create_deal(CreateDealParams {
            deal_id: deal_address,
            peer_id: test_entities.swarms[0].peer_id,
            commitment_id,
            unit: compute_unit_1,
        })
        .await;
    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    test_entities
        .chain_server
        .change_deal_status(deal_address, Status::INSUFFICIENT_FUNDS)
        .await;

    let block_number = test_entities.chain_server.send_next_block().await;
    wait_block_process_on_nox(&test_entities.swarms[0], block_number)
        .await
        .unwrap();

    // assert CCP
    let result_1 = test_entities
        .ccp_server
        .get_or_wait_active_commitment_params()
        .await;

    let result_2 = test_entities
        .ccp_server
        .get_or_wait_active_commitment_params()
        .await;

    assert_eq!(
        result_1.difficulty.to_string(),
        "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    assert_eq!(
        result_2.difficulty.to_string(),
        "0001afffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    );
    assert_eq!(
        result_1.global_nonce.to_string(),
        "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a"
    );
    assert_eq!(
        result_2.global_nonce.to_string(),
        "4183468390b09d71644232a1d1ce670bc93897183d3f56c305fabfb16cab806a"
    );

    // The timeline is like that:
    // t0: result_2 => compute_unit_1 allocated to Deal
    // t1: Deal is detected as INSUFFICIENT_FUNDS
    // t2: result_1 => compute_unit_1 allocated to CCP
    //
    // p.s. order of result_1 and result_2 is reversed bc they are stored in a stack

    // Here we check that compute_unit_1 is allocated to CCP after status was detected as INSUFFICIENT_FUNDS
    assert_allocation(
        result_1.cu_allocation,
        vec![
            (PhysicalCoreId::new(125), cu_id_3),
            (PhysicalCoreId::new(126), cu_id_2),
            (PhysicalCoreId::new(127), cu_id_1),
        ],
    );

    // here we check that compute_unit_1 is not allocated to CCP after Deal was deployed to it
    assert_allocation(
        result_2.cu_allocation,
        vec![
            (PhysicalCoreId::new(125), cu_id_3),
            (PhysicalCoreId::new(126), cu_id_2),
        ],
    );
}

fn assert_allocation(
    allocation: HashMap<PhysicalCoreId, CUID>,
    expected: Vec<(PhysicalCoreId, CUID)>,
) {
    let allocation = allocation
        .into_iter()
        .map(|(k, v)| (k, v))
        .sorted_by(|(l, _), (r, _)| l.cmp(r));
    assert!(
        allocation.clone().eq(expected.clone()),
        "{}",
        format!("{:?} != {:?}", allocation, expected)
    );
}
