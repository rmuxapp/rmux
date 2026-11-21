//! In-memory demo session service. Emits terminal output without owning a view
//! or terminal parser; deliberately does not execute commands on the host.
use crate::{sample_input::SampleInput, terminal_text::Grapheme};

#[derive(Default)]
pub struct SampleSession {
    draft: SampleInput,
}
impl SampleSession {
    pub fn display(&self, columns: u16) -> String { self.draft.display(columns) }

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
            "\x7f" | "\x08" => self.draft.backspace(),
            "\x03" => {
                output.push_str(&format!("\r\x1b[2K$ {}^C\r\n", self.draft.text()));
                self.draft.clear();
            }
            "\x15" => self.draft.clear(),
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
}
