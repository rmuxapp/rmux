//! Bounded plain-text export of the active screen and its retained history.
use super::*;

#[derive(Default)]
struct Output<'a> {
    bytes: Vec<u8>,
    limit: usize,
    skip_lines: usize,
    truncated: bool,
    allocation_failed: bool,
    current: Option<&'a dyn Fn() -> bool>,
    next_check: usize,
    canceled: bool,
    callback_panicked: bool,
}

unsafe extern "C" fn write_output(userdata: *mut c_void, data: *const u8, len: usize) -> bool {
    // Ghostty calls synchronously with a valid context and borrowed byte slice.
    let output = unsafe { &mut *userdata.cast::<Output>() };
    if let Some(current) = output.current && output.bytes.len() >= output.next_check {
        output.next_check = output.bytes.len().saturating_add(4096);
        // Never unwind through Ghostty's C callback. Most callbacks contain just
        // one character, so sample cancellation by exported bytes instead.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(current)) {
            Ok(true) => {},
            Ok(false) => { output.canceled = true; return false; },
            Err(_) => { output.callback_panicked = true; return false; },
        }
    }
    let mut bytes = unsafe { std::slice::from_raw_parts(data, len) };
    while output.skip_lines > 0 {
        let Some(index) = bytes.iter().position(|byte| *byte == b'\n') else { return true; };
        bytes = &bytes[index + 1..];
        output.skip_lines -= 1;
    }
    let len = bytes.len();
    let accepted = len.min(output.limit.saturating_sub(output.bytes.len()));
    let required = output.bytes.len() + accepted;
    if required > output.bytes.capacity() {
        // Formatter callbacks can be as small as one character. Grow
        // geometrically rather than reallocating for each callback, while
        // keeping the requested capacity within the caller's export limit.
        let capacity = required
            .max(output.bytes.capacity().saturating_mul(2))
            .max(4096)
            .min(output.limit);
        if output
            .bytes
            .try_reserve_exact(capacity - output.bytes.len())
            .is_err()
        {
            output.allocation_failed = true;
            return false;
        }
    }
    output.bytes.extend_from_slice(&bytes[..accepted]);
    output.truncated |= accepted != len;
    !output.truncated
}

impl Terminal {
    /// Exports the active screen, including retained history, without selecting
    /// text or moving the viewport. Soft-wrapped rows are unwrapped.
    ///
    /// Returns the oldest text first, stopping at `max_bytes` (at most 16 MiB)
    /// and dropping any incomplete final UTF-8 code point. The boolean reports
    /// whether output was truncated. This is synchronous: callers must arrange
    /// background execution rather than hold up a UI thread for large exports.
    pub fn history_text_limited(&self, max_bytes: usize) -> Result<(String, bool)> {
        ensure!(max_bytes <= 16 * 1024 * 1024, "history export limit exceeds 16 MiB");
        self.export_text_limited(max_bytes, false).map(|text| text.expect("history always has a screen"))
    }

