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
            .and_then(|offset| plain_url_at(&text, offset))
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

#[cfg(test)]
mod tests {
    use super::*;
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
