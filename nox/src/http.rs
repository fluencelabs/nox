/*
 * Copyright 2024 Fluence DAO
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

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
use server_config::ResolvedConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::oneshot;

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "No such endpoint")
}

async fn handle_metrics(State(state): State<RouteState>) -> axum::response::Result<Response<Body>> {
    let mut buf = String::new();
    let registry = state
        .0
        .metric_registry
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "No such endpoint"))?;
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

/// Health check endpoint follows consul contract https://developer.hashicorp.com/consul/docs/services/usage/checks#http-checks
async fn handle_health(State(state): State<RouteState>) -> axum::response::Result<Response> {
    fn make_json(keys: Vec<&'static str>, status: &str) -> Vec<Value> {
        keys.into_iter().map(|k| json!({k: status})).collect()
    }

    let registry = state
        .0
        .health_registry
        .as_ref()
        .ok_or((StatusCode::NOT_FOUND, "No such endpoint"))?;
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

async fn handle_config(State(state): State<RouteState>) -> axum::response::Result<Response> {
    let toml = toml::to_string_pretty(&state.0.nox_config);
    match toml {
        Ok(toml) => Ok((StatusCode::OK, toml).into_response()),
        Err(error) => {
            tracing::warn!(error = error.to_string(), "Could not serialize config");
            Err(StatusCode::INTERNAL_SERVER_ERROR.into())
        }
    }
}

#[derive(Clone)]
struct RouteState(Arc<Inner>);

struct Inner {
    peer_id: PeerId,
    versions: Versions,
    metric_registry: Option<Registry>,
    health_registry: Option<HealthCheckRegistry>,
    nox_config: Option<ResolvedConfig>,
}
#[derive(Debug)]
pub struct StartedHttp {
    pub listen_addr: SocketAddr,
}

#[derive(Default)]
pub struct HttpEndpointData {
    metrics_registry: Option<Registry>,
    health_registry: Option<HealthCheckRegistry>,
    nox_config: Option<ResolvedConfig>,
}

impl HttpEndpointData {
    pub fn new(
        metrics_registry: Option<Registry>,
        health_registry: Option<HealthCheckRegistry>,
        nox_config: Option<ResolvedConfig>,
    ) -> Self {
        Self {
            metrics_registry,
            health_registry,
            nox_config,
        }
    }
}

pub async fn start_http_endpoint(
    listen_addr: SocketAddr,
    peer_id: PeerId,
    versions: Versions,
    http_endpoint_data: HttpEndpointData,
    notify: oneshot::Sender<StartedHttp>,
) -> eyre::Result<()> {
    let state = RouteState(Arc::new(Inner {
        peer_id,
        versions,
        metric_registry: http_endpoint_data.metrics_registry,
        health_registry: http_endpoint_data.health_registry,
        nox_config: http_endpoint_data.nox_config,
    }));
    let app: Router = Router::new()
        .route("/metrics", get(handle_metrics))
        .route("/peer_id", get(handle_peer_id))
        .route("/versions", get(handle_versions))
        .route("/health", get(handle_health))
        .route("/config", get(handle_config))
        .fallback(handler_404)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(listen_addr).await?;
    let local_addr = listener.local_addr()?;
    let server = axum::serve(listener, app.into_make_service());
    notify
        .send(StartedHttp {
            listen_addr: local_addr,
        })
        .expect("Could not send http info");
    server.await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eyre::Context;
    use health::HealthCheck;
    use reqwest::StatusCode;
    use server_config::UnresolvedConfig;
    use std::net::SocketAddr;
    use std::path::Path;

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
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();

        let (notify_sender, notify_receiver) = oneshot::channel();
        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                PeerId::random(),
                test_versions(),
                HttpEndpointData::default(),
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/versions", http_info.listen_addr))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], br#"{"node":"node_test_version","avm":"avm_test_version","spell":"spell_test_version","aqua_ipfs":"aqua_ipfs_test_version","trust_graph":"trust_graph_test_version","registry":"registry_test_version","decider":"decider_test_version"}"#);
    }

    #[tokio::test]
    async fn test_peer_id_route() {
        // Create a test server
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let (notify_sender, notify_receiver) = oneshot::channel();
        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                peer_id,
                test_versions(),
                HttpEndpointData::default(),
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/peer_id", http_info.listen_addr))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            &body[..],
            format!(r#"{{"peer_id":"{}"}}"#, peer_id).as_bytes()
        );
    }

    #[tokio::test]
    async fn test_health_route_empty_registry() {
        // Create a test server
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let (notify_sender, notify_receiver) = oneshot::channel();
        let health_registry = HealthCheckRegistry::new();
        let endpoint_config = HttpEndpointData {
            metrics_registry: None,
            health_registry: Some(health_registry),
            nox_config: None,
        };

        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                peer_id,
                test_versions(),
                endpoint_config,
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/health", http_info.listen_addr))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], (r#"[]"#).as_bytes());
    }

    #[tokio::test]
    async fn test_health_route_success_checks() {
        // Create a test server
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let (notify_sender, notify_receiver) = oneshot::channel();
        let mut health_registry = HealthCheckRegistry::new();
        struct SuccessHealthCheck {}
        impl HealthCheck for SuccessHealthCheck {
            fn status(&self) -> eyre::Result<()> {
                Ok(())
            }
        }
        let success_check = SuccessHealthCheck {};
        health_registry.register("test_check", success_check);

        let endpoint_config = HttpEndpointData {
            metrics_registry: None,
            health_registry: Some(health_registry),
            nox_config: None,
        };
        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                peer_id,
                test_versions(),
                endpoint_config,
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/health", http_info.listen_addr))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], (r#"[{"test_check":"Ok"}]"#).as_bytes());
    }

    #[tokio::test]
    async fn test_health_route_warn_checks() {
        // Create a test server
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let (notify_sender, notify_receiver) = oneshot::channel();
        let mut health_registry = HealthCheckRegistry::new();
        struct SuccessHealthCheck {}
        impl HealthCheck for SuccessHealthCheck {
            fn status(&self) -> eyre::Result<()> {
                Ok(())
            }
        }
        let success_check = SuccessHealthCheck {};
        struct FailHealthCheck {}
        impl HealthCheck for FailHealthCheck {
            fn status(&self) -> eyre::Result<()> {
                Err(eyre::eyre!("Failed"))
            }
        }
        let fail_check = FailHealthCheck {};
        health_registry.register("test_check", success_check);
        health_registry.register("test_check_2", fail_check);
        let endpoint_config = HttpEndpointData {
            metrics_registry: None,
            health_registry: Some(health_registry),
            nox_config: None,
        };
        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                peer_id,
                test_versions(),
                endpoint_config,
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/health", http_info.listen_addr))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            &body[..],
            (r#"[{"test_check":"Ok"},{"test_check_2":"Fail"}]"#).as_bytes()
        );
    }

    #[tokio::test]
    async fn test_health_route_fail_checks() {
        // Create a test server
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let (notify_sender, notify_receiver) = oneshot::channel();
        let mut health_registry = HealthCheckRegistry::new();
        struct FailHealthCheck {}
        impl HealthCheck for FailHealthCheck {
            fn status(&self) -> eyre::Result<()> {
                Err(eyre::eyre!("Failed"))
            }
        }
        let fail_check = FailHealthCheck {};
        health_registry.register("test_check", fail_check);
        let endpoint_config = HttpEndpointData {
            metrics_registry: None,
            health_registry: Some(health_registry),
            nox_config: None,
        };

        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                peer_id,
                test_versions(),
                endpoint_config,
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/health", http_info.listen_addr))
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body = response.bytes().await.unwrap();
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(&body[..], (r#"[{"test_check":"Fail"}]"#).as_bytes());
    }

    #[tokio::test]
    async fn test_config_endpoint() {
        let tmp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let tmp_path = tmp_dir.path();
        let resolved_config = get_config(tmp_path).await;

        // Create a test server
        let addr = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let peer_id = PeerId::random();

        let (notify_sender, notify_receiver) = oneshot::channel();

        let endpoint_config = HttpEndpointData {
            metrics_registry: None,
            health_registry: None,
            nox_config: Some(resolved_config),
        };

        tokio::spawn(async move {
            start_http_endpoint(
                addr,
                peer_id,
                test_versions(),
                endpoint_config,
                notify_sender,
            )
            .await
            .unwrap();
        });

        let http_info = notify_receiver.await.unwrap();

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/config", http_info.listen_addr))
            .send()
            .await
            .unwrap();

        let status = response.status();
        let body = response.bytes().await.unwrap();
        let expected_config = tokio::fs::read("./tests/http_expected_config.toml")
            .await
            .wrap_err("read test data")
            .unwrap();

        let expected_config = String::from_utf8(expected_config)
            .wrap_err("decode test data")
            .unwrap();

        let base_dir = tmp_path.canonicalize().unwrap().display().to_string();

        let expected_config = expected_config.replace("{base_dir}", &base_dir);

        let result = std::str::from_utf8(&body[..])
            .wrap_err("decode response data")
            .unwrap();

        assert_eq!(status, StatusCode::OK);
        assert_eq!(result, expected_config);
    }

    async fn get_config(path: &Path) -> ResolvedConfig {
        let unresolved_config = tokio::fs::read("./tests/http_test_config.toml")
            .await
            .wrap_err("read test data")
            .unwrap();

        let unresolved_config = String::from_utf8(unresolved_config)
            .wrap_err("decode test data")
            .unwrap();

        let unresolved_config =
            unresolved_config.replace("{base_dir}", &path.display().to_string());

        let unresolved_config: UnresolvedConfig = toml::de::from_str(unresolved_config.as_str())
            .wrap_err("parse config")
            .unwrap();

        unresolved_config
            .resolve()
            .wrap_err("resolve config")
            .unwrap()
    }
}
