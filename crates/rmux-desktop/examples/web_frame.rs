//! Capture a disposable Ghostty viewport for the shared web renderer probe.
fn main() -> anyhow::Result<()> {
    let mut terminal = rmux_terminal::Terminal::new(52, 8)?;
    terminal.write(b"rmux shared renderer\r\n\x1b[32m$ cargo test\x1b[0m\r\n\x1b[1mCached terminal rows\x1b[0m\r\n\x1b[4munderline\x1b[0m  \x1b[9mstrike\x1b[0m\r\n\x1b[44m colored cells \x1b[0m\r\n\x1b[8mhidden text\x1b[0mvisible\r\n$ ");
    println!("{}", serde_json::to_string(&terminal.frame()?)?);
    Ok(())
}
