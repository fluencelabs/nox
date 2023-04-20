use console_subscriber::ConsoleLayer;
use opentelemetry::sdk::Resource;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use tracing::level_filters::LevelFilter;
use tracing::Subscriber;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

const TOKIO_CONSOLE_VAR: &str = "FLUENCE_TOKIO_CONSOLE_ENABLED";
const TRACING_TRACER_VAR: &str = "FLUENCE_TRACING_TRACER";

pub fn log_layer<S>() -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let log_layer = tracing_logfmt::layer().with_filter(
        tracing_subscriber::EnvFilter::builder()
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
            .add_directive("avm_server::runner=error".parse().unwrap()),
    );
    log_layer
}

pub fn tokio_console_layer<S>() -> Option<impl Layer<S>>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    std::env::var(TOKIO_CONSOLE_VAR)
        .map(|_| ConsoleLayer::builder().with_default_env().spawn())
        .ok()
}

pub fn tracing_layer<S>() -> eyre::Result<Option<impl Layer<S>>>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let tracing_layer = match std::env::var(TRACING_TRACER_VAR)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "otlp" => {
            log::debug!("Setup the OTLP tracer");
            let resource = Resource::new(vec![KeyValue::new("service.name", "rust-peer")]);

            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_env())
                .with_trace_config(opentelemetry::sdk::trace::config().with_resource(resource))
                .install_batch(opentelemetry::runtime::Tokio)?;

            let tracing_layer = tracing_opentelemetry::layer::<S>().with_tracer(tracer);
            Some(tracing_layer)
        }
        _ => {
            log::debug!("No tracer selected");
            None
        }
    };
    Ok(tracing_layer)
}
