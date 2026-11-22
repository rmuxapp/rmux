//! Small external client: no GPUI, Ghostty, shell or terminal dependencies.
mod commands;
mod completions;
mod pipe;
mod follow;
use serde_json::{Value, json};
use std::{ffi::OsString, path::PathBuf};

const HELP: &str = "Usage: rmuxctl [--socket PATH] METHOD [JSON_OBJECT]\n\nConnect to an existing rmux Unix socket. Without --socket, use\n$RMUX_SOCKET_DIR/rpc.sock. Print the JSON-RPC response to stdout.\nExit 0 on success, 1 on RPC error, 2 on usage/transport failure.\nRequests are never retried: a timeout may have an unknown outcome.\n";
#[derive(Clone)]
struct Request {
    socket: PathBuf,
    body: Vec<u8>,
    pipe_command: Option<String>,
    pipe_json: bool,
    follow: bool,
}
fn parse(
    args: impl IntoIterator<Item = OsString>,
    directory: Option<OsString>,
) -> Result<Option<Request>, String> {
    let mut args = args.into_iter().peekable();
    if args.peek().is_some_and(|v| v == "--help" || v == "-h") {
        return Ok(None);
    }
    let socket = if args.peek().is_some_and(|v| v == "--socket") {
        args.next();
        let path = args.next().ok_or("--socket requires a path")?;
        if path.is_empty() {
            return Err("socket path must not be empty".into());
        }
        PathBuf::from(path)
    } else {
        let directory = directory
            .filter(|v| !v.is_empty())
            .ok_or("specify --socket PATH or RMUX_SOCKET_DIR")?;
        PathBuf::from(directory).join("rpc.sock")
    };
    let method = args
        .next()
        .ok_or("a method is required")?
        .into_string()
        .map_err(|_| "method must be UTF-8")?;
    if method.is_empty() || method.starts_with('-') {
        return Err("a nonempty RPC method is required".into());
    }
    let piping = method == "pipe-pane";
    let reading_events = method == "read-events";
    let (method, mut params) = commands::expand_with_input(&method, args.collect(), std::io::stdin().lock())?;
    let follow = reading_events && params.as_object_mut().unwrap().remove("follow").is_some();
    if follow {
        let wait = params.as_object_mut().unwrap().entry("wait_ms").or_insert(json!(3000));
        if wait.as_u64() == Some(0) { return Err("--follow requires --wait-ms greater than zero".into()); }
    }
    let pipe_json = piping && params.as_object_mut().unwrap().remove("json").is_some();
    let pipe_command = if piping {
        Some(params.as_object_mut().unwrap().remove("command").unwrap().as_str().unwrap().to_owned())
    } else { None };
    let mut body =
        serde_json::to_vec(&json!({"jsonrpc":"2.0", "id":1, "method":method, "params":params}))
            .unwrap();
    if body.len() > 64 * 1024 {
        return Err("request exceeds 64 KiB".into());
    }
    body.push(b'\n');
    Ok(Some(Request { socket, body, pipe_command, pipe_json, follow }))
}
#[cfg(unix)]
fn exchange(request: Request) -> Result<Value, String> {
    use std::{
        io::{BufRead, BufReader, Read, Write},
        os::unix::net::UnixStream,
        time::Duration,
    };
    let mut stream = UnixStream::connect(request.socket).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(7)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| e.to_string())?;
    stream
        .write_all(&request.body)
        .map_err(|e| format!("write: {e}; request outcome may be unknown"))?;
    // Bound allocation even if the endpoint is not an rmux server.
    const LIMIT: u64 = 8 * 1024 * 1024;
    let mut line = Vec::new();
    BufReader::new(stream)
        .take(LIMIT + 1)
        .read_until(b'\n', &mut line)
        .map_err(|e| format!("read: {e}; request outcome may be unknown"))?;
    if line.len() as u64 > LIMIT || !line.ends_with(b"\n") {
        return Err("incomplete or oversized reply; request outcome may be unknown".into());
    }
    let reply: Value = serde_json::from_slice(&line).map_err(|e| format!("invalid reply: {e}"))?;
    if reply.get("jsonrpc") != Some(&json!("2.0"))
        || reply.get("id") != Some(&json!(1))
        || (reply.get("result").is_some() == reply.get("error").is_some())
    {
        return Err("invalid JSON-RPC response envelope".into());
    }
    Ok(reply)
}
#[cfg(not(unix))]
fn exchange(_: Request) -> Result<Value, String> {
    Err("local socket transport is not yet available on this platform".into())
}
fn local_help(args: &[OsString]) -> Result<Option<String>, String> {
    if args.len() == 1 && args[0] == "--version" {
        return Ok(Some(format!("rmuxctl {}\n", env!("CARGO_PKG_VERSION"))));
    }
    let args = if args.first().is_some_and(|arg| arg == "--socket") && args.len() >= 2 {
        &args[2..]
    } else { args };
    if args.len() == 1 && matches!(args[0].to_str(), Some("--help" | "-h" | "help")) {
        return Ok(Some(format!("{HELP}\n{}\nOffline: rmuxctl help COMMAND | --version | completions bash|zsh|fish\n", commands::HELP)));
    }
    let command = if args.len() == 2 && args[0] == "help" {
        Some(&args[1])
    } else if args.len() == 2 && matches!(args[1].to_str(), Some("--help" | "-h")) {
        Some(&args[0])
    } else { None };
    command.map(|command| commands::help(command.to_str().ok_or("command must be UTF-8")?)).transpose()
}

