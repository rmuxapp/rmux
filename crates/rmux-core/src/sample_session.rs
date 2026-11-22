//! In-memory demo session service. Emits terminal output without owning a view
//! or terminal parser; deliberately does not execute commands on the host.
use crate::{sample_input::SampleInput, terminal_text::Grapheme};

#[derive(Clone, Default)]
pub struct SampleSession {
    draft: SampleInput,
}
impl SampleSession {
    pub fn display(&self, columns: u16) -> String { self.draft.display(columns) }

    /// Consume unbracketed terminal paste bytes transactionally. The sample
    /// service has no bracketed-paste negotiation; real shell backends own that.
    pub fn paste(&mut self, text: &str, columns: u16,
        mut measure: impl FnMut(&str) -> Option<Vec<Grapheme>>) -> Result<String, &'static str> {
        if text.len() > crate::sample_input::MAX_INPUT_BYTES {
            return Err("Sample paste is limited to 4 KiB.");
        }
        if text.chars().any(|c| c.is_control() && c != '\r' && c != '\n') {
            return Err("Sample paste contains unsupported control characters.");
        }
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        let mut pending = self.clone();
        let mut output = String::new();
        for (index, line) in normalized.split('\n').enumerate() {
            if index > 0 { output.push_str(&pending.input("\r", columns, &mut measure)?.unwrap()); }
            if let Some(echo) = pending.input(line, columns, &mut measure)? { output.push_str(&echo); }
            if output.len() > 1024 * 1024 { return Err("Sample paste produces too much output."); }
        }
        *self = pending;
        Ok(output)
    }

    /// None means an unsupported/empty input event; errors preserve the draft.
    pub fn input(&mut self, input: &str, columns: u16,
        measure: impl FnOnce(&str) -> Option<Vec<Grapheme>>) -> Result<Option<String>, &'static str> {
        let mut output = String::new();
        match input {
            "\r" => {
                output.push_str(&format!("\r\x1b[2K$ {}\r\n", self.draft.text()));
                if let Some(text) = self.draft.text().strip_prefix("echo ") {
                    output.push_str(text);
                    output.push_str("\r\n");
                }
                if self.draft.text().trim() == "logs" {
                    for line in 1..=80 { output.push_str(&format!("Sample log {line:03}: workspace ready\r\n")); }
                }
                output.push_str(match self.draft.text().trim() {
                    "help" => "Sample commands: help, clear, ls, logs, echo <text>\r\nNo shell or server access.\r\n",
                    "logs" | "" => "",
                    "ls" => "\x1b[34mcrates/  docs/  scripts/\x1b[0m  Cargo.toml\r\n",
                    "clear" => "\x1b[2J\x1b[H",
                    _ if self.draft.text().starts_with("echo ") => "",
                    _ => "Unknown sample command. Try help.\r\n",
                });
                self.draft.clear();
            }
            "\x7f" | "\x08" => self.draft.delete(true, measure)?,
            "\x1b[3~" | "\x04" => self.draft.delete(false, measure)?,
            "\x1b[D" | "\x1bOD" | "\x02" => self.draft.move_cursor(true, false),
            "\x1b[C" | "\x1bOC" | "\x06" => self.draft.move_cursor(false, false),
            "\x1bb" | "\x1b[1;3D" | "\x1b[1;5D" => self.draft.move_cursor(true, true),
            "\x1bf" | "\x1b[1;3C" | "\x1b[1;5C" => self.draft.move_cursor(false, true),
            "\x01" | "\x1b[H" | "\x1bOH" | "\x1b[1~" => self.draft.edge(false),
            "\x05" | "\x1b[F" | "\x1bOF" | "\x1b[4~" => self.draft.edge(true),
            "\x03" => {
                output.push_str(&format!("\r\x1b[2K$ {}^C\r\n", self.draft.text()));
                self.draft.clear();
            }
            "\x15" => self.draft.erase_to_edge(false, measure)?,
            "\x0b" => self.draft.erase_to_edge(true, measure)?,
            text => {
                if text.is_empty() || text.chars().any(char::is_control) { return Ok(None); }
                self.draft.append(text, measure)?;
            }
        }
        output.push_str(&self.display(columns));
        Ok(Some(output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn send(session: &mut SampleSession, input: &str) -> String {
        session.input(input, 80, |_| panic!("ASCII requires no measurement")).unwrap().unwrap()
    }
    #[test]
    fn cursor_editing_and_word_navigation_preserve_the_command() {
        let mut session = SampleSession::default();
        send(&mut session, "echo alpha beta");
        send(&mut session, "\x1bb");
        send(&mut session, "new ");
        send(&mut session, "\x1bf");
        send(&mut session, "!");
        assert!(send(&mut session, "\r").contains("\r\nalpha new beta!\r\n"));
        send(&mut session, "echo old");
        send(&mut session, "\x01");
        for _ in 0..5 { send(&mut session, "\x1b[C"); }
        send(&mut session, "\x1b[3~");
        send(&mut session, "\x05");
        send(&mut session, "\x7f");
        assert!(send(&mut session, "\r").contains("\r\nl\r\n"));
        send(&mut session, "discard echo keep discard");
        send(&mut session, "\x01");
        for _ in 0..8 { send(&mut session, "\x06"); }
        send(&mut session, "\x15");
        send(&mut session, "\x05");
        send(&mut session, "\x1bb");
        send(&mut session, "\x0b");
        assert!(send(&mut session, "\r").contains("\r\nkeep \r\n"));
    }
    #[test]
    fn sessions_keep_independent_drafts_and_interrupt_without_execution() {
        let mut first = SampleSession::default();
        let mut second = SampleSession::default();
        send(&mut first, "echo first");
        send(&mut second, "echo second");
        assert_eq!(send(&mut first, "\r"), "\r\x1b[2K$ echo first\r\nfirst\r\n\r\x1b[2K$ ");
        assert!(send(&mut second, "\x03").contains("echo second^C\r\n"));
        assert_eq!(second.display(80), "\r\x1b[2K$ ");
        assert!(second.input("\x1b[A", 80, |_| None).unwrap().is_none());
        send(&mut second, "logs");
        let logs = send(&mut second, "\r");
        assert_eq!(logs.matches("Sample log ").count(), 80);
    }
    #[test]
    fn multiline_paste_normalizes_line_endings_and_keeps_final_draft() {
        let mut session = SampleSession::default();
        send(&mut session, "echo ");
        let output = session.paste("one\r\necho two\recho three\necho draft", 80, |_| None).unwrap();
        for line in ["one", "two", "three"] { assert!(output.contains(&format!("\r\n{line}\r\n"))); }
        assert_eq!(session.display(80), "\r\x1b[2K$ echo draft");
        assert!(session.paste("\x03", 80, |_| None).is_err());
        assert!(session.paste(&"x".repeat(4097), 80, |_| None).is_err());
        assert!(session.paste(&"logs\n".repeat(800), 80, |_| None).is_err());
        assert_eq!(session.display(80), "\r\x1b[2K$ echo draft");
    }
}
