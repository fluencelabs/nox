use console_subscriber::ConsoleLayer;
use eyre::anyhow;
use libp2p::PeerId;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::Sampler;
use opentelemetry_sdk::Resource;
use server_config::{ConsoleConfig, LogConfig, LogFormat, TracingConfig};
use std::net::{SocketAddr, ToSocketAddrs};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use tracing::level_filters::LevelFilter;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

pub fn env_filter<S>() -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let rust_log = std::env::var("RUST_LOG")
        .unwrap_or_default()
        .replace(char::is_whitespace, "");

    tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy(rust_log)
        .add_directive("cranelift_codegen=off".parse().unwrap())
        .add_directive("walrus=off".parse().unwrap())
        .add_directive("polling=off".parse().unwrap())
        .add_directive("wasmer_wasi_fl=error".parse().unwrap())
        .add_directive("wasmer_interface_types_fl=error".parse().unwrap())
        .add_directive("wasmer_wasi=error".parse().unwrap())
        .add_directive("tokio_threadpool=error".parse().unwrap())
        .add_directive("tokio_reactor=error".parse().unwrap())
        .add_directive("mio=error".parse().unwrap())
        .add_directive("tokio_io=error".parse().unwrap())
        .add_directive("soketto=error".parse().unwrap())
        .add_directive("cranelift_codegen=error".parse().unwrap())
        .add_directive("tracing=error".parse().unwrap())
        .add_directive("avm_server::runner=error".parse().unwrap())
}
pub fn log_layer<S>(log_config: &Option<LogConfig>) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let log_format = log_config
        .as_ref()
        .map(|c| &c.format)
        .unwrap_or(&LogFormat::Default);

    let layer = match log_format {
        LogFormat::Logfmt => tracing_logfmt::builder()
            .with_target(true)
            .with_span_path(false)
            .with_span_name(false)
            .layer()
            .boxed(),
        LogFormat::Default => tracing_subscriber::fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .boxed(),
    };

    layer
}

pub fn tokio_console_layer<S>(
    console_config: &Option<ConsoleConfig>,
) -> eyre::Result<Option<impl Layer<S>>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let console = console_config.as_ref().unwrap_or(&ConsoleConfig::Disabled);

    let console_layer = match console {
        ConsoleConfig::Disabled => None,
        ConsoleConfig::Enabled { bind } => {
            let addr: SocketAddr = bind
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| anyhow!("tokio console could not resolve bind address"))?;
            Some(ConsoleLayer::builder().server_addr(addr).spawn())
        }
    };
    Ok(console_layer)
}

pub fn tracing_layer<S>(
    tracing_config: &Option<TracingConfig>,
    peer_id: PeerId,
    version: &str
) -> eyre::Result<Option<impl Layer<S>>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let tracing_config = tracing_config.as_ref().unwrap_or(&TracingConfig::Disabled);
    let tracing_layer = match tracing_config {
        TracingConfig::Disabled => None,
        TracingConfig::Stdout => {
            global::set_text_map_propagator(TraceContextPropagator::new());
            let exporter = opentelemetry_stdout::SpanExporter::default();
            let provider = opentelemetry_sdk::trace::TracerProvider::builder()
                .with_simple_exporter(exporter)
                .build();

            let tracer = provider.tracer("rust-peer");

            let tracing_layer = tracing_opentelemetry::layer::<S>().with_tracer(tracer);
            Some(tracing_layer)
        }
        TracingConfig::Otlp {
            endpoint,
            sample_ratio,
        } => {
            global::set_text_map_propagator(TraceContextPropagator::new());
            let resource = Resource::new(vec![
                KeyValue::new("service.name", "rust-peer"),
                KeyValue::new("service.version", version.to_string()),
                KeyValue::new("peer_id", peer_id.to_base58()),
            ]);

            let mut config = opentelemetry_sdk::trace::config().with_resource(resource);

            if let Some(ratio) = sample_ratio {
                config = config.with_sampler(Sampler::ParentBased(Box::new(
                    Sampler::TraceIdRatioBased(*ratio),
                )));
            }

            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(endpoint),
                )
                .with_trace_config(config)
                .install_batch(opentelemetry_sdk::runtime::TokioCurrentThread)?;

            let tracing_layer = tracing_opentelemetry::layer::<S>().with_tracer(tracer);
            Some(tracing_layer)
        }
    };

    Ok(tracing_layer)
}
