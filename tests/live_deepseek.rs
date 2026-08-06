use deepseek_websearch_mcp::{config::Config, deepseek::DeepSeekSearchClient};

#[tokio::test]
#[ignore = "requires DEEPSEEK_API_KEY and makes a billed DeepSeek request"]
async fn real_deepseek_search_returns_a_summary() {
    let config = Config::from_env().expect("valid DeepSeek configuration");
    assert!(
        config.api_key.is_some(),
        "set DEEPSEEK_API_KEY before running ignored tests"
    );

    let client = DeepSeekSearchClient::new(config).expect("create client");
    let result = client
        .search("DeepSeek 官方最近发布了什么产品或公告？")
        .await
        .expect("DeepSeek web search should return a summary");

    assert!(!result.summary.trim().is_empty());
    eprintln!(
        "live probe completed: summary_bytes={}, public_sources={}",
        result.summary.len(),
        result.sources.len()
    );
}
