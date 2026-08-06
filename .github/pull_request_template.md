## Summary

## Validation

- [ ] Ran `cargo fmt --check`
- [ ] Ran `cargo check --locked`
- [ ] Ran `cargo clippy --all-targets --locked -- -D warnings`
- [ ] Ran `cargo test --locked`
- [ ] Updated tests and documentation where applicable

## Safety checklist

- [ ] stdout remains limited to MCP JSON-RPC output
- [ ] No API key, Authorization header, raw upstream response, encrypted content, or private endpoint was added
- [ ] Any upstream request or error-handling change has an offline contract test
