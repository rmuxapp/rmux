//! Private opt-in shell history OSC. Payload is UTF-8 encoded as hex, never code.
pub(crate) const PREFIX: &[u8] = b"9999;rmux-history;";
pub(crate) const CAPTURE_LIMIT: usize = PREFIX.len() + rmux_core::shell_history::COMMAND_LIMIT * 2;
pub(crate) enum Report { Command(String), Clear }
pub(crate) fn decode(bytes: &[u8]) -> Option<Report> {
    let text = bytes.strip_prefix(PREFIX)?;
    if text == b"clear" { return Some(Report::Clear); }
    if text.is_empty() || text.len() % 2 != 0 || text.len() > rmux_core::shell_history::COMMAND_LIMIT * 2 { return None; }
    let nibble = |byte: u8| match byte {
        b'0'..=b'9' => Some(byte - b'0'), b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10), _ => None,
    };
    let mut decoded = Vec::with_capacity(text.len() / 2);
    for pair in text.chunks_exact(2) { decoded.push(nibble(pair[0])? * 16 + nibble(pair[1])?); }
    String::from_utf8(decoded).ok().filter(|text| !text.contains('\0')).map(Report::Command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Terminal;
    #[test]
    fn shell_history_reports_survive_every_chunk_split_without_screen_output() -> anyhow::Result<()> {
        let command = "printf '%s\\n' '世界'\necho second";
        let encoded: String = command.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect();
        let report = format!("\x1b]9999;rmux-history;{encoded}\x07");
        for split in 0..=report.len() {
            let mut terminal = Terminal::new(40, 2)?;
            terminal.write(&report.as_bytes()[..split]);
            terminal.write(&report.as_bytes()[split..]);
            let (revision, history) = terminal.shell_history_since(0).expect("history report");
            assert_eq!(history.entries().collect::<Vec<_>>(), vec![command], "split {split}");
            assert!(terminal.shell_history_since(revision).is_none());
            terminal.write(report.as_bytes());
            assert!(terminal.shell_history_since(revision).is_none(), "duplicate should not dirty history");
            assert!(!terminal.frame()?.text().contains("rmux-history"));
        }
        Ok(())
    }
    #[test]
    fn malformed_history_is_rejected_and_does_not_poison_following_output() -> anyhow::Result<()> {
        let mut terminal = Terminal::new(40, 2)?;
        for payload in ["0", "zz", "ff", "00"] {
            terminal.write(format!("\x1b]9999;rmux-history;{payload}\x07").as_bytes());
        }
        terminal.write(format!("\x1b]9999;rmux-history;{}\x07", "61".repeat(8193)).as_bytes());
        assert!(terminal.shell_history_since(0).is_none());
        terminal.write(b"\x1b]9999;rmux-history;6563686f206f6b\x1b\\visible");
        assert_eq!(terminal.shell_history_since(0).unwrap().1.entries().next(), Some("echo ok"));
        assert!(terminal.frame()?.text().contains("visible"));
        assert!(decode(b"99;rmux-history;61").is_none());
        Ok(())
    }
}

#[cfg(test)]
mod clear_tests {
    use crate::Terminal;
    #[test]
    fn clear_reports_drop_old_records_and_allow_same_command_again() -> anyhow::Result<()> {
        let clear = b"\x1b]9999;rmux-history;clear\x07";
        for split in 0..=clear.len() {
            let mut terminal = Terminal::new(40, 2)?;
            terminal.write(b"\x1b]9999;rmux-history;6563686f206f6b\x07");
            let before = terminal.shell_history_since(0).unwrap().0;
            terminal.write(&clear[..split]);
            terminal.write(&clear[split..]);
            let (cleared, history) = terminal.shell_history_since(before).unwrap();
            assert_eq!(history.entries().len(), 0);
            assert_eq!(history.text_bytes(), 0);
            terminal.write(clear);
            assert!(terminal.shell_history_since(cleared).is_none());
            terminal.write(b"\x1b]9999;rmux-history;6563686f206f6b\x07");
            assert_eq!(terminal.shell_history_since(cleared).unwrap().1.entries().next(), Some("echo ok"));
        }
        Ok(())
    }
}
