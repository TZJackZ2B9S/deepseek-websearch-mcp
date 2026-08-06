use std::{
    io::Write,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use serde_json::{json, Value};

fn read_response(stdout: &mut std::io::BufReader<std::process::ChildStdout>) -> Value {
    use std::io::BufRead;
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn missing_key_keeps_tool_available_and_stdout_clean() {
    let binary = env!("CARGO_BIN_EXE_deepseek-websearch-mcp");
    let mut child = Command::new(binary)
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("RUST_LOG")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());

    writeln!(
        stdin,
        "{}",
        json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{
                "protocolVersion":"2026-07-28",
                "capabilities":{},
                "clientInfo":{"name":"test","version":"0"}
            }
        })
    )
    .unwrap();
    stdin.flush().unwrap();
    let initialize = read_response(&mut stdout);
    assert_eq!(initialize["id"], 1);
    assert!(
        initialize["result"]["capabilities"]["tools"].is_object(),
        "{initialize}"
    );

    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}})
    )
    .unwrap();
    stdin.flush().unwrap();
    let tool_list = read_response(&mut stdout);
    assert_eq!(tool_list["id"], 2);
    assert!(tool_list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "web_search"));

    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"current news"}}})
    )
    .unwrap();
    stdin.flush().unwrap();
    let tool_call = read_response(&mut stdout);
    assert_eq!(tool_call["id"], 3);
    assert_eq!(tool_call["result"]["isError"], true);
    assert_eq!(
        tool_call["result"]["structuredContent"]["error"]["code"],
        "missing_api_key"
    );

    drop(stdin);
    thread::sleep(Duration::from_millis(50));
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}
