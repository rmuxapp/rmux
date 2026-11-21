//! PTY + engine pressure test. No GUI, glyph shaping or presentation timing.
use anyhow::{Result, ensure};
use portable_pty::CommandBuilder;
use rmux_terminal::session::Session;
use std::{
    hint::black_box,
    io::{self, BufRead, Write},
    thread,
    time::{Duration, Instant},
};

fn child(mode: &str) -> Result<()> {
    let mut output = io::stdout().lock();
    if mode == "--flood" || mode == "--focus-flood" {
        if mode == "--focus-flood" { output.write_all(b"\x1b[?1004h")?; }
        let chunk = "\x1b[32mbuild\x1b[0m output pressure 界 e\u{301}\r\n".repeat(256);
        loop {
            if mode == "--focus-flood" { output.write_all(b"\x1b]7501;state=working:id=build\x07")?; }
            output.write_all(chunk.as_bytes())?;
        }
    }
    writeln!(output, "responder-ready")?;
    output.flush()?;
    for line in io::stdin().lock().lines() {
        // Responses deliberately differ from input so local PTY echo cannot
        // satisfy the latency measurement.
        writeln!(output, "pong-{}", line?.trim())?;
        output.flush()?;
    }
    Ok(())
}

fn spawn(mode: &str) -> Result<Session> {
    let mut command = CommandBuilder::new(std::env::current_exe()?);
    command.arg(mode);
    command.env("TERM", "xterm-256color");
    Session::spawn(command, 160, 48)
}

fn percentile(values: &[u128], percent: usize) -> u128 {
    // Nearest-rank percentile, with a minimum rank of one.
    values[(values.len() * percent).div_ceil(100).max(1) - 1]
}

fn benchmark(panes: usize) -> Result<()> {
    ensure!([1, 4, 8, 16].contains(&panes), "use 1, 4, 8 or 16 panes");
    let responder = spawn("--echo")?;
    let busy: Vec<_> = (1..panes)
        .map(|_| spawn("--flood"))
        .collect::<Result<_>>()?;
    let ready_deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if responder.frame()?.text().contains("responder-ready")
            && busy.iter().all(|s| s.output_bytes() > 0)
        {
            break;
        }
        ensure!(
            Instant::now() < ready_deadline,
            "PTY workloads did not become ready"
        );
        thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(Duration::from_millis(200));
    let before: Vec<_> = busy.iter().map(Session::output_bytes).collect();
    let start = Instant::now();
    let mut admission = Vec::with_capacity(128);
    let mut responses = Vec::with_capacity(128);
    let mut snapshot_batches = Vec::new();
    for sequence in 0..128 {
        let request = format!("{sequence:04}\n");
        let expected = format!("pong-{sequence:04}");
        let sent = Instant::now();
        responder.input(request.as_bytes())?;
        admission.push(sent.elapsed().as_nanos());
        loop {
            let snapshot_start = Instant::now();
            for session in &busy {
                black_box(session.frame()?);
            }
            let response = responder.frame()?;
            snapshot_batches.push(snapshot_start.elapsed().as_nanos());
            if response.text().lines().any(|line| line == expected) {
                responses.push(sent.elapsed().as_nanos());
                break;
            }
            ensure!(
                sent.elapsed() < Duration::from_secs(2),
                "response timeout at sample {sequence}"
            );
            // Polling is in this measurement harness, not the application.
            thread::sleep(Duration::from_micros(100));
        }
        thread::sleep(Duration::from_millis(5));
    }
    let elapsed = start.elapsed().as_secs_f64();
    let bytes: Vec<_> = busy
        .iter()
        .zip(before)
        .map(|(session, before)| session.output_bytes() - before)
        .collect();
    ensure!(
        bytes.iter().all(|count| *count > 0),
        "a flood pane stopped producing output"
    );
    let total_bytes: u64 = bytes.iter().sum();
    admission.sort_unstable();
    responses.sort_unstable();
    snapshot_batches.sort_unstable();
    println!(
        "{{\"panes\":{panes},\"flood_panes\":{},\"columns\":160,\"rows\":48,\"samples\":128,\"elapsed_seconds\":{elapsed:.6},\"parsed_flood_bytes\":{total_bytes},\"flood_mib_per_second\":{:.3},\"input_call_p50_ns\":{},\"input_call_p99_ns\":{},\"response_snapshot_p50_ns\":{},\"response_snapshot_p95_ns\":{},\"response_snapshot_p99_ns\":{},\"snapshot_batch_p99_ns\":{}}}",
        busy.len(),
        total_bytes as f64 / 1048576. / elapsed,
        percentile(&admission, 50),
        percentile(&admission, 99),
        percentile(&responses, 50),
        percentile(&responses, 95),
        percentile(&responses, 99),
        percentile(&snapshot_batches, 99)
    );
    Ok(())
}

