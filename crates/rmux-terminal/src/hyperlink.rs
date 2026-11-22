//! On-demand URI lookup; no URI allocation in the normal rendering path.
use super::*;
impl Terminal {
    pub fn link_at(&mut self, column: u16, row: u32) -> Result<Option<String>> {
        if self.frame.revision != self.revision {
            return Ok(None);
        }
        if let Some(uri) = self.hyperlink_at(column, row)? {
            return Ok(Some(uri));
        }
        if column >= self.frame.columns || row as usize >= self.frame.rows.len() {
            return Ok(None);
        }
        let mut first = row;
        while first > 0 && self.row_wraps(first - 1)? {
            first -= 1;
        }
        // A clipped logical line must not launch a truncated URL.
        if first == 0 && self.row_flag(0, GHOSTTY_ROW_DATA_WRAP_CONTINUATION)? {
            return Ok(None);
        }
        let mut text = String::new();
        let mut clicked = None;
        let mut current = first;
        loop {
            for cell in self.frame.rows[current as usize]
                .iter()
                .filter(|c| c.width != 0)
            {
                if current == row
                    && column >= cell.column
                    && column < cell.column + cell.width as u16
                {
                    clicked = Some(text.len());
                }
                if cell.hidden || cell.text.is_empty() {
                    text.push(' ');
                } else {
                    text.push_str(&cell.text);
                }
                if text.len() > 8192 {
                    return Ok(None);
                }
            }
            if !self.row_wraps(current)? {
                break;
            }
            current += 1;
            if current as usize >= self.frame.rows.len() {
                return Ok(None);
            }
        }
        Ok(clicked
            .and_then(|offset| plain_url_at(&text, offset).or_else(|| plain_path_at(&text, offset)))
            .map(str::to_owned))
    }

    fn row_wraps(&self, row: u32) -> Result<bool> {
        self.row_flag(row, GHOSTTY_ROW_DATA_WRAP)
    }

    fn row_flag(&self, row: u32, flag: GhosttyRowData) -> Result<bool> {
        let point = GhosttyPoint {
            tag: GHOSTTY_POINT_TAG_VIEWPORT,
            value: GhosttyPointValue {
                coordinate: GhosttyPointCoordinate { x: 0, y: row },
            },
        };
        // SAFETY: the reference is used immediately without terminal mutation.
        unsafe {
            let mut grid = GhosttyGridRef::default();
            let mut raw = 0;
            let mut value = false;
            check(ghostty_terminal_grid_ref(self.terminal, point, &mut grid))?;
            check(ghostty_grid_ref_row(&grid, &mut raw))?;
            check(ghostty_row_get(raw, flag, (&mut value as *mut bool).cast()))?;
            Ok(value)
        }
    }

    pub fn hyperlink_at(&mut self, column: u16, row: u32) -> Result<Option<String>> {
        if self.frame.revision != self.revision {
            return Ok(None);
        }
        if column >= self.frame.columns || row as usize >= self.frame.rows.len() {
            return Ok(None);
        }
        let point = GhosttyPoint {
            tag: GHOSTTY_POINT_TAG_VIEWPORT,
            value: GhosttyPointValue {
                coordinate: GhosttyPointCoordinate { x: column, y: row },
            },
        };
        // SAFETY: the borrowed grid reference is read under exclusive engine
        // access and never survives a mutation or escapes this call.
        unsafe {
            let mut grid = GhosttyGridRef::default();
            check(ghostty_terminal_grid_ref(self.terminal, point, &mut grid))?;
            let mut len = 0;
            let result = ghostty_grid_ref_hyperlink_uri(&grid, ptr::null_mut(), 0, &mut len);
            if result != GHOSTTY_OUT_OF_SPACE {
                check(result)?;
            }
            if len == 0 {
                return Ok(None);
            }
            ensure!(len <= 8192, "hyperlink exceeds 8 KiB");
            let mut bytes = vec![0; len];
            check(ghostty_grid_ref_hyperlink_uri(
                &grid,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut len,
            ))?;
            bytes.truncate(len);
            Ok(Some(String::from_utf8(bytes)?))
        }
    }
}

