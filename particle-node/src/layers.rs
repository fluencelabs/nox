use console_subscriber::ConsoleLayer;
use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use server_config::{LogConfig, LogFormat, TracingConfig};
use tracing::level_filters::LevelFilter;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

const TOKIO_CONSOLE_VAR: &str = "FLUENCE_TOKIO_CONSOLE_ENABLED";

pub fn log_layer<S>(log_config: &Option<LogConfig>) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy()
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
        .add_directive("avm_server::runner=error".parse().unwrap());

    let log_format = log_config
        .as_ref()
        .map(|c| &c.format)
        .unwrap_or(&LogFormat::Default);

    let layer = match log_format {
        LogFormat::Logfmt => tracing_logfmt::builder()
            .with_target(false)
            .with_span_path(false)
            .with_span_name(false)
            .layer()
            .with_filter(env_filter)
            .boxed(),
        LogFormat::Default => tracing_subscriber::fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_filter(env_filter)
            .boxed(),
    };

    layer
}

pub fn tokio_console_layer<S>() -> Option<impl Layer<S>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    std::env::var(TOKIO_CONSOLE_VAR)
        .map(|_| ConsoleLayer::builder().with_default_env().spawn())
        .ok()
}

pub fn tracing_layer<S>(
    tracing_config: &Option<TracingConfig>,
) -> eyre::Result<Option<impl Layer<S>>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let tracing_config = tracing_config.as_ref().unwrap_or(&TracingConfig::Disabled);
    let tracing_layer = match tracing_config {
        TracingConfig::Disabled => None,
        TracingConfig::Otlp { endpoint } => {
            let resource = Resource::new(vec![KeyValue::new("service.name", "rust-peer")]);

            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(endpoint),
                )
                .with_trace_config(opentelemetry::sdk::trace::config().with_resource(resource))
                .install_batch(opentelemetry::runtime::TokioCurrentThread)?;

            let tracing_layer = tracing_opentelemetry::layer::<S>().with_tracer(tracer);
            Some(tracing_layer)
        }
    };

    Ok(tracing_layer)
}