// Unlike the responder benchmark, input targets a terminal actively parsing output.
// This measures caller stalls, not rendering latency or child response latency.
fn keyboard_benchmark(panes: usize) -> Result<()> {
    let sessions: Vec<_> = (0..panes).map(|_| spawn("--flood")).collect::<Result<_>>()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while sessions.iter().any(|session| session.output_bytes() == 0) {
        ensure!(Instant::now() < deadline, "keyboard flood workloads did not become ready");
        thread::sleep(Duration::from_millis(5));
    }
    let before: u64 = sessions.iter().map(Session::output_bytes).sum();
    let mut presses = Vec::with_capacity(128);
    let mut releases = Vec::with_capacity(128);
    let mut text = Vec::with_capacity(128);
    let started = Instant::now();
    for _ in 0..128 {
        let target = &sessions[0];
        let time = Instant::now();
        ensure!(target.key("up", Default::default(), false)?, "up key was not encoded");
        presses.push(time.elapsed().as_nanos());
        let time = Instant::now();
        target.key_release("up", Default::default())?;
        releases.push(time.elapsed().as_nanos());
        let time = Instant::now();
        ensure!(!target.key_with_text("a", Default::default(), false, Some("a"))?, "ordinary text should use native composition");
        text.push(time.elapsed().as_nanos());
        thread::sleep(Duration::from_millis(5));
    }
    presses.sort_unstable(); releases.sort_unstable(); text.sort_unstable();
    let parsed = sessions.iter().map(Session::output_bytes).sum::<u64>() - before;
    ensure!(parsed > 0, "output stopped during keyboard sampling");
    println!(
        "{{\"mode\":\"keyboard_same_pane_output\",\"panes\":{panes},\"samples\":128,\"elapsed_seconds\":{:.6},\"parsed_flood_bytes\":{parsed},\"key_press_p50_ns\":{},\"key_press_p99_ns\":{},\"key_press_max_ns\":{},\"key_release_p99_ns\":{},\"key_release_max_ns\":{},\"native_text_dispatch_p99_ns\":{},\"native_text_dispatch_max_ns\":{}}}",
        started.elapsed().as_secs_f64(), percentile(&presses, 50), percentile(&presses, 99), presses.last().unwrap(),
        percentile(&releases, 99), releases.last().unwrap(), percentile(&text, 99), text.last().unwrap(),
    );
    // Wait for owned workload cleanup so the next size starts without prior floods.
    let shutdowns: Vec<_> = sessions.iter().map(Session::shutdown_handle).collect();
    drop(sessions);
    let deadline = Instant::now() + Duration::from_secs(5);
    while shutdowns.iter().any(|shutdown| !shutdown.is_finished()) {
        ensure!(Instant::now() < deadline, "keyboard workload cleanup timed out");
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn focus_benchmark(panes: usize, queued: bool) -> Result<()> {
    let sessions: Vec<_> = (0..panes).map(|_| spawn("--focus-flood")).collect::<Result<_>>()?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while sessions.iter().any(|session| session.output_bytes() == 0) {
        ensure!(Instant::now() < deadline, "focus flood workloads did not become ready");
        thread::sleep(Duration::from_millis(5));
    }
    let before: u64 = sessions.iter().map(Session::output_bytes).sum();
    let mut entering = Vec::with_capacity(128);
    let mut leaving = Vec::with_capacity(128);
    let mut details = Vec::with_capacity(128);
    let started = Instant::now();
    let target = &sessions[0];
    let mut revision = 0;
    let mut status_busy = 0;
    for _ in 0..128 {
        let time = Instant::now();
        if queued { target.queue_focus(true)?; } else { target.focus(true)?; }
        // Preserve the legacy synchronous summary read only in baseline mode.
        // The queued UI path receives metadata on writer completion instead.
        if !queued && let Some((next, summary)) = target.program_status_since(revision) {
            revision = next;
            black_box(summary);
        }
        entering.push(time.elapsed().as_nanos());
        let time = Instant::now();
        if queued {
            let records = target.try_program_status_records();
            status_busy += usize::from(records.is_none());
            black_box(records);
        } else { black_box(target.program_status_records()); }
        details.push(time.elapsed().as_nanos());
        let time = Instant::now();
        if queued { target.queue_focus_with_releases(false, vec!["up".into()])?; }
        else { target.focus(false)?; }
        leaving.push(time.elapsed().as_nanos());
        thread::sleep(Duration::from_millis(5));
    }
    entering.sort_unstable(); leaving.sort_unstable(); details.sort_unstable();
    let parsed = sessions.iter().map(Session::output_bytes).sum::<u64>() - before;
    ensure!(parsed > 0, "output stopped during focus sampling");
    let mode = if queued { "queued_focus_same_pane_output" } else { "focus_same_pane_output" };
    println!(
        "{{\"mode\":\"{mode}\",\"panes\":{panes},\"samples\":128,\"elapsed_seconds\":{:.6},\"parsed_flood_bytes\":{parsed},\"status_busy\":{status_busy},\"focus_in_p50_ns\":{},\"focus_in_p99_ns\":{},\"focus_in_max_ns\":{},\"focus_out_p99_ns\":{},\"focus_out_max_ns\":{},\"status_details_p99_ns\":{},\"status_details_max_ns\":{}}}",
        started.elapsed().as_secs_f64(), percentile(&entering, 50), percentile(&entering, 99), entering.last().unwrap(),
        percentile(&leaving, 99), leaving.last().unwrap(), percentile(&details, 99), details.last().unwrap(),
    );
    let shutdowns: Vec<_> = sessions.iter().map(Session::shutdown_handle).collect();
    drop(sessions);
    let deadline = Instant::now() + Duration::from_secs(5);
    while shutdowns.iter().any(|shutdown| !shutdown.is_finished()) {
        ensure!(Instant::now() < deadline, "focus workload cleanup timed out");
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

fn main() -> Result<()> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "all".into());
    if mode == "--flood" || mode == "--focus-flood" || mode == "--echo" {
        return child(&mode);
    }
    if mode == "keyboard" {
        for panes in [1, 4, 8, 16] { keyboard_benchmark(panes)?; }
    } else if mode == "focus" || mode == "queued-focus" {
        for panes in [1, 4, 8, 16] { focus_benchmark(panes, mode == "queued-focus")?; }
    } else if mode == "all" {
        for panes in [1, 4, 8, 16] {
            benchmark(panes)?;
        }
    } else {
        benchmark(mode.parse()?)?;
    }
    Ok(())
}