    /// Export the last `lines` logical rows of the active screen and history.
    /// Trailing rows omitted by the formatter do not count; soft wraps do not. Locates the
    /// range backwards before formatting, avoiding a copy of preceding history.
    /// The byte cap bounds the returned prefix of that range (not a byte tail).
    /// Like history_text_limited, run this on a background executor.
    pub fn history_tail_limited(&self, lines: usize, max_bytes: usize) -> Result<(String, bool)> {
        ensure!(max_bytes <= 16 * 1024 * 1024, "history export limit exceeds 16 MiB");
        if lines == 0 { return Ok((String::new(), false)); }
        // All references remain within this immutable engine borrow, and are
        // passed only to the formatter, never installed as the user's selection.
        unsafe {
            let mut total = 0usize;
            let mut columns = 0u16;
            check(ghostty_terminal_get(self.terminal, GHOSTTY_TERMINAL_DATA_TOTAL_ROWS, out(&mut total)))?;
            check(ghostty_terminal_get(self.terminal, GHOSTTY_TERMINAL_DATA_COLS, out(&mut columns)))?;
            if total == 0 || columns == 0 { return Ok((String::new(), false)); }
            ensure!(total - 1 <= u32::MAX as usize, "history exceeds coordinate range");
            let grid_at = |x, y| -> Result<GhosttyGridRef> {
                let mut grid = GhosttyGridRef::default();
                check(ghostty_terminal_grid_ref(self.terminal, GhosttyPoint {
                    tag: GHOSTTY_POINT_TAG_SCREEN,
                    value: GhosttyPointValue { coordinate: GhosttyPointCoordinate { x, y } },
                }, &mut grid))?;
                Ok(grid)
            };
            let mut row = total - 1;
            // Ask the same formatter whether a row emits text, with a zero-byte
            // writer budget. This avoids duplicating its Unicode/space/spacer
            // rules and allocates no output buffer. Only trailing blank rows
            // and the first nonblank row are probed.
            let end = loop {
                let end = grid_at(columns - 1, row as u32)?;
                let probe = GhosttySelection { start: grid_at(0, row as u32)?, end, rectangle: false };
                let (_, has_text) = self.export_text_range(0, false, true, Some(probe), None)?
                    .expect("explicit row range");
                if has_text { break end; }
                // Explicit spaces can cause the preceding line break to be
                // emitted even though trimming removes the row's own bytes.
                let mut spaces = false;
                for column in 0..columns {
                    let grid = grid_at(column, row as u32)?;
                    let mut cell = 0;
                    let mut codepoint = 0u32;
                    check(ghostty_grid_ref_cell(&grid, &mut cell))?;
                    check(ghostty_cell_get(cell, GHOSTTY_CELL_DATA_CODEPOINT, out(&mut codepoint)))?;
                    if codepoint == 32 { spaces = true; break; }
                }
                if spaces { break end; }
                if row == 0 { return Ok((String::new(), false)); }
                row -= 1;
            };
            let mut remaining = lines;
            let mut grid = grid_at(0, row as u32)?;
            let start = loop {
                if row == 0 { break grid; }
                let previous = grid_at(0, (row - 1) as u32)?;
                let mut raw = 0;
                let mut continuation = false;
                check(ghostty_grid_ref_row(&previous, &mut raw))?;
                check(ghostty_row_get(raw, GHOSTTY_ROW_DATA_WRAP, out(&mut continuation)))?;
                if !continuation { remaining -= 1; }
                if remaining == 0 { break grid; }
                row -= 1;
                // Reuse this reference on the next iteration instead of
                // resolving the same screen coordinate a second time.
                grid = previous;
            };
            let selection = GhosttySelection { start, end, rectangle: false };
            self.export_text_range(max_bytes, false, true, Some(selection), Some(lines))
                .map(|text| text.expect("explicit history range"))
        }
    }

    /// Stream only the current selection, preserving UTF-8 at the byte limit.
    pub fn selection_text_prefix(&self, max_bytes: usize) -> Result<Option<(String, bool)>> {
        self.selection_text_prefix_with_wraps(max_bytes, true)
    }

    pub fn selection_text_prefix_with_wraps(&self, max_bytes: usize, unwrap: bool) -> Result<Option<(String, bool)>> {
        ensure!(max_bytes <= 16 * 1024 * 1024, "history export limit exceeds 16 MiB");
        self.export_text_with_wraps(max_bytes, true, unwrap)
    }

    // Shared with the complete-selection API, which rejects truncated output.
    pub(super) fn export_text_limited(&self, max_bytes: usize, selected: bool) -> Result<Option<(String, bool)>> {
        self.export_text_with_wraps(max_bytes, selected, true)
    }

    pub(super) fn export_text_with_wraps(&self, max_bytes: usize, selected: bool, unwrap: bool) -> Result<Option<(String, bool)>> {
        self.export_text_range(max_bytes, selected, unwrap, None, None)
    }

    fn export_text_range(&self, max_bytes: usize, selected: bool, unwrap: bool, range: Option<GhosttySelection>, tail_lines: Option<usize>) -> Result<Option<(String, bool)>> {
        self.export_text_range_cancellable(max_bytes, selected, unwrap, range, tail_lines, None)
    }

    /// Complete clipboard selection, or no result if the owning copy is obsolete.
    pub fn selected_text_cancellable(&self, max_bytes: usize, unwrap: bool, current: &dyn Fn() -> bool) -> Result<Option<String>> {
        ensure!(max_bytes <= 16 * 1024 * 1024, "selection export limit exceeds 16 MiB");
        let Some((text, truncated)) = self.export_text_range_cancellable(max_bytes, true, unwrap, None, None, Some(current))? else { return Ok(None); };
        ensure!(!truncated, "selection exceeds {max_bytes} byte limit");
        Ok(Some(text))
    }

