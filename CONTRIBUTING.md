# Contributing

感谢你考虑为 DeepSeek Web Search MCP 做贡献。

## 开发要求

- Rust 1.88 或更新版本。
- 默认测试必须离线运行，不能依赖真实 API key 或真实网络。

提交前请运行：

```bash
cargo fmt --check
cargo check --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

## 真实上游测试

`tests/live_deepseek.rs` 是 `#[ignore]` 测试，会调用 DeepSeek 并可能产生费用。它只能由维护者在安全注入 API key 后手动运行：

```bash
cargo test --locked --test live_deepseek -- --ignored --nocapture
```

不要在普通 CI、fork PR 或公开日志中运行该测试。

## 贡献边界

- 不要提交 API key、`.env`、Authorization header、完整上游原始响应、`encrypted_content`、probe 输出或私有 endpoint。
- 新增或修改上游交互行为时，先在 `tests/upstream_contract.rs` 增加离线 mock contract test。
- 不要向 stdout 写日志；stdio stdout 只能输出 MCP JSON-RPC。
- 保持单一 `web_search` 工具、显式环境变量配置和无状态请求模型，除非变更已在 Issue 或 PR 中明确讨论。
- 文档、错误码、测试和实现必须同步更新。

## 提交 Pull Request

请说明问题、解决方案和验证命令。若修改了安全边界、凭据处理或上游请求行为，请在 PR 描述中明确标注，并确认示例与日志已脱敏。