fn plain_url_at(text: &str, clicked: usize) -> Option<&str> {
    let delimiter =
        |c: char| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\'');
    for (start, _) in text.char_indices().filter(|(_, c)| matches!(c, 'h' | 'H' | 'f' | 'F')) {
        let rest = &text[start..];
        let file = rest.get(..7).is_some_and(|s| s.eq_ignore_ascii_case("file://"));
        let prefix = rest
            .get(..8)
            .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
            || rest
                .get(..7)
                .is_some_and(|s| s.eq_ignore_ascii_case("http://"));
        if !prefix && !file {
            continue;
        }
        let end = rest.find(delimiter).unwrap_or(rest.len());
        let mut uri = rest[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
            while uri.ends_with(close) && uri.matches(close).count() > uri.matches(open).count() {
                uri = &uri[..uri.len() - close.len_utf8()];
            }
        }
        let authority = uri.split_once("://")?.1.split(['/', '?', '#']).next()?;
        if (!authority.is_empty() || file && uri.len() > 8) && clicked >= start && clicked < start + uri.len() {
            return Some(uri);
        }
    }
    None
}

// Compiler coordinates are numeric data, not a general parenthesized token.
fn parenthesized_location_len(text: &str) -> Option<usize> {
    let rest = text.strip_prefix('(')?;
    let end = rest.find(')')?;
    let parts = rest[..end].split(',');
    ((1..=2).contains(&parts.clone().count()) && parts.clone().all(|part|
        !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())))
        .then_some(end + 2)
}

// Recognize path-shaped tokens only; existence and local/remote routing belong
// to the host. This runs on demand, never in the normal terminal draw path.
fn plain_path_at(text: &str, clicked: usize) -> Option<&str> {
    // Quotes delimit a single literal path, not shell syntax. Do not fall back
    // to a misleading partial token inside an unclosed or invalid quoted span.
    let mut quote = None;
    for (offset, character) in text.char_indices() {
        if let Some((start, delimiter)) = quote {
            if character == delimiter {
                let after_quote = offset + character.len_utf8();
                let quoted_path = &text[start..offset];
                if quoted_path.rsplit(['/', '\\']).next().is_some_and(|name| name.contains('.'))
                    && let Some(length) = parenthesized_location_len(&text[after_quote..]) {
                    let begin = start - delimiter.len_utf8();
                    let end = after_quote + length;
                    if clicked >= begin && clicked < end { return Some(&text[begin..end]); }
                }
                let suffix = &text[after_quote..];
                let suffix = &suffix[..suffix.find(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ')' | ']' | '}')).unwrap_or(suffix.len())];
                let suffix = suffix.trim_end_matches(':');
                let coordinates = suffix.split(':').skip(1);
                let candidate = &text[start..offset];
                if suffix.starts_with(':') && (1..=2).contains(&coordinates.clone().count())
                    && coordinates.clone().all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
                    && (path_shaped(candidate) || candidate.contains('.')) {
                    let begin = start - delimiter.len_utf8();
                    let end = after_quote + suffix.len();
                    if clicked >= begin && clicked < end { return Some(&text[begin..end]); }
                }
                if clicked >= start && clicked < offset {
                    let candidate = &text[start..offset];
                    return path_shaped(candidate).then_some(candidate);
                }
                quote = None;
            }
        } else if matches!(character, '\'' | '"')
            && (offset == 0 || text[..offset].chars().next_back().is_some_and(|c| c.is_whitespace() || matches!(c, '=' | ':' | '(' | '['))) {
            quote = Some((offset + character.len_utf8(), character));
        }
    }
    if quote.is_some_and(|(start, _)| clicked >= start) { return None; }
    let delimiter = |c: char| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"' | '\'' | '(' | ')' | '[' | ']' | '{' | '}');
    let mut start = 0;
    for token in text.split_inclusive(delimiter) {
        let candidate = token.trim_end_matches(delimiter).trim_end_matches([',', ';', ':']);
        if candidate.rsplit(['/', '\\']).next().is_some_and(|name| name.contains('.'))
            && !candidate.contains("://")
            && let Some(length) = parenthesized_location_len(&text[start + candidate.len()..]) {
            let end = start + candidate.len() + length;
            if clicked >= start && clicked < end { return Some(&text[start..end]); }
        }
        if clicked >= start && clicked < start + candidate.len() {
            return path_shaped(candidate).then_some(candidate);
        }
        start += token.len();
    }
    None
}

