use timeline_plugin_text_server::TextPlugin;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .finish(),
    );
    timeline_plugin_sdk::launch::<TextPlugin>("config.toml").await
}
