use crate::Versions;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use libp2p::PeerId;
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn handle_metrics(State(state): State<RouteState>) -> Response<Body> {
    let mut buf = String::new();
    let registry = state
        .0
        .registry
        .as_ref()
        .expect("Registry is not initialized");
    encode(&mut buf, registry).unwrap(); //TODO: fix unwrap

    let body = Body::from(buf);
    Response::builder()
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(body)
        .unwrap() //TODO: fix unwrap
}

async fn handle_peer_id(State(state): State<RouteState>) -> Response {
    let peer_id = state.0.peer_id;
    Json(json!({
        "peer_id": peer_id.to_string(),
    }))
    .into_response()
}

async fn handle_versions(State(state): State<RouteState>) -> Response {
    let versions = &state.0.versions;
    Json(json!({
        "node": versions.node_version,
        "avm": versions.avm_version,
        "spell": versions.spell_version,
        "aqua_ipfs": versions.system_service.aqua_ipfs_version,
        "trust_graph": versions.system_service.trust_graph_version,
        "registry": versions.system_service.registry_version,
        "decider": versions.system_service.decider_version,
    }))
    .into_response()
}

#[derive(Debug, Clone)]
struct RouteState(Arc<Inner>);

#[derive(Debug)]
struct Inner {
    registry: Option<Registry>,
    peer_id: PeerId,
    versions: Versions,
}

pub async fn start_http_endpoint(
    listen_addr: SocketAddr,
    registry: Option<Registry>,
    peer_id: PeerId,
    versions: Versions,
) {
    let state = RouteState(Arc::new(Inner {
        registry,
        peer_id,
        versions,
    }));
    let app: Router = Router::new()
        .route("/metrics", get(handle_metrics))
        .route("/peer_id", get(handle_peer_id))
        .route("/versions", get(handle_versions))
        .fallback(handler_404)
        .with_state(state);

    axum::Server::bind(&listen_addr)
        .serve(app.into_make_service())
        .await
        .expect("Could not make http endpoint")
}
