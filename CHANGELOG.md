# Changelog

所有值得注意的项目变更都会记录在本文件中。

## [0.1.0] - 2026-08-06

### Added

- 提供 `web_search({ "query": string })` stdio MCP 工具。
- 通过 DeepSeek Anthropic-compatible Messages endpoint 使用 `web_search_20250305`。
- 返回结构化 `summary`、公开来源 metadata 和稳定的错误结果。
- 提供无 key、上游错误、stdio 和 mock HTTP 的离线测试。

### Security

- 只接受 HTTPS 上游 endpoint，并禁用上游重定向跟随。
- 不解密、记录或返回 `encrypted_content`，不回传原始上游错误正文。
- 对公开 source URL、标题和数量设置校验及上限。