fn run() -> Result<i32, String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if let Some(help) = local_help(&args)? {
        print!("{help}");
        return Ok(0);
    }
    if args.first().is_some_and(|arg| arg == "completions") {
        if args.len() != 2 { return Err("usage: rmuxctl completions bash|zsh|fish".into()); }
        print!("{}", completions::script(args[1].to_str().ok_or("shell must be UTF-8")?)?);
        return Ok(0);
    }
    if args.first().is_some_and(|arg| arg == "__complete") {
        for candidate in completions::candidates(&args[1..]) { println!("{candidate}"); }
        return Ok(0);
    }
    let Some(mut request) = parse(
        args,
        std::env::var_os("RMUX_SOCKET_DIR"),
    )?
    else {
        print!("{HELP}\n{}\nOffline shell setup: rmuxctl completions bash|zsh|fish\n", commands::HELP);
        return Ok(0);
    };
    let pipe_command = request.pipe_command.take();
    let pipe_json = request.pipe_json;
    if request.follow { return follow::run(request, &mut std::io::stdout().lock(), exchange); }
    let reply = exchange(request)?;
    if let Some(command) = pipe_command {
        if reply.get("error").is_some() { println!("{reply}"); return Ok(1); }
        let result = &reply["result"];
        if result.get("truncated").and_then(Value::as_bool) != Some(false) {
            return Err("pipe-pane capture is truncated or incomplete; shell command was not started".into());
        }
        let text = result.get("text").and_then(Value::as_str).ok_or("pipe-pane capture has no text; shell command was not started")?;
        if pipe_json {
            let (code, output) = pipe::run_json(&command, text)?;
            println!("{output}");
            return Ok(code);
        }
        return pipe::run(&command, text);
    }
    println!("{reply}");
    Ok(if reply.get("error").is_some() { 1 } else { 0 })
}
fn main() {
    std::process::exit(match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("rmuxctl: {error}");
            2
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pipe_pane_keeps_shell_command_out_of_the_rpc_request() {
        let request = parse(["--socket", "/tmp/test.sock", "pipe-pane", "--json", "--surface", "42", "--command", "cat"].map(OsString::from), None).unwrap().unwrap();
        assert_eq!(request.pipe_command.as_deref(), Some("cat"));
        assert!(request.pipe_json);
        let wire: Value = serde_json::from_slice(&request.body).unwrap();
        assert_eq!(wire["method"], "surface.read_text");
        assert_eq!(wire["params"], json!({"surface_id":"42","scope":"history","max_bytes":1048576}));
        assert!(parse(["--socket", "/tmp/test.sock", "pipe-pane", "cat"].map(OsString::from), None).is_err());
    }
    #[test]
    fn help_is_local_without_consuming_literal_payloads() {
        for args in [vec!["send", "--help"], vec!["help", "send"], vec!["--socket", "/missing/socket", "send", "-h"]] {
            let args: Vec<_> = args.into_iter().map(OsString::from).collect();
            assert!(local_help(&args).unwrap().unwrap().contains("--paste"));
        }
        let args: Vec<_> = ["send", "--surface", "id", "--", "--help"].into_iter().map(OsString::from).collect();
        assert!(local_help(&args).unwrap().is_none());
        assert!(local_help(&["help".into(), "unknown".into()]).is_err());
        assert!(local_help(&["--version".into()]).unwrap().unwrap().starts_with("rmuxctl "));
    }
    #[test]
    fn rejects_ambiguous_or_oversized_requests() {
        for args in [
            vec!["method", "[]"],
            vec!["method", "{}", "extra"],
            vec!["--socket", ""],
            vec!["--bad"],
        ] {
            assert!(
                parse(
                    args.into_iter().map(OsString::from),
                    Some("/tmp/test".into())
                )
                .is_err()
            );
        }
        assert!(
            parse(
                [
                    "method".into(),
                    json!({"text":"x".repeat(65536)}).to_string().into()
                ],
                Some("/tmp/test".into())
            )
            .is_err()
        );
    }
    #[cfg(unix)]
    #[test]
    fn exchanges_unicode_and_preserves_rpc_errors() {
        use std::{
            io::{BufRead, BufReader, Write},
            os::unix::net::UnixListener,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rpc.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = String::new();
            BufReader::new(&stream).read_line(&mut request).unwrap();
            let request: Value = serde_json::from_str(&request).unwrap();
            assert_eq!(request["params"]["name"], "世界");
            stream.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32601,\"message\":\"unknown\"}}\n").unwrap();
        });
        let request = parse(
            [
                "--socket".into(),
                path.into_os_string(),
                "test".into(),
                r#"{"name":"世界"}"#.into(),
            ],
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(exchange(request).unwrap()["error"]["code"], -32601);
        worker.join().unwrap();
    }
}
