#[cfg(feature = "otel")]
use std::sync::OnceLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[cfg(feature = "otel")]
static OTEL_PROVIDER: OnceLock<opentelemetry_sdk::trace::SdkTracerProvider> = OnceLock::new();

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    #[cfg(feature = "otel")]
    if std::env::var_os("OTEL_EXPORTER_OTLP_ENDPOINT").is_some()
        || std::env::var_os("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").is_some()
    {
        use opentelemetry::trace::TracerProvider;
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .build();
        match exporter {
            Ok(exporter) => {
                let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
                    .with_batch_exporter(exporter)
                    .build();
                let tracer = provider.tracer("forge");
                let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
                let subscriber = tracing_subscriber::registry()
                    .with(filter)
                    .with(tracing_subscriber::fmt::layer())
                    .with(telemetry);
                if subscriber.try_init().is_ok() {
                    opentelemetry::global::set_tracer_provider(provider.clone());
                    let _ = OTEL_PROVIDER.set(provider);
                    return;
                }
            }
            Err(error) => eprintln!("could not initialize OTLP exporter: {error}"),
        }
    }
    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

/// Flushes OTLP telemetry before process shutdown.
#[cfg(feature = "otel")]
pub fn shutdown_otel() {
    if let Some(provider) = OTEL_PROVIDER.get() {
        if let Err(error) = provider.shutdown() {
            tracing::warn!(%error, "OTLP provider shutdown failed");
        }
    }
}

#[cfg(not(feature = "otel"))]
pub fn shutdown_otel() {}

pub fn record_request(operation: &str) {
    tracing::info!(operation, "forge.request");
}
