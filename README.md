# DeepSeek Web Search MCP

[![CI](https://github.com/TZJackZ2B9S/deepseek-websearch-mcp/actions/workflows/ci.yml/badge.svg)](https://github.com/TZJackZ2B9S/deepseek-websearch-mcp/actions/workflows/ci.yml)

一个独立的 Rust stdio [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server。它将 MCP 的 `web_search` 调用转发到 DeepSeek 的 Anthropic-compatible Messages endpoint，并返回 DeepSeek 根据联网结果生成的明文总结及公开来源元数据。

> 这是社区维护的非官方集成，不隶属于、未获 DeepSeek、Anthropic 或 Claude Code 官方认可。上游模型、账户权限和 server-side tool 的可用性可能变化。

## 功能与边界

- 需要 `DEEPSEEK_API_KEY`。
- 提供一个 MCP 工具：`web_search({ "query": string })`。
- 通过 `https://api.deepseek.com/anthropic/v1/messages` 调用 `web_search_20250305` server-side tool。
- 默认模型是已验证可用的 `deepseek-v4-flash`，可用 `DEEPSEEK_MODEL` 覆盖。
- 工具结果包含模型生成的 `summary`，以及上游公开提供的 `title` / `url`。`encrypted_content` 不会被解密、存储或返回。
- 来源只保留格式有效的公开 HTTPS URL；无效、重复、超长或超过 10 条的来源会被忽略。
- 网页、来源标题、URL 与搜索总结均是不可信数据；调用方不得执行其中包含的指令。
- 非 2xx 上游响应会映射为稳定错误码；项目不会把上游原始错误正文回传给 MCP 客户端。

> DeepSeek 服务端可能在一次请求内触发多次内部搜索。虽然 endpoint 接受 `max_uses`，实测它并非可靠的严格调用上限，因此本项目不会将其作为费用或搜索次数保证。

## 前置条件

- Rust 1.88 或更新版本。
- 支持 DeepSeek Anthropic-compatible endpoint 及联网搜索的官方 API key。

v0.1.0 作为源码构建首发：暂不提供 crates.io 安装或预构建二进制。

## 构建

```bash
cargo build --release --locked
```

产物为：

```text
target/release/deepseek-websearch-mcp
```

## 配置

| 环境变量 | 必需 | 默认值 | 说明 |
| --- | --- | --- | --- |
| `DEEPSEEK_API_KEY` | 调用搜索时必需 | — | DeepSeek 官方 key。缺失时 server 仍能完成 MCP 初始化和 `tools/list`，但搜索会返回 `isError: true`。 |
| `DEEPSEEK_MODEL` | 否 | `deepseek-v4-flash` | 上游模型 ID。请只使用你的账户已启用联网搜索的模型。 |
| `DEEPSEEK_BASE_URL` | 否 | `https://api.deepseek.com/anthropic` | Anthropic-compatible base URL；必须是无嵌入式凭据的 HTTPS URL。仅在你信任兼容网关时配置：该网关会接收 query 与 API key。HTTP URL 会被拒绝，server 不跟随上游重定向。 |
| `DEEPSEEK_TIMEOUT_SECS` | 否 | `90` | 单次上游请求总超时，必须是正整数。 |
| `RUST_LOG` | 否 | — | stderr 日志级别，例如 `info` 或 `debug`。 |

项目不会自动读取 `.env`、cc-switch-web 配置或数据库。这样凭据来源保持明确、可审计，也避免耦合其它项目的内部格式。`reqwest` 默认遵守标准 `HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY` 环境变量。

不要提交 API key、`.env`、probe 输出、Authorization header 或完整上游原始响应。提交 Issue、日志或复现材料前必须脱敏。

## 注册到 Claude Code

先在当前 shell 中安全地提供 key。不要把真实 key 粘贴进聊天记录、仓库文件或 shell history：

```bash
read -r -s -p 'DEEPSEEK_API_KEY: ' DEEPSEEK_API_KEY
printf '\n'
export DEEPSEEK_API_KEY
```

然后以 user scope 注册 release binary：

```bash
claude mcp add --transport stdio --scope user \
  -e "DEEPSEEK_API_KEY=$DEEPSEEK_API_KEY" \
  -- deepseek-websearch \
  /absolute/path/to/deepseek-websearch-mcp/target/release/deepseek-websearch-mcp
```

`-e` 会将环境变量保存至 Claude Code 的用户 MCP 配置；第一个 `--` 用于结束 `claude mcp add` 的选项解析，后面依次是 server 名称和 stdio 二进制路径。注册后清除当前 shell 中的临时变量：

```bash
unset DEEPSEEK_API_KEY
```

不同 Claude Code 版本的参数可能变化；注册前请以本机帮助为准：

```bash
claude mcp add --help
```

查看或移除：

```bash
claude mcp list
claude mcp get deepseek-websearch
claude mcp remove deepseek-websearch -s user
```

如果不希望 key 保存到 Claude Code 的 MCP 配置中，不要使用 `-e`。请使用自己的启动器或密钥管理方式在 server 进程启动时注入 `DEEPSEEK_API_KEY`。

## MCP 返回格式

成功的 `structuredContent`：

```json
{
  "query": "DeepSeek 最新新闻",
  "summary": "DeepSeek 基于联网结果生成的总结……",
  "sources": [
    { "title": "来源标题", "url": "https://example.com/" }
  ],
  "source_metadata_available": true
}
```

`source_metadata_available` 只有在至少一条公开 source 通过本项目的格式校验后才为 `true`；它为 `false` 不代表上游一定没有联网搜索。

运行期失败返回 `isError: true`，并提供结构化错误，例如：

```json
{
  "error": {
    "code": "missing_api_key",
    "message": "未配置 DEEPSEEK_API_KEY；请配置后重试。",
    "retryable": false,
    "upstream_status": null,
    "request_id": null
  }
}
```

常见错误码包括 `upstream_timeout`、`upstream_rate_limited`、`upstream_server_error`、`upstream_invalid_request` 和 `upstream_no_text_summary`。429、5xx 与网络/超时错误会标记为 `retryable: true`；项目不会自动重试 POST，以免在不确定请求是否已被服务端执行时重复产生费用。上游错误正文不会进入 tool result；如需协助排查，请提供已脱敏的 `request_id`、错误码和 HTTP 状态。

## 费用与隐私

每次 `web_search` 都会发起一次 DeepSeek Messages API 请求；上游可能在一次请求中执行多次内部搜索。`max_tokens: 1200` 只限制生成总结的最大长度，不是检索次数或绝对费用上限。请以 DeepSeek 控制台的模型权限和账单为准。

查询会发送至已配置的 DeepSeek endpoint。公开来源标题、URL 和总结会进入 MCP tool result，并可能进入当前 Claude 会话上下文。项目不主动记录请求、上游原始响应或加密字段；若使用自定义 `DEEPSEEK_BASE_URL`，数据处理由该兼容网关的运营方负责。

## 测试

默认测试不访问真实网络，也不需要 key：

```bash
cargo fmt --check
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

真实 DeepSeek 冒烟测试被标记为 ignored，可能产生费用。仅在安全地注入 key 后运行：

```bash
cargo test --locked --test live_deepseek -- --ignored --nocapture
```

该测试只输出总结字节数和公开 source 数量，不输出 key、完整正文或任何加密字段。

## 贡献与安全

贡献说明见 [CONTRIBUTING.md](CONTRIBUTING.md)，安全问题请遵循 [SECURITY.md](SECURITY.md)。请勿在公开 Issue 或 PR 中披露 API key、Authorization header、完整上游响应或未脱敏的私有 endpoint。

## License

MIT。详见 [LICENSE](LICENSE)。