fn path_shaped(candidate: &str) -> bool {
    if candidate.contains("://") || candidate.starts_with('-') || candidate.chars().any(char::is_control) { return false; }
    let has_location = candidate.rsplit_once(':').is_some_and(|(name, line)|
        name.contains('.') && !line.is_empty() && line.bytes().all(|c| c.is_ascii_digit()));
    candidate.contains('/') || candidate.contains('\\') || has_location
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiler_parenthesized_coordinates_are_part_of_the_link() {
        for target in ["src/main.ts(12,3)", "main.cs(42)", "\"src/my file.ts\"(12,3)"] {
            assert_eq!(plain_path_at(target, 2), Some(target));
            assert_eq!(plain_path_at(target, target.len() - 2), Some(target));
        }
        assert_eq!(plain_path_at("function(12,3)", 10), None);
        assert_eq!(plain_path_at("src/file.rs(draft)", 13), None);
        assert_eq!(plain_path_at("src/file.rs(12,3,4)", 14), None);
    }
    #[test]
    fn quoted_diagnostic_locations_include_external_coordinates() {
        for text in [r#""src/my file.rs":12:3"#, "'main.rs':27", r#""~/my file.rs":8"#] {
            for clicked in [1, text.len() - 1] {
                assert_eq!(plain_path_at(text, clicked), Some(text));
            }
        }
        let wrapped = r#"("src/my file.rs":12:3)"#;
        assert_eq!(plain_path_at(wrapped, wrapped.rfind('3').unwrap()), Some(r#""src/my file.rs":12:3"#));
        assert_eq!(plain_path_at("'ordinary words':12", 18), None);
    }
    #[test]
    fn quoted_paths_preserve_spaces_punctuation_and_wide_cells() -> Result<()> {
        for text in ["File \"/tmp/my project/世界 (draft).rs\"", "error 'src/my file.rs:42:7'"] {
            let clicked = text.find("my").unwrap();
            let expected = text.trim_start_matches("File ").trim_start_matches("error ").trim_matches(['\'', '"']);
            assert_eq!(plain_path_at(text, clicked), Some(expected));
        }
        assert_eq!(plain_path_at("File \"/tmp/unfinished path", 12), None);
        assert_eq!(plain_path_at("'ordinary words'", 4), None);
        assert_eq!(plain_path_at("can't read src/a.rs", 14), Some("src/a.rs"));
        assert_eq!(plain_path_at("'src/a.rs' src/b.rs:12", 15), Some("src/b.rs:12"));
        let mut terminal = Terminal::new(20, 4)?;
        terminal.write("File \"/tmp/my project/世界 (draft).rs\"".as_bytes());
        terminal.frame()?;
        assert_eq!(terminal.link_at(12, 0)?.as_deref(), Some("/tmp/my project/世界 (draft).rs"));
        assert_eq!(terminal.link_at(2, 1)?.as_deref(), Some("/tmp/my project/世界 (draft).rs"));
        Ok(())
    }
    #[test]
    fn compiler_paths_follow_wrapped_cells_and_keep_url_priority() -> Result<()> {
        let mut terminal = Terminal::new(20, 5)?;
        terminal.write("  --> src/世界/main.rs:42:7\r\n".as_bytes());
        terminal.frame()?;
        assert_eq!(terminal.link_at(9, 0)?.as_deref(), Some("src/世界/main.rs:42:7"));
        assert_eq!(terminal.link_at(1, 1)?.as_deref(), Some("src/世界/main.rs:42:7"));
        for text in ["src/main.rs:42:7:", "(src/main.rs:42:7)", "[src/main.rs:42:7]"] {
            let offset = text.find("main").unwrap();
            assert_eq!(plain_path_at(text, offset), Some("src/main.rs:42:7"));
        }
        assert_eq!(plain_path_at("main.rs:42", 2), Some("main.rs:42"));
        assert_eq!(plain_path_at("some words", 2), None);
        assert_eq!(plain_path_at("12:30", 2), None);
        assert_eq!(plain_path_at("https://example.com/x.rs:42", 10), None);
        Ok(())
    }
    #[test]
    fn file_urls_follow_visible_cells_and_leave_routing_to_the_host() -> Result<()> {
        let mut terminal = Terminal::new(24, 4)?;
        terminal.write(b"file:///tmp/a%20b.txt");
        terminal.frame()?;
        assert_eq!(terminal.link_at(12, 0)?.as_deref(), Some("file:///tmp/a%20b.txt"));
        assert_eq!(plain_url_at("file://remote/tmp/a", 10), Some("file://remote/tmp/a"));
        assert_eq!(plain_url_at("file://", 2), None);
        assert_eq!(plain_url_at("https:///missing-host", 5), None);
        Ok(())
    }
    #[test]
    fn links_wait_for_a_matching_frame_after_output_or_viewport_changes() -> Result<()> {
        let mut terminal = Terminal::new(40, 3)?;
        terminal.write(b"https://old.test");
        let old = terminal.frame()?;
        assert_eq!(terminal.link_at(0, 0)?.as_deref(), Some("https://old.test"));
        terminal.write(b"\x1b[?2026h\r\x1b[2Khttps://new.test");
        assert_eq!(terminal.frame()?.revision, old.revision);
        assert_eq!(terminal.link_at(0, 0)?, None);
        terminal.write(b"\x1b[?2026l");
        let new = terminal.frame()?;
        assert_ne!(new.revision, old.revision);
        assert_eq!(terminal.link_at(0, 0)?.as_deref(), Some("https://new.test"));
        terminal.scroll(Scroll::Top)?;
        assert_eq!(terminal.link_at(0, 0)?, None);
        terminal.frame()?;
        assert!(terminal.link_at(0, 0)?.is_some());
        terminal.resize(50, 3)?;
        assert_eq!(terminal.link_at(0, 0)?, None);
        terminal.frame()?;
        assert!(terminal.link_at(0, 0)?.is_some());
        Ok(())
    }
    #[test]
    fn plain_links_follow_cells_wraps_and_explicit_link_precedence() -> Result<()> {
        let mut terminal = Terminal::new(20, 5)?;
        terminal.write("界 (https://example.com/a_(b)). next".as_bytes());
        terminal.frame()?;
        assert_eq!(terminal.link_at(0, 0)?, None);
        assert_eq!(
            terminal.link_at(6, 0)?.as_deref(),
            Some("https://example.com/a_(b)")
        );
        assert_eq!(
            terminal.link_at(2, 1)?.as_deref(),
            Some("https://example.com/a_(b)")
        );
        terminal.write(b"\r\n\x1b]8;;https://explicit.test\x1b\\https://shown.test\x1b]8;;\x1b\\");
        terminal.frame()?;
        assert_eq!(
            terminal.link_at(3, 2)?.as_deref(),
            Some("https://explicit.test")
        );
        let mut clipped = Terminal::new(10, 2)?;
        clipped.write(b"https://example.com/very-long-path-more");
        clipped.frame()?;
        assert_eq!(clipped.link_at(0, 0)?, None);
        clipped.scroll(Scroll::Top)?;
        clipped.frame()?;
        assert_eq!(clipped.link_at(0, 0)?, None);
        assert_eq!(
            plain_url_at("(https://example.com/a_(b)).", 5),
            Some("https://example.com/a_(b)")
        );
        assert_eq!(plain_url_at("https:///missing-host", 5), None);
        assert_eq!(plain_url_at("https://example.com next", 21), None);
        Ok(())
    }
    #[test]
    fn hyperlink_lookup_tracks_viewport_and_active_screen() -> Result<()> {
        let mut terminal = Terminal::new(30, 3)?;
        terminal
            .write(b"\x1b]8;;https://example.com/a\x1b\\LINK\x1b]8;;\x1b\\ plain\r\n2\r\n3\r\n4");
        terminal.frame()?;
        assert_eq!(terminal.hyperlink_at(0, 0)?, None);
        terminal.scroll(Scroll::Top)?;
        terminal.frame()?;
        assert_eq!(
            terminal.hyperlink_at(0, 0)?.as_deref(),
            Some("https://example.com/a")
        );
        assert_eq!(terminal.hyperlink_at(5, 0)?, None);
        assert_eq!(terminal.hyperlink_at(30, 0)?, None);
        terminal.write(b"\x1b[?1049hALT");
        terminal.frame()?;
        assert_eq!(terminal.hyperlink_at(0, 0)?, None);
        Ok(())
    }

    #[test]
    fn hyperlink_lookup_handles_unicode_and_bounds_uri_allocation() -> Result<()> {
        let mut terminal = Terminal::new(30, 3)?;
        terminal.write("\x1b]8;;https://example.com/日本\x1b\\界\x1b]8;;\x1b\\".as_bytes());
        terminal.frame()?;
        for column in [0, 1] {
            assert_eq!(
                terminal.hyperlink_at(column, 0)?.as_deref(),
                Some("https://example.com/日本")
            );
        }
        assert_eq!(terminal.hyperlink_at(2, 0)?, None);
        assert_eq!(terminal.hyperlink_at(0, 3)?, None);
        let oversized = format!(
            "\r\n\x1b]8;;https://example.com/{}\x1b\\X\x1b]8;;\x1b\\",
            "a".repeat(8192)
        );
        terminal.write(oversized.as_bytes());
        terminal.frame()?;
        // Ghostty may reject the oversized OSC before it reaches the grid.
        // If a backend version retains it, our lookup must reject it instead.
        match terminal.hyperlink_at(0, 1) {
            Ok(None) => {}
            Err(error) => assert!(error.to_string().contains("8 KiB")),
            Ok(Some(_)) => panic!("oversized hyperlink escaped the URI bound"),
        }
        Ok(())
    }
}
