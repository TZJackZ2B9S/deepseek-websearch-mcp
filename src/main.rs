use deepseek_websearch_mcp::{
    config::Config, deepseek::DeepSeekSearchClient, server::WebSearchServer,
};
use rmcp::ServiceExt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let config = Config::from_env()?;
    let client = DeepSeekSearchClient::new(config)?;
    let service = WebSearchServer::new(client)
        .serve(rmcp::transport::stdio())
        .await?;

    service.waiting().await?;
    Ok(())
}