    fn export_text_range_cancellable(&self, max_bytes: usize, selected: bool, unwrap: bool, range: Option<GhosttySelection>, tail_lines: Option<usize>, current: Option<&dyn Fn() -> bool>) -> Result<Option<(String, bool)>> {
        let explicit_range = range.is_some();
        struct Formatter(GhosttyFormatter);
        impl Drop for Formatter {
            fn drop(&mut self) {
                unsafe { ghostty_formatter_free(self.0) };
            }
        }
        let mut output = Output {
            bytes: Vec::new(),
            limit: max_bytes,
            skip_lines: 0,
            truncated: false,
            allocation_failed: false,
            current,
            next_check: 0,
            canceled: false,
            callback_panicked: false,
        };
        // The immutable engine borrow excludes writes throughout formatting.
        unsafe {
            let mut selection = range.unwrap_or_default();
            if selected {
                let result = ghostty_terminal_get(self.terminal, GHOSTTY_TERMINAL_DATA_SELECTION, out(&mut selection));
                if result == GHOSTTY_NO_VALUE { return Ok(None); }
                check(result)?;
            }
            let mut formatter = Formatter(ptr::null_mut());
            check(ghostty_formatter_terminal_new(
                ptr::null(),
                &mut formatter.0,
                self.terminal,
            ))?;
            if selected || explicit_range {
                check(ghostty_formatter_set(formatter.0, GHOSTTY_FORMATTER_OPT_SELECTION, input(&selection)))?;
            }
            check(ghostty_formatter_set(
                formatter.0,
                GHOSTTY_FORMATTER_OPT_UNWRAP,
                input(&unwrap),
            ))?;
            if let Some(lines) = tail_lines {
                unsafe extern "C" fn count_lines(userdata: *mut c_void, data: *const u8, len: usize) -> bool {
                    let count = unsafe { &mut *userdata.cast::<usize>() };
                    let bytes = unsafe { std::slice::from_raw_parts(data, len) };
                    *count += bytes.iter().filter(|byte| **byte == b'\n').count();
                    true
                }
                // Row metadata bounds the candidate range, but erased/wrapped
                // rows may emit additional line breaks. Count actual formatted
                // separators without retaining text, then skip excess lines.
                let mut newlines = 0usize;
                check(ghostty_formatter_format(formatter.0, GhosttyWriter {
                    write: Some(count_lines), userdata: (&mut newlines as *mut usize).cast(),
                }))?;
                output.skip_lines = newlines.saturating_sub(lines - 1);
            }
            let result = ghostty_formatter_format(
                formatter.0,
                GhosttyWriter {
                    write: Some(write_output),
                    userdata: (&mut output as *mut Output).cast(),
                },
            );
            ensure!(!output.callback_panicked, "clipboard cancellation callback panicked");
            if output.canceled { return Ok(None); }
            ensure!(
                !output.allocation_failed,
                "could not allocate history export"
            );
            if !(output.truncated && result == GHOSTTY_IO_ERROR) {
                check(result)?;
            }
        }
        if let Err(error) = std::str::from_utf8(&output.bytes) {
            ensure!(
                output.truncated && error.error_len().is_none(),
                "formatter returned invalid UTF-8"
            );
            output.bytes.truncate(error.valid_up_to());
        }
        Ok(Some((String::from_utf8(output.bytes)?, output.truncated)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_tail_counts_logical_lines_without_mutating_viewport() -> Result<()> {
        let mut terminal = Terminal::new(8, 2)?;
        terminal.write("old\r\n世界abcdefgh\r\nlast".as_bytes());
        terminal.select_all()?;
        terminal.scroll(Scroll::Top)?;
        let before = terminal.frame()?;
        let selected = terminal.selected_text()?;
        assert_eq!(terminal.history_tail_limited(1, 4096)?, ("last".into(), false));
        assert_eq!(terminal.history_tail_limited(2, 4096)?, ("世界abcdefgh\nlast".into(), false));
        assert_eq!(terminal.history_tail_limited(2, 5)?, ("世".into(), true));
        assert_eq!(terminal.history_tail_limited(0, 4096)?, (String::new(), false));
        assert_eq!(terminal.history_tail_limited(usize::MAX, 4096)?, terminal.history_text_limited(4096)?);
        assert_eq!(terminal.selected_text()?, selected);
        assert_eq!(terminal.frame()?.scrollback_offset, before.scrollback_offset);
        terminal.write(b"\x1b[?1049h\x1b[HALT");
        assert_eq!(terminal.history_tail_limited(2, 4096)?, ("ALT".into(), false));
        assert_eq!(terminal.history_tail_limited(1, 4096)?, ("ALT".into(), false));
        Ok(())
    }

    #[test]
    fn history_tail_matches_full_text_suffix_with_blank_and_wrapped_rows() -> Result<()> {
        for input in [
            "", "hello", "hello\r\n", "a\r\n\r\nb\r\n\r\n",
            "世界abcdefgh\r\n\r\nlast\r\n", "a\r\n   ",
            "12345678        ", "old\r\n\x1b[41m   ",
            "a\r\nb\r\nc\r\nd\r\ne\r\nf\r\ng\r\n",
        ] {
            let mut terminal = Terminal::new(8, 5)?;
            terminal.write(input.as_bytes());
            let (full, truncated) = terminal.history_text_limited(4096)?;
            assert!(!truncated);
            for lines in 1..=8 {
                let start = full.match_indices('\n').rev().nth(lines - 1).map_or(0, |(index, _)| index + 1);
                let expected = &full[start..];
                assert_eq!(terminal.history_tail_limited(lines, 4096)?, (expected.into(), false), "input={input:?}, lines={lines}");
            }
        }
        Ok(())
    }

    #[test]
    fn history_tail_matches_suffix_after_terminal_edits() -> Result<()> {
        let mut seed = 17u64;
        let fragments = ["x", " ", "世界", "\r\n", "\t", "\x1b[K", "\x1b[2K", "\r", "\x1b[31m", "\x1b[0m"];
        let mut terminal = Terminal::new(8, 5)?;
        for step in 0..200 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            terminal.write(fragments[(seed >> 32) as usize % fragments.len()].as_bytes());
            let (full, truncated) = terminal.history_text_limited(65536)?;
            assert!(!truncated);
            for lines in 1..=6 {
                let start = full.match_indices('\n').rev().nth(lines - 1).map_or(0, |(index, _)| index + 1);
                let expected = &full[start..];
                assert_eq!(terminal.history_tail_limited(lines, 65536)?, (expected.into(), false), "step={step}, lines={lines}, full={full:?}");
                for limit in [0, 1, 5] {
                    let (text, truncated) = terminal.history_tail_limited(lines, limit)?;
                    assert!(text.len() <= limit && expected.starts_with(&text));
                    assert_eq!(truncated, expected.len() > limit);
                }
            }
        }
        Ok(())
    }

    #[test]
    #[ignore = "explicit tail export timing fixture"]
    fn history_tail_costs() -> Result<()> {
        for (name, input, lines) in [
            ("short_tail_large_history", "old output line abcdefghijklmnopqrstuvwxyz\r\n".repeat(25000), 20),
            ("long_wrapped_line", "abcdefgh".repeat(131072), 1),
            ("blank_screen", String::new(), 20),
        ] {
            let mut terminal = Terminal::new(80, 24)?;
            terminal.set_scrollback_limit(64 * 1024 * 1024)?;
            terminal.write(input.as_bytes());
            let mut times = Vec::new();
            let mut bytes = 0;
            let mut truncated = false;
            for _ in 0..11 {
                let started = std::time::Instant::now();
                let result = terminal.history_tail_limited(lines, 65536)?;
                times.push(started.elapsed().as_micros());
                bytes = result.0.len();
                truncated = result.1;
                std::hint::black_box(result);
            }
            times.sort_unstable();
            println!("{{\"case\":\"{name}\",\"input_bytes\":{},\"returned_bytes\":{bytes},\"truncated\":{truncated},\"median_us\":{},\"max_us\":{}}}", input.len(), times[5], times[10]);
        }
        Ok(())
    }

    #[test]
    fn selection_prefix_is_bounded_and_preserves_selection_and_viewport() -> Result<()> {
        let mut terminal = Terminal::new(20, 3)?;
        terminal.write("世界 end\r\nnext".as_bytes());
        assert!(terminal.selection_text_prefix(5)?.is_none());
        terminal.select_all()?;
        let selected = terminal.selected_text()?.unwrap();
        let before = terminal.frame()?;
        assert_eq!(terminal.selection_text_prefix(5)?, Some(("世".into(), true)));
        assert_eq!(terminal.selection_text_prefix(1024)?, Some((selected.clone(), false)));
        assert_eq!(terminal.selected_text()?, Some(selected));
        let after = terminal.frame()?;
        assert_eq!(after.selections, before.selections);
        assert_eq!(after.scrollback_offset, before.scrollback_offset);
        Ok(())
    }

    #[test]
    fn small_writes_grow_geometrically_and_stop_at_limit() {
        let mut output = Output {
            bytes: Vec::new(),
            limit: 10_000,
            skip_lines: 0,
            truncated: false,
            allocation_failed: false,
            ..Default::default()
        };
        let mut growths = 0;
        for _ in 0..output.limit {
            let capacity = output.bytes.capacity();
            assert!(unsafe { write_output((&mut output as *mut Output).cast(), b"x".as_ptr(), 1) });
            growths += usize::from(output.bytes.capacity() != capacity);
        }
        assert!(growths <= 3, "{growths} allocations for one-byte writes");
        assert!(!unsafe { write_output((&mut output as *mut Output).cast(), b"x".as_ptr(), 1) });
        assert_eq!(output.bytes.len(), output.limit);
        assert!(output.truncated && !output.allocation_failed);
    }
    #[test]
    fn clipboard_cancellation_stops_streaming_without_changing_selection() -> Result<()> {
        let mut terminal = Terminal::new(80, 8)?;
        for _ in 0..300 { terminal.write("hello 世界 e\u{301} repeated text\r\n".as_bytes()); }
        terminal.select_all()?;
        let expected = terminal.selected_text()?.unwrap();
        assert!(expected.len() > 8192);
        let calls = std::cell::Cell::new(0);
        let current = || { let count = calls.get(); calls.set(count + 1); count == 0 };
        assert!(terminal.selected_text_cancellable(16 * 1024 * 1024, true, &current)?.is_none());
        assert_eq!(calls.get(), 2, "copy must stop at the first obsolete callback");
        assert_eq!(terminal.selected_text()?, Some(expected.clone()));
        assert_eq!(terminal.selected_text_cancellable(16 * 1024 * 1024, true, &|| true)?, Some(expected));
        assert!(terminal.selected_text_cancellable(16, true, &|| true).is_err(), "partial clipboard contents must not be returned");
        Ok(())
    }
    #[test]
    fn history_export_preserves_viewport_and_selection_and_is_screen_local() -> Result<()> {
        let mut terminal = Terminal::new(12, 2)?;
        terminal.write("first 世界\r\nsecond\r\nthird\r\nfourth".as_bytes());
        terminal.scroll(Scroll::Top)?;
        terminal.selection_event(selection::Event {
            action: selection::Action::Press,
            x: 10.,
            y: 0.,
            cell_width: 10.,
            cell_height: 20.,
            rectangle: false,
            clicks: 2,
        })?;
        let selected = terminal.selected_text()?;
        let before = terminal.frame()?;
        let (text, truncated) = terminal.history_text_limited(4096)?;
        assert!(
            text.starts_with("first 世界\nsecond\nthird\nfourth"),
            "{text:?}"
        );
        assert!(!truncated);
        assert_eq!(terminal.selected_text()?, selected);
        let after = terminal.frame()?;
        assert_eq!(before.scrollback_offset, after.scrollback_offset);
        assert_eq!(before.text(), after.text());
        terminal.write(b"\x1b[?1049hALT");
        let (alternate, _) = terminal.history_text_limited(4096)?;
        assert!(alternate.contains("ALT") && !alternate.contains("first"));
        Ok(())
    }
    #[test]
    fn history_export_stops_at_utf8_safe_byte_limit() -> Result<()> {
        let mut terminal = Terminal::new(8, 2)?;
        terminal.write("世界abcdefghijk".as_bytes());
        let (full, truncated) = terminal.history_text_limited(4096)?;
        assert!(
            !truncated && full.starts_with("世界abcdefghijk"),
            "{full:?}"
        );
        for limit in 0..=full.len() {
            let (prefix, truncated) = terminal.history_text_limited(limit)?;
            assert!(prefix.len() <= limit && full.starts_with(&prefix));
            assert_eq!(truncated, limit < full.len());
        }
        Ok(())
    }
}
