//! Exact segmentation/width for terminal text with grapheme mode 2027 enabled.
use crate::ffi::ghostty_unicode_grapheme_width;
use anyhow::{Result, ensure};
use rmux_core::terminal_text::{Grapheme, MAX_MEASURE_BYTES};

pub fn graphemes(text: &str) -> Result<Vec<Grapheme>> {
    ensure!(
        text.len() <= MAX_MEASURE_BYTES,
        "text exceeds 1 MiB measurement limit"
    );
    ensure!(
        !text.chars().any(char::is_control),
        "terminal text must not contain controls"
    );
    let codepoints: Vec<u32> = text.chars().map(u32::from).collect();
    let mut result = Vec::new();
    let (mut offset, mut end) = (0, 0);
    while offset < codepoints.len() {
        let mut width = 0;
        // SAFETY: the remaining initialized scalar slice and width output live
        // throughout this pure call; Ghostty does not retain either pointer.
        let consumed = unsafe {
            ghostty_unicode_grapheme_width(
                codepoints[offset..].as_ptr(),
                codepoints.len() - offset,
                &mut width,
            )
        };
        ensure!(
            consumed > 0 && consumed <= codepoints.len() - offset && width <= 2,
            "invalid grapheme measurement"
        );
        for &cp in &codepoints[offset..offset + consumed] {
            end += char::from_u32(cp).unwrap().len_utf8();
        }
        offset += consumed;
        result.push(Grapheme { end, width });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sample_input_handles_split_unicode_commits_and_narrow_panes() -> Result<()> {
        use rmux_core::sample_input::{SampleInput, MAX_INPUT_BYTES};
        let mut input = SampleInput::default();
        let measure = |text: &str| graphemes(text).ok();
        for text in ["echo ", "e", "\u{301}", "界", "👩", "\u{200d}", "💻"] {
            input.append(text, measure).unwrap();
        }
        assert_eq!(input.text(), "echo e\u{301}界👩‍💻");
        assert_eq!(input.display(7), "\r\x1b[2K$ 界👩‍💻");
        input.backspace();
        assert_eq!(input.text(), "echo e\u{301}界");
        input.backspace(); input.backspace();
        assert_eq!(input.text(), "echo ");
        assert!(input.append("\n", measure).is_err());
        assert!(input.append(&"x".repeat(MAX_INPUT_BYTES), measure).is_err());
        assert_eq!(input.text(), "echo ");
        input.clear();
        assert_eq!(input.display(1), "\r\x1b[2K");
        assert_eq!(input.display(2), "\r\x1b[2K$");
        assert!(input.append("bad", |_| Some(vec![])).is_err());
        assert_eq!(input.text(), "");
        Ok(())
    }

    #[test]
    fn engine_graphemes_cover_wide_combining_and_emoji() -> Result<()> {
        assert_eq!(
            graphemes("a界e\u{301}👩‍💻")?,
            vec![
                Grapheme { end: 1, width: 1 },
                Grapheme { end: 4, width: 2 },
                Grapheme { end: 7, width: 1 },
                Grapheme { end: 18, width: 2 }
            ]
        );
        for text in ["👍🏽", "🇮🇪", "❤️"] {
            assert_eq!(
                graphemes(text)?,
                vec![Grapheme {
                    end: text.len(),
                    width: 2
                }]
            );
        }
        assert!(graphemes("")?.is_empty());
        assert!(graphemes("a\nb").is_err());
        assert!(graphemes(&"é".repeat(MAX_MEASURE_BYTES / 2 + 1)).is_err());
        Ok(())
    }
}
