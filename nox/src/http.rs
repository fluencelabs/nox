use crate::Versions;
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::response::ErrorResponse;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use health::{HealthCheckRegistry, HealthStatus};
use libp2p::PeerId;
use prometheus_client::encoding::text::encode;
use prometheus_client::registry::Registry;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Notify;

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn handle_metrics(State(state): State<RouteState>) -> axum::response::Result<Response<Body>> {
    let mut buf = String::new();
    let registry = state
        .0
        .metric_registry
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "nothing to see here"))?;
    encode(&mut buf, registry).map_err(|e| {
        tracing::warn!("Metrics encode error: {}", e);
        ErrorResponse::from(StatusCode::INTERNAL_SERVER_ERROR)
    })?;

    let body = Body::from(buf);
    Response::builder()
        .header(
            CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(body)
        .map_err(|e| {
            tracing::warn!("Could not create metric response: {}", e);
            ErrorResponse::from(StatusCode::INTERNAL_SERVER_ERROR)
        })
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

async fn handle_health(State(state): State<RouteState>) -> axum::response::Result<Response> {
    fn make_json(keys: Vec<&'static str>, status: &str) -> Vec<Value> {
        keys.into_iter().map(|k| json!({k: status})).collect()
    }

    let registry = state
        .0
        .health_registry
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "nothing to see here"))?;
    let result = match registry.status() {
        HealthStatus::Ok(keys) => (StatusCode::OK, Json(make_json(keys, "Ok"))).into_response(),
        HealthStatus::Warning(ok, fail) => {
            let mut result = make_json(ok, "Ok");
            let mut fail = make_json(fail, "Fail");
            result.append(&mut fail);
            (StatusCode::TOO_MANY_REQUESTS, Json(result)).into_response()
        }
        HealthStatus::Fail(keys) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(make_json(keys, "Fail")),
        )
            .into_response(),
    };
    Ok(result)
}

#[derive(Clone)]
struct RouteState(Arc<Inner>);

struct Inner {
    metric_registry: Option<Registry>,
    health_registry: Option<HealthCheckRegistry>,
    peer_id: PeerId,
    versions: Versions,
}

pub async fn start_http_endpoint(
    listen_addr: SocketAddr,
    metric_registry: Option<Registry>,
    health_registry: Option<HealthCheckRegistry>,
    peer_id: PeerId,
    versions: Versions,
    notify: Arc<Notify>,
) {
    let state = RouteState(Arc::new(Inner {
        metric_registry,
        health_registry,
        peer_id,
        versions,
    }));
    let app: Router = Router::new()
        .route("/metrics", get(handle_metrics))
        .route("/peer_id", get(handle_peer_id))
        .route("/versions", get(handle_versions))
        .route("/health", get(handle_health))
        .fallback(handler_404)
        .with_state(state);

    let server = axum::Server::bind(&listen_addr).serve(app.into_make_service());
    notify.notify_one();
    server.await.expect("Could not make http endpoint")
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use std::net::{SocketAddr, TcpListener};

    fn get_free_tcp_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind to port 0");
        let socket_addr = listener
            .local_addr()
            .expect("Failed to retrieve local address");
        let port = socket_addr.port();
        drop(listener);
        port
    }

    fn test_versions() -> Versions {
        Versions {
            node_version: "node_test_version".to_string(),
            avm_version: "avm_test_version".to_string(),
            spell_version: "spell_test_version".to_string(),
            system_service: system_services::Versions {
                aqua_ipfs_version: "aqua_ipfs_test_version",
                trust_graph_version: "trust_graph_test_version",
                registry_version: "registry_test_version",
                decider_version: "decider_test_version",
            },
        }
    }

    #[tokio::test]
    async fn test_version_route() {
        // Create a test server
        let port = get_free_tcp_port();
        let addr = format!("127.0.0.1:{port}").parse::<SocketAddr>().unwrap();

        let notify = Arc::new(Notify::new());
        let cloned_notify = notify.clone();
        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                None,
                None,
                PeerId::random(),
                test_versions(),
                cloned_notify,
            )
            .await;
        });

        notify.notified().await;

        let client = hyper::Client::new();

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://{}/versions", addr))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], br#"{"node":"node_test_version","avm":"avm_test_version","spell":"spell_test_version","aqua_ipfs":"aqua_ipfs_test_version","trust_graph":"trust_graph_test_version","registry":"registry_test_version","decider":"decider_test_version"}"#);
    }

    #[tokio::test]
    async fn test_peer_id_route() {
        // Create a test server
        let port = get_free_tcp_port();
        let addr = format!("127.0.0.1:{port}").parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let notify = Arc::new(Notify::new());
        let cloned_notify = notify.clone();
        tokio::spawn(async move {
            start_http_endpoint(addr, None, None, peer_id, test_versions(), cloned_notify).await;
        });

        notify.notified().await;

        let client = hyper::Client::new();

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://{}/peer_id", addr))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            &body[..],
            format!(r#"{{"peer_id":"{}"}}"#, peer_id).as_bytes()
        );
    }
}
