use crate::{execution_time_buckets, register};
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::counter::Counter;
use prometheus_client::metrics::exemplar::CounterWithExemplar;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::metrics::histogram::Histogram;
use prometheus_client::registry::Registry;

#[derive(EncodeLabelSet, Hash, Clone, Eq, PartialEq, Debug)]
struct TxLabel {
    tx_hash: String,
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
    // How many blocks we have received from the newHead subscription
    blocks_seen: Counter,
    last_seen_block: Gauge,
    // How many block we manage to process while processing the block
    blocks_processed: Counter,
    last_process_block: Gauge,
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

        Self {
            ccp_requests_total,
            ccp_replies_total,
            ccp_request_duration_msec,
            ccp_proofs_submitted,
            ccp_proofs_submit_failed,
            ccp_proofs_tx_success,
            ccp_proofs_tx_failed,
            blocks_seen,
            last_seen_block,
            blocks_processed,
            last_process_block,
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

    pub fn observe_new_block(&self, block_number: i64) {
        self.blocks_seen.inc();
        self.last_seen_block.set(block_number);
    }

    pub fn observe_processed_block(&self, block_number: i64) {
        self.blocks_processed.inc();
        self.last_process_block.set(block_number);
    }
}
