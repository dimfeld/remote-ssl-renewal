use tracing::subscriber::set_global_default;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_tree::HierarchicalLayer;

pub fn init_tracing() {
    LogTracer::builder()
        .ignore_crate("rustls")
        .with_max_level(log::LevelFilter::Debug)
        .init()
        .expect("Failed to create logger");

    let env_filter = EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    let tree = HierarchicalLayer::new(2)
        .with_targets(true)
        .with_bracketed_fields(true);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(tree)
        .with(ErrorLayer::default());

    set_global_default(subscriber).expect("Failed to set global subscriber");
}
