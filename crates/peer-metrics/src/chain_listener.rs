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

use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::exemplar::CounterWithExemplar;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

use crate::{execution_time_buckets, register};

#[derive(EncodeLabelSet, Hash, Clone, Eq, PartialEq, Debug)]
struct TxLabel {
    tx_hash: String,
}

#[derive(EncodeLabelSet, Hash, Clone, Eq, PartialEq, Debug)]
struct CommitmentLabel {
    commitment_id: String,
}

#[derive(Clone)]
pub struct ChainListenerMetrics {
    // how many request Nox sends to ccp
    ccp_requests_total: Counter,
    // how many success replies Nox receives from ccp
    // an error is either error from ccp or connection errors
    ccp_replies_total: Counter,
    // how long we wait for a reply from ccp
    ccp_request_duration_msec: Histogram,
    // how many proofs we submitted
    ccp_proofs_submitted: Counter,
    // how many proofs we failed to submit
    ccp_proofs_submit_failed: Counter,
    // how many proofs transaction are ok
    ccp_proofs_tx_success: Counter,
    // how many proofs transaction are failed
    ccp_proofs_tx_failed: CounterWithExemplar<TxLabel>,
    // max amount of proofs are allowed
    ccp_proofs_per_epoch_allowed_max: Gauge,
    // min amount of proofs are allowed
    ccp_proofs_per_epoch_allowed_min: Gauge,

    // How many blocks we have received from the newHead subscription
    blocks_seen: Counter,
    last_seen_block: Gauge,
    // How many block we manage to process while processing the block
    blocks_processed: Counter,
    // The number of the latest block
    last_process_block: Gauge,

    // Current commitment status
    current_commitment_status: Gauge,
    // Current commitment id
    current_commitment: Family<CommitmentLabel, Gauge>,

    // CUs metrics
    cus_total: Gauge,
    cus_in_deals: Gauge,

    // Epoch Metrics
    current_epoch: Gauge,
    current_epoch_start_timestamp_sec: Gauge,
    current_epoch_duration_sec: Gauge,
}

impl ChainListenerMetrics {
    pub fn new(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("chain_listener");

        let ccp_requests_total = register(
            sub_registry,
            Counter::default(),
            "ccp_requests_total",
            "Total number of requests sent to ccp",
        );

        let ccp_replies_total = register(
            sub_registry,
            Counter::default(),
            "ccp_replies_total",
            "Total number of successful replies from ccp",
        );

        let ccp_request_duration_msec = register(
            sub_registry,
            Histogram::new(execution_time_buckets()),
            "ccp_request_duration",
            "Duration of ccp requests",
        );

        let ccp_proofs_submitted = register(
            sub_registry,
            Counter::default(),
            "cpp_proofs_submitted",
            "Total number of proofs submitted to ccp",
        );

        let ccp_proofs_submit_failed = register(
            sub_registry,
            Counter::default(),
            "cpp_proofs_submit_failed",
            "Total number of proofs we failed to submit to ccp",
        );

        let ccp_proofs_tx_success = register(
            sub_registry,
            Counter::default(),
            "ccp_proofs_tx_success",
            "Total number of successfully processed proofs (transaction is ok)",
        );

        let ccp_proofs_tx_failed = register(
            sub_registry,
            CounterWithExemplar::default(),
            "ccp_proofs_tx_failed",
            "Total number of failed proofs (transaction isn't ok)",
        );

        let blocks_seen = register(
            sub_registry,
            Counter::default(),
            "blocks_seen",
            "Total number of blocks seen from the newHead subscription",
        );

        let blocks_processed = register(
            sub_registry,
            Counter::default(),
            "blocks_processed",
            "Total number of blocks processed",
        );

        let last_seen_block = register(
            sub_registry,
            Gauge::default(),
            "last_seen_block",
            "Last block seen from the newHead subscription",
        );
        let last_process_block = register(
            sub_registry,
            Gauge::default(),
            "last_process_block",
            "Last processed block from the newHead subscription",
        );

        let current_commitment_status = register(
            sub_registry,
            Gauge::default(),
            "current_commitment_status",
            "Current commitment status",
        );

        let current_commitment = register(
            sub_registry,
            Family::default(),
            "current_commitment",
            "Current commitment",
        );

        let cus_total = register(
            sub_registry,
            Gauge::default(),
            "cus_total",
            "Total number of CUs",
        );

        let cus_in_deals = register(
            sub_registry,
            Gauge::default(),
            "cus_in_deals",
            "Total number of CUs in deals",
        );

        let current_epoch = register(
            sub_registry,
            Gauge::default(),
            "current_epoch",
            "Current epoch",
        );

        let current_epoch_start_timestamp_sec = register(
            sub_registry,
            Gauge::default(),
            "current_epoch_start_timestamp_sec",
            "Current epoch start timestamp",
        );

        let current_epoch_duration_sec = register(
            sub_registry,
            Gauge::default(),
            "current_epoch_duration_sec",
            "Current epoch duration",
        );

        let ccp_proofs_per_epoch_allowed_max = register(
            sub_registry,
            Gauge::default(),
            "ccp_proofs_per_epoch_allowed_max",
            "Max amount of proofs are allowed per epoch",
        );

        let ccp_proofs_per_epoch_allowed_min = register(
            sub_registry,
            Gauge::default(),
            "ccp_proofs_per_epoch_allowed_min",
            "Min amount of proofs are allowed per epoch",
        );

        Self {
            ccp_requests_total,
            ccp_replies_total,
            ccp_request_duration_msec,
            ccp_proofs_submitted,
            ccp_proofs_submit_failed,
            ccp_proofs_tx_success,
            ccp_proofs_tx_failed,
            ccp_proofs_per_epoch_allowed_max,
            ccp_proofs_per_epoch_allowed_min,
            blocks_seen,
            last_seen_block,
            blocks_processed,
            last_process_block,
            cus_total,
            cus_in_deals,
            current_commitment_status,
            current_commitment,
            current_epoch,
            current_epoch_start_timestamp_sec,
            current_epoch_duration_sec,
        }
    }

