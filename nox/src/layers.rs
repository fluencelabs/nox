use std::str::FromStr;

use libp2p::PeerId;
use log_format::Format;
use opentelemetry::trace::TracerProvider;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::Sampler;
use opentelemetry_sdk::Resource;
use server_config::TracingConfig;
use tracing::level_filters::LevelFilter;
use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
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
        //TODO: remove after debug
        .add_directive("run-console=info".parse().unwrap())
        .add_directive("expired=info".parse().unwrap())
    
    
}

pub fn log_layer<S>() -> (impl Layer<S>, WorkerGuard)
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let log_format = std::env::var("FLUENCE_LOG_FORMAT").unwrap_or_default();
    let log_format = LogFormat::from_str(log_format.as_str()).unwrap_or(LogFormat::Default);

    let log_display_span_list = std::env::var("FLUENCE_LOG_DISPLAY_SPAN_LIST").unwrap_or_default();
    let log_display_span_list = log_display_span_list.trim().parse().unwrap_or_default();

    let (non_blocking, guard) = tracing_appender::non_blocking(std::io::stdout());

    let layer = match log_format {
        LogFormat::Logfmt => tracing_logfmt::builder()
            .with_target(true)
            .with_span_path(false)
            .with_span_name(false)
            .layer()
            .with_writer(non_blocking)
            .boxed(),
        LogFormat::Default => {
            let format = Format::default().with_display_span_list(log_display_span_list);
            tracing_subscriber::fmt::layer()
                .event_format(format)
                .with_writer(non_blocking)
                .boxed()
        }
    };
    (layer, guard)
}

#[derive(Clone, Debug, PartialEq)]
pub enum LogFormat {
    Logfmt,
    Default,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "logfmt" => Ok(LogFormat::Logfmt),
            "default" => Ok(LogFormat::Default),
            _ => Err("Unsupported log format".to_string()),
        }
    }
}

pub fn tracing_layer<S>(
    tracing_config: &TracingConfig,
    peer_id: PeerId,
    version: &str,
) -> eyre::Result<Option<impl Layer<S>>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
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
                        .with_endpoint(endpoint.as_str()),
                )
                .with_trace_config(config)
                .install_batch(opentelemetry_sdk::runtime::Tokio)?;

            let tracing_layer = tracing_opentelemetry::layer::<S>().with_tracer(tracer);

            global::set_text_map_propagator(TraceContextPropagator::new());
            global::set_error_handler(move |err| {
                tracing::warn!("OpenTelemetry trace error occurred. {}", err)
            })?;

            Some(tracing_layer)
        }
    };

    Ok(tracing_layer)
}
