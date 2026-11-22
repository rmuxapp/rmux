//! Read-only event following. No retries or unbounded buffered output.
use super::{Request, Value};
use std::io::{ErrorKind, Write};

pub(super) fn run(mut request: Request, output: &mut impl Write, mut exchange: impl FnMut(Request) -> Result<Value, String>) -> Result<i32, String> {
    let mut wire: Value = serde_json::from_slice(&request.body).map_err(|error| error.to_string())?;
    loop {
        let reply = exchange(request.clone())?;
        let cursor = if reply.get("error").is_some() { None } else {
            let batch = reply.get("result").ok_or("event reply has no result")?;
            let cursor = batch.get("cursor").and_then(Value::as_str)
                .filter(|cursor| !cursor.is_empty() && cursor.len() <= 96).ok_or("event reply has no valid cursor")?;
            if !batch.get("events").is_some_and(Value::is_array)
                || !batch.get("has_more").is_some_and(Value::is_boolean)
                || !matches!(batch.get("gap"), Some(Value::Null))
                    && !matches!(batch.get("gap").and_then(Value::as_str), Some("retention" | "restart")) {
                return Err("invalid event batch; following stopped".into());
            }
            Some(cursor.to_owned())
        };
        // Print/flush before advancing: a failed consumer must not silently
        // consume a checkpoint. Backpressure lives here, outside the server.
        if let Err(error) = writeln!(output, "{reply}").and_then(|_| output.flush()) {
            if error.kind() == ErrorKind::BrokenPipe { return Ok(0); }
            return Err(format!("event output: {error}"));
        }
        let Some(cursor) = cursor else { return Ok(1); };
        wire["params"]["after"] = Value::String(cursor);
        request.body = serde_json::to_vec(&wire).map_err(|error| error.to_string())?;
        request.body.push(b'\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn request() -> Request {
        super::super::parse(["--socket", "/unused", "read-events", "--follow", "--workspace", "42"].map(Into::into), None).unwrap().unwrap()
    }
    #[test]
    fn follow_preserves_filters_and_prints_checkpoints_gaps_and_rpc_errors() {
        let mut calls = 0;
        let mut output = Vec::new();
        let code = run(request(), &mut output, |request| {
            let wire: Value = serde_json::from_slice(&request.body).unwrap();
            assert_eq!(wire["params"]["workspace_id"], "42");
            assert_eq!(wire["params"]["wait_ms"], 3000);
            assert!(wire["params"].get("follow").is_none());
            if calls > 0 { assert_eq!(wire["params"]["after"], format!("1-1:{calls}")); }
            calls += 1;
            Ok(if calls == 3 { json!({"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"stop"}}) }
                else { json!({"jsonrpc":"2.0","id":1,"result":{"events":[],"cursor":format!("1-1:{calls}"),"has_more":calls==1,"gap":if calls==2 { Some("restart") } else { None }}}) })
        }).unwrap();
        assert_eq!(code, 1);
        let lines: Vec<_> = output.split(|byte| *byte == b'\n').filter(|line| !line.is_empty()).map(|line| serde_json::from_slice::<Value>(line).unwrap()).collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1]["result"]["gap"], "restart");
    }
    #[test]
    fn follow_stops_on_transport_or_invalid_batch_without_retry() {
        let mut output = Vec::new();
        let mut calls = 0;
        assert!(run(request(), &mut output, |_| { calls += 1; Err("disconnected".into()) }).is_err());
        assert_eq!(calls, 1);
        assert!(output.is_empty());
        assert!(run(request(), &mut output, |_| Ok(json!({"result":{"cursor":"1-1:1"}}))).is_err());
        assert!(output.is_empty());
        assert!(super::super::parse(["--socket", "/unused", "read-events", "--follow", "--wait-ms", "0"].map(Into::into), None).is_err());
    }
    #[test]
    fn a_closed_consumer_stops_following_without_another_request() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> { Err(ErrorKind::BrokenPipe.into()) }
            fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
        }
        let mut calls = 0;
        assert_eq!(run(request(), &mut Closed, |_| {
            calls += 1;
            Ok(serde_json::json!({"result":{"events":[],"cursor":"1-1:1","has_more":false,"gap":null}}))
        }).unwrap(), 0);
        assert_eq!(calls, 1);
    }
}