    pub fn observe_ccp_request(&self) {
        self.ccp_requests_total.inc();
    }

    pub fn observe_ccp_reply(&self, duration: f64) {
        self.ccp_replies_total.inc();
        self.ccp_request_duration_msec.observe(duration);
    }

    pub fn observe_proof_failed(&self) {
        self.ccp_proofs_submit_failed.inc();
    }

    pub fn observe_proof_submitted(&self) {
        self.ccp_proofs_submitted.inc();
    }

    pub fn observe_proof_tx_success(&self) {
        self.ccp_proofs_tx_success.inc();
    }

    pub fn observe_proof_tx_failed(&self, tx_hash: String) {
        self.ccp_proofs_tx_failed
            .inc_by(1, Some(TxLabel { tx_hash }));
    }

    pub fn observe_new_block(&self, block_number: u64) {
        self.blocks_seen.inc();
        self.last_seen_block.set(block_number as i64);
    }

    pub fn observe_processed_block(&self, block_number: u64) {
        self.blocks_processed.inc();
        self.last_process_block.set(block_number as i64);
    }

    pub fn observe_commiment_status(&self, status: u64) {
        self.current_commitment_status.set(status as i64);
    }

    pub fn observe_new_commitment(&self, commitment_id: String) {
        self.current_commitment
            .get_or_create(&CommitmentLabel { commitment_id })
            .set(1);
    }

    pub fn observe_removed_commitment(&self, commitment_id: String) {
        self.current_commitment
            .get_or_create(&CommitmentLabel { commitment_id })
            .set(0);
    }

    pub fn observe_allowed_proofs_settings(&self, max_allowed: i64, min_allowed: i64) {
        self.ccp_proofs_per_epoch_allowed_max.set(max_allowed);
        self.ccp_proofs_per_epoch_allowed_min.set(min_allowed);
    }

    pub fn observe_current_epoch(&self, epoch: i64) {
        self.current_epoch.set(epoch);
    }

    pub fn observe_epoch_settings(&self, start_timestamp: i64, duration: i64) {
        self.current_epoch_start_timestamp_sec.set(start_timestamp);
        self.current_epoch_duration_sec.set(duration);
    }

    pub fn observe_cus_total(&self, n: i64) {
        self.cus_total.set(n);
    }

    pub fn observe_cus_in_deals(&self, n: i64) {
        self.cus_in_deals.set(n);
    }

    pub fn observe_cus_in_deals_added(&self, n: i64) {
        self.cus_in_deals.inc_by(n);
    }

    pub fn observe_cus_in_deals_removed(&self, n: i64) {
        self.cus_in_deals.dec_by(n);
    }
}
