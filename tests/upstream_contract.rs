use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    response::Response,
    routing::post,
    Router,
};
use deepseek_websearch_mcp::{config::ApiKey, deepseek::DeepSeekSearchClient};
use reqwest::Client;
use serde_json::{json, Value};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use url::Url;

struct MockServer {
    url: Url,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl MockServer {
    async fn start(status: StatusCode, body: Value) -> Self {
        let app = Router::new()
            .route("/anthropic/v1/messages", post(mock_response))
            .with_state(MockResponse { status, body });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, signal) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = signal.await;
                })
                .await
                .unwrap();
        });
        Self {
            url: Url::parse(&format!("http://{address}/anthropic/v1/messages")).unwrap(),
            shutdown: Some(shutdown),
            task,
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.abort();
    }
}

#[derive(Clone)]
struct MockResponse {
    status: StatusCode,
    body: Value,
}

async fn mock_response(
    State(response): State<MockResponse>,
    request: Request<Body>,
) -> Response<Body> {
    assert_eq!(
        request.headers().get("anthropic-version").unwrap(),
        "2023-06-01"
    );
    assert_eq!(
        request.headers().get(header::AUTHORIZATION).unwrap(),
        "Bearer test-key"
    );

    let request_body = axum::body::to_bytes(request.into_body(), 64 * 1024)
        .await
        .unwrap();
    let request_body: Value = serde_json::from_slice(&request_body).unwrap();
    assert_eq!(request_body["model"], "deepseek-v4-flash");
    assert_eq!(request_body["max_tokens"], 1200);
    assert_eq!(request_body["tools"][0]["type"], "web_search_20250305");
    assert_eq!(request_body["tools"][0]["name"], "web_search");
    assert!(request_body.pointer("/tools/0/max_uses").is_none());

    Response::builder()
        .status(response.status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(response.body.to_string()))
        .unwrap()
}

fn client(url: Url) -> DeepSeekSearchClient {
    DeepSeekSearchClient::with_parts(
        Some(ApiKey::new("test-key")),
        url,
        "deepseek-v4-flash",
        Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap(),
    )
}

#[tokio::test]
async fn converts_summary_and_public_sources() {
    let server = MockServer::start(
        StatusCode::OK,
        json!({
            "stop_reason": "end_turn",
            "content": [
                {"type":"server_tool_use","name":"web_search","input":{"query":"test"}},
                {"type":"web_search_tool_result","content":[
                    {"type":"web_search_result","title":"Official","url":"https://example.com","page_age":"today","encrypted_content":"opaque"},
                    {"type":"web_search_tool_result_error","error_code":"search_error"}
                ]},
                {"type":"text","text":"第一段"},
                {"type":"text","text":"第二段"}
            ]
        }),
    )
    .await;

    let result = client(server.url.clone()).search("test").await.unwrap();
    assert_eq!(result.summary, "第一段\n\n第二段");
    assert_eq!(result.sources.len(), 1);
    assert!(result.source_metadata_available);
    assert_eq!(result.sources[0].title, "Official");
    assert_eq!(result.sources[0].url, "https://example.com/");
}

#[tokio::test]
async fn discards_invalid_and_excess_source_metadata() {
    let long_title = "文".repeat(300);
    let sources: Vec<Value> = (0..11)
        .map(|index| {
            json!({
                "type": "web_search_result",
                "title": if index == 0 { long_title.clone() } else { format!("Source {index}") },
                "url": format!("https://example{index}.com")
            })
        })
        .collect();
    let server = MockServer::start(
        StatusCode::OK,
        json!({
            "content": [
                {"type": "web_search_tool_result", "content": [
                    {"type": "web_search_result", "title": "Insecure", "url": "http://example.com"},
                    {"type": "web_search_result", "title": "Script", "url": "javascript:alert(1)"},
                    {"type": "web_search_result", "title": "Credentials", "url": "https://user:password@example.com"},
                    {"type": "web_search_result", "title": "Relative", "url": "/relative"}
                ]},
                {"type": "web_search_tool_result", "content": sources},
                {"type": "text", "text": "summary"}
            ]
        }),
    )
    .await;

    let result = client(server.url.clone()).search("test").await.unwrap();
    assert_eq!(result.sources.len(), 10);
    assert!(result.source_metadata_available);
    assert!(result
        .sources
        .iter()
        .all(|source| source.url.starts_with("https://")));
    assert_eq!(result.sources[0].title.chars().count(), 256);
    assert!(result.sources[0].title.ends_with('…'));
}

#[tokio::test]
async fn does_not_follow_upstream_redirects() {
    let redirected_requests = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route("/anthropic/v1/messages", post(redirect_response))
        .route("/redirected", post(record_redirect))
        .with_state(redirected_requests.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (shutdown, signal) = oneshot::channel();
    let task = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = signal.await;
            })
            .await
            .unwrap();
    });

    let messages_url = Url::parse(&format!("http://{address}/anthropic/v1/messages")).unwrap();
    let result = client_with_redirects_disabled(messages_url)
        .search("test")
        .await
        .unwrap_err();

    let _ = shutdown.send(());
    task.abort();
    assert_eq!(result.code, "upstream_http_error");
    assert_eq!(result.upstream_status, Some(307));
    assert_eq!(redirected_requests.load(Ordering::SeqCst), 0);
}

async fn redirect_response(State(_): State<Arc<AtomicUsize>>) -> Response<Body> {
    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header(header::LOCATION, "/redirected")
        .body(Body::empty())
        .unwrap()
}

async fn record_redirect(State(requests): State<Arc<AtomicUsize>>) -> StatusCode {
    requests.fetch_add(1, Ordering::SeqCst);
    StatusCode::OK
}

fn client_with_redirects_disabled(url: Url) -> DeepSeekSearchClient {
    DeepSeekSearchClient::with_parts(
        Some(ApiKey::new("test-key")),
        url,
        "deepseek-v4-flash",
        Client::builder()
            .timeout(Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap(),
    )
}

#[tokio::test]
async fn maps_upstream_errors_without_echoing_body() {
    let server = MockServer::start(
        StatusCode::TOO_MANY_REQUESTS,
        json!({"error":{"type":"rate_limit_error","message":"sentinel-upstream-detail"}}),
    )
    .await;

    let error = client(server.url.clone()).search("test").await.unwrap_err();
    assert_eq!(error.code, "upstream_rate_limited");
    assert!(error.retryable);
    assert_eq!(error.upstream_status, Some(429));
    assert!(!error.message.contains("test-key"));
    assert!(!error.message.contains("sentinel-upstream-detail"));
    assert!(!error
        .as_json()
        .to_string()
        .contains("sentinel-upstream-detail"));
}
