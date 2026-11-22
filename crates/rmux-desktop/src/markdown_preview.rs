//! Bounded, inert CommonMark reading blocks. No HTML/web engine or asset loads.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
pub use pulldown_cmark::Alignment;

/// Resolve links without filesystem access. Local targets stay inside rmux's
/// bounded preview loader; no file URL is handed to the operating system.
pub fn link_destination(uri: &str, source: &std::path::Path) -> Option<String> {
    if uri.is_empty() || uri.len() > 8192 || uri.chars().any(char::is_control) { return None; }
    if super::openable_uri(uri) { return Some(uri.to_owned()); }
    if uri.starts_with('#') || uri.starts_with("//") || uri.contains('\\') { return None; }
    let base = url::Url::from_file_path(source).ok()?;
    let target = base.join(uri).ok()?;
    super::terminal_link::local_file(target.as_str(), false)?;
    Some(target.into())
}
/// A fragment stays inside the current preview; it is never an OS URL.
pub fn fragment(uri: &str) -> Option<String> {
    if uri.len() > 8192 { return None; }
    let raw = uri.strip_prefix('#')?;
    let mut bytes = Vec::with_capacity(raw.len());
    let mut input = raw.bytes();
    while let Some(byte) = input.next() {
        bytes.push(if byte == b'%' {
            let high = (input.next()? as char).to_digit(16)?;
            let low = (input.next()? as char).to_digit(16)?;
            (high * 16 + low) as u8
        } else { byte });
    }
    let value = String::from_utf8(bytes).ok()?;
    (!value.chars().any(char::is_control)).then_some(value)
}

pub fn heading_targets<'a>(headings: impl Iterator<Item = (usize, &'a str)>) -> std::collections::HashMap<String, usize> {
    let mut targets = std::collections::HashMap::new();
    for (index, text) in headings {
        let base: String = text.to_lowercase().chars().filter_map(|ch| {
            if ch.is_whitespace() { Some('-') }
            else if ch.is_alphanumeric() || ch == '_' || ch == '-' { Some(ch) }
            else { None }
        }).collect();
        let mut id = base.clone();
        let mut suffix = 0;
        while targets.contains_key(&id) { suffix += 1; id = format!("{base}-{suffix}"); }
        targets.insert(id, index);
    }
    targets
}

fn supported_link(uri: &str) -> bool {
    if fragment(uri).is_some() { return true; }
    // The source path is supplied when the preview is assembled. Use a local
    // absolute base here only to classify and reject unsupported destinations.
    let base = if cfg!(windows) { "C:/rmux/document.md" } else { "/rmux/document.md" };
    link_destination(uri, std::path::Path::new(base)).is_some()
}

pub const TABLE_VISIBLE_COLUMNS: usize = 8;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Kind { #[default] Paragraph, Heading(u8), Code, Quote, Table { header: bool, columns: usize, alignment: [Alignment; TABLE_VISIBLE_COLUMNS] }, List { depth: usize, number: Option<u64>, marker: bool, checked: Option<bool> } }
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InlineStyle { pub bold: bool, pub italic: bool, pub code: bool, pub strike: bool, pub link: Option<usize> }
#[derive(Clone, Debug, PartialEq)]
pub struct Span { pub range: std::ops::Range<usize>, pub style: InlineStyle }
pub struct Block { pub text: String, pub source_line: u32, pub kind: Kind, pub spans: Vec<Span>, pub links: Vec<String> }
pub fn blocks(source: &str) -> (Vec<Block>, bool) {
    let mut output = Vec::new();
    let mut text = String::new();
    let mut kind = Kind::Paragraph;
    let mut spans: Vec<Span> = Vec::new();
    let mut links: Vec<String> = Vec::new();
    let mut link = None;
    let mut link_bytes = 0usize;
    let (mut bold, mut italic) = (0usize, 0usize);
    let mut strike = 0usize;
    let mut table_columns = 0usize;
    let mut table_alignment = [Alignment::None; TABLE_VISIBLE_COLUMNS];
    let mut table_cell = 0usize;
    let mut quote = 0usize;
    let mut lists: Vec<Option<u64>> = Vec::new();
    let mut items: Vec<Kind> = Vec::new();
    let line_starts: Vec<usize> = std::iter::once(0).chain(source.bytes().enumerate()
        .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1))).collect();
    let block_offset = std::cell::Cell::new(0);
    let source_line = |offset| line_starts.partition_point(|start| *start <= offset) as u32;
    let flush = |output: &mut Vec<Block>, text: &mut String, spans: &mut Vec<Span>, links: &mut Vec<String>, kind| {
        if text.trim().is_empty() && !matches!(kind, Kind::Table { .. }) { text.clear(); spans.clear(); links.clear(); return false; }
        if output.len() >= 512 { return true; }
        output.push(Block { source_line: source_line(block_offset.get()), text: std::mem::take(text), kind, spans: std::mem::take(spans), links: std::mem::take(links) });
        false
    };
    for (event, range) in Parser::new_ext(source, Options::ENABLE_TASKLISTS | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES).into_offset_iter() {
        if text.is_empty() { block_offset.set(range.start); }
        match event {
            Event::Start(Tag::Table(columns)) => {
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); }
                table_columns = columns.len();
                table_alignment = [Alignment::None; TABLE_VISIBLE_COLUMNS];
                for (target, value) in table_alignment.iter_mut().zip(columns) { *target = value; }
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                let header = matches!(event, Event::Start(Tag::TableHead));
                kind = Kind::Table { header, columns: table_columns, alignment: table_alignment };
                table_cell = 0;
            }
            Event::Start(Tag::TableCell) => {
                if table_cell > 0 { text.push('\t'); }
                table_cell += 1;
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); }
            }
            Event::End(TagEnd::Table) => {
                table_columns = 0;
                kind = items.last().copied().unwrap_or(if quote > 0 { Kind::Quote } else { Kind::Paragraph });
            }
            Event::Start(Tag::BlockQuote(_)) => quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote = quote.saturating_sub(1),
            Event::Start(Tag::List(start)) => {
                if !text.trim().is_empty() && let Some(Kind::List { marker, .. }) = items.last_mut() { *marker = false; }
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); }
                lists.push(start);
            }
            Event::End(TagEnd::List(_)) => { lists.pop(); }
            Event::Start(Tag::Item) => {
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); }
                let number = lists.last().copied().flatten();
                kind = Kind::List { depth: lists.len(), number, marker: true, checked: None };
                items.push(kind);
                if let Some(Some(next)) = lists.last_mut() { *next = next.saturating_add(1); }
            }
            Event::End(TagEnd::Item) => {
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); }
                items.pop();
                kind = items.last().copied().unwrap_or(if quote > 0 { Kind::Quote } else { Kind::Paragraph });
            }
            Event::Start(tag @ (Tag::HtmlBlock | Tag::Paragraph | Tag::Heading { .. } | Tag::CodeBlock(_))) => {
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); }
                kind = match tag { Tag::Heading { level, .. } => Kind::Heading(level as u8), Tag::CodeBlock(_) => Kind::Code,
                    _ if !items.is_empty() => *items.last().unwrap(), _ if quote > 0 => Kind::Quote, _ => Kind::Paragraph };
                if let Some(Kind::List { marker, .. }) = items.last_mut() { *marker = false; }
            }
            Event::End(TagEnd::HtmlBlock | TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::CodeBlock) => { if flush(&mut output, &mut text, &mut spans, &mut links, kind) { return (output, true); } },
            Event::Start(Tag::Link { dest_url, .. }) => {
                link = None;
                if supported_link(&dest_url) {
                    link = links.iter().position(|url| url == dest_url.as_ref());
                    // Reference links can repeat a long destination many times.
                    // Bound retained URLs across the whole document, not only each row.
                    if link.is_none() && links.len() < 64 && link_bytes + dest_url.len() <= 65536 {
                        link_bytes += dest_url.len();
                        link = Some(links.len());
                        links.push(dest_url.into_string());
                    }
                }
            }
            Event::End(TagEnd::Link) => link = None,
            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold = bold.saturating_sub(1),
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic = italic.saturating_sub(1),
            Event::Start(Tag::Strikethrough) => strike += 1,
            Event::End(TagEnd::Strikethrough) => strike = strike.saturating_sub(1),
            event @ (Event::Text(_) | Event::Code(_) | Event::Html(_) | Event::InlineHtml(_)) => {
                let code = matches!(event, Event::Code(_));
                let (Event::Text(value) | Event::Code(value) | Event::Html(value) | Event::InlineHtml(value)) = event else { unreachable!() };
                let start = text.len();
                if matches!(kind, Kind::Table { .. }) { text.extend(value.chars().map(|ch| if ch == '\t' { ' ' } else { ch })); }
                else { text.push_str(&value); }
                if start < text.len() && (bold > 0 || italic > 0 || strike > 0 || code || link.is_some()) {
                    let style = InlineStyle { bold: bold > 0, italic: italic > 0, code, strike: strike > 0, link };
                    if let Some(previous) = spans.last_mut() && previous.range.end == start && previous.style == style { previous.range.end = text.len(); }
                    else { spans.push(Span { range: start..text.len(), style }); }
                }
            },
            Event::TaskListMarker(value) => {
                if let Kind::List { checked, .. } = &mut kind { *checked = Some(value); }
                if let Some(Kind::List { checked, .. }) = items.last_mut() { *checked = Some(value); }
            }
            Event::SoftBreak | Event::HardBreak => text.push('\n'),
            Event::Rule => {
                if flush(&mut output, &mut text, &mut spans, &mut links, kind) || output.len() >= 512 { return (output, true); }
                output.push(Block { source_line: source_line(range.start), text: "────────".into(), kind: Kind::Paragraph, spans: Vec::new(), links: Vec::new() });
            },
            _ => {},
        }
    }
    let limited = flush(&mut output, &mut text, &mut spans, &mut links, kind);
    (output, limited)
}
/// Project original block styles through the same bounded/sanitized excerpt as
/// the file viewer, including Unicode byte lengths and a list-marker prefix.
pub fn excerpt_spans(text: &str, query: &str, spans: &[Span], prefix: usize) -> Vec<Span> {
    if spans.is_empty() { return Vec::new(); }
    let start = crate::file_browser::preview_excerpt_start(text, query);
    let mut display = prefix + if start > 0 { "…".len() } else { 0 };
    let mut current = 0;
    let mut output: Vec<Span> = Vec::new();
    for (source, ch) in text.char_indices().skip(start).take(1024) {
        let width = if ch.is_control() { 1 } else { ch.len_utf8() };
        while current < spans.len() && spans[current].range.end <= source { current += 1; }
        if let Some(span) = spans.get(current) && span.range.contains(&source) {
            if let Some(previous) = output.last_mut() && previous.range.end == display && previous.style == span.style { previous.range.end += width; }
            else { output.push(Span { range: display..display + width, style: span.style }); }
        }
        display += width;
    }
    output
}
/// At most eight visible cells per row; callers label omitted columns and retain
/// the complete tab-separated row for search/copy. No per-cell source copies.
pub fn table_cells<'a>(text: &'a str, spans: &[Span], query: &str) -> Vec<(String, Vec<Span>)> {
    let mut offset = 0;
    text.split('\t').take(TABLE_VISIBLE_COLUMNS).map(|cell| {
        let end = offset + cell.len();
        let styles: Vec<_> = spans.iter().filter_map(|span| {
            let start = span.range.start.max(offset);
            let stop = span.range.end.min(end);
            (start < stop).then(|| Span { range: start - offset..stop - offset, style: span.style })
        }).collect();
        offset = end + 1;
        (if cell.is_empty() { " ".into() } else { crate::file_browser::preview_line_title(cell, query) }, excerpt_spans(cell, query, &styles, 0))
    }).collect()
}

/// Sorted, disjoint runs combine emphasis with search colors without overlap.
pub fn merge_highlights(spans: &[Span], search: &[std::ops::Range<usize>]) -> Vec<(std::ops::Range<usize>, InlineStyle, bool)> {
    if spans.is_empty() { return search.iter().cloned().map(|range| (range, InlineStyle::default(), true)).collect(); }
    if search.is_empty() { return spans.iter().map(|span| (span.range.clone(), span.style, false)).collect(); }
    let mut edges: Vec<_> = spans.iter().flat_map(|span| [span.range.start, span.range.end]).chain(search.iter().flat_map(|range| [range.start, range.end])).collect();
    edges.sort_unstable(); edges.dedup();
    let (mut style_index, mut search_index) = (0, 0);
    edges.windows(2).filter_map(|pair| {
        while style_index < spans.len() && spans[style_index].range.end <= pair[0] { style_index += 1; }
        while search_index < search.len() && search[search_index].end <= pair[0] { search_index += 1; }
        let style = spans.get(style_index).filter(|span| span.range.contains(&pair[0])).map(|span| span.style).unwrap_or_default();
        let matched = search.get(search_index).is_some_and(|range| range.contains(&pair[0]));
        (style != InlineStyle::default() || matched).then_some((pair[0]..pair[1], style, matched))
    }).collect()
}

/// Display all captured code without executing controls or losing indentation.
/// The file loader bounds the source to 64 KiB / 512 source lines.
pub fn code_lines(source: &str) -> Vec<String> {
    let mut lines: Vec<_> = source.lines().map(|line| {
        let mut output = String::new();
        let mut columns = 0;
        for ch in line.chars() {
            let width = if ch == '\t' { 4 - columns % 4 } else if ch.is_control() { 1 }
                else { unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) };
            if ch == '\t' { output.extend(std::iter::repeat_n(' ', width)); }
            else { output.push(if ch.is_control() { ' ' } else { ch }); }
            columns += width;
        }
        output
    }).collect();
    if lines.is_empty() { lines.push("(empty code block)".into()); }
    lines
}

pub fn code_match_line(source: &str, query: &str) -> usize {
    let query = query.to_lowercase();
    let words: Vec<_> = query.split_whitespace().collect();
    source.lines().position(|line| {
        let line = line.to_lowercase();
        words.iter().any(|word| line.contains(word))
    }).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_links_resolve_against_the_document_and_reject_remote_or_executable_targets() {
        let source = std::env::temp_dir().join("rmux docs/sub/guide.md");
        let resolved = link_destination("../other%20guide.md", &source).unwrap();
        assert_eq!(super::super::terminal_link::local_file(&resolved, false).unwrap(),
            std::env::temp_dir().join("rmux docs/other guide.md"));
        assert!(link_destination("./", &source).unwrap().ends_with('/'));
        assert_eq!(link_destination("https://example.com/#part", &source).as_deref(), Some("https://example.com/#part"));
        for uri in ["", "#part", "other.md#part", "other.md?query", "//server/file", "file://server/a", "javascript:alert(1)", "data:text/plain,a", "mailto:a@b", "a%00.md", "a\\b"] {
            assert!(link_destination(uri, &source).is_none(), "{uri}");
        }
        let (rows, _) = blocks("[local](../guide.md) [script](javascript:alert) [anchor](#part)");
        assert_eq!(rows[0].links, ["../guide.md", "#part"]);
    }
    #[test]
    fn reading_blocks_retain_source_lines_across_blank_lines_and_markup() {
        let source = "# Title\r\n\r\nParagraph **bold**.\r\ncontinued\r\n\r\n- item\r\n\r\n```sh\r\necho 界\r\n```\r\n\r\n---\r\n\r\n| A | B |\r\n|---|---|\r\n| x | y |\r\n";
        let (rows, limited) = blocks(source);
        assert!(!limited);
        assert_eq!(rows.iter().map(|row| row.source_line).collect::<Vec<_>>(), [1, 3, 6, 9, 12, 14, 16]);
    }
    #[test]
    fn heading_fragments_decode_and_resolve_duplicate_unicode_titles() {
        let (rows, _) = blocks("# Hello, **World**!\n\n## Hello, World!\n\n## Hello World-1\n\n## Café 日本語\n");
        let targets = heading_targets(rows.iter().enumerate().filter_map(|(i, row)| matches!(row.kind, Kind::Heading(_)).then_some((i, row.text.as_str()))));
        assert_eq!(targets.get("hello-world"), Some(&0));
        assert_eq!(targets.get("hello-world-1"), Some(&1));
        assert_eq!(targets.get("hello-world-1-1"), Some(&2));
        assert_eq!(targets.get(&fragment("#caf%C3%A9-%E6%97%A5%E6%9C%AC%E8%AA%9E").unwrap()), Some(&3));
        assert_eq!(fragment("#"), Some(String::new()));
        for invalid in ["#%", "#%GG", "#%FF", "#%00", "#%0a", "file.md#part"] { assert!(fragment(invalid).is_none()); }
    }
    #[test]
    fn preview_link_navigation_tracks_identity_and_wraps() {
        assert_eq!(link_index(None, 7, 0, 1), None);
        assert_eq!(link_index(None, 7, 3, 0), Some(0));
        assert_eq!(link_index(Some((7, 2)), 7, 3, 1), Some(0));
        assert_eq!(link_index(Some((7, 0)), 7, 3, -1), Some(2));
        assert_eq!(link_index(Some((7, 2)), 8, 3, 0), Some(0));
        assert_eq!(link_index(Some((7, 9)), 7, 1, 0), Some(0));
    }
    #[test]
    fn links_preserve_unicode_styles_and_keep_images_inert() {
        let (rows, _) = blocks("[**界** code](https://example.com/a) [again](https://example.com/a) [file](../a.txt) ![image](https://example.com/image.png)\n\n[next](https://example.org)");
        assert_eq!(rows[0].links, ["https://example.com/a", "../a.txt"]);
        assert_eq!(rows[1].links, ["https://example.org"]);
        let first = rows[0].spans.first().unwrap();
        assert_eq!(&rows[0].text[first.range.clone()], "界");
        assert!(first.style.bold && first.style.link == Some(0));
        let projected = excerpt_spans(&rows[0].text, "code", &rows[0].spans, "• ".len());
        assert!(projected.iter().any(|span| span.style.link == Some(0)));
        let (table, _) = blocks("| Link | Other |\n|---|---|\n| [界](https://example.com) | plain |");
        let cells = table_cells(&table[1].text, &table[1].spans, "");
        assert_eq!(cells[0].1[0].range, 0.."界".len());
        assert_eq!(cells[0].1[0].style.link, Some(0));
        assert!(cells[1].1.is_empty());
        let source = (0..200).map(|index| format!("[label](https://example.com/{}?{index})\n\n", "x".repeat(1000))).collect::<String>();
        let (rows, _) = blocks(&source);
        assert!(rows.iter().map(|row| row.links.iter().map(String::len).sum::<usize>()).sum::<usize>() <= 65536);
        assert_eq!(rows.len(), 200);
        assert!(rows.last().unwrap().links.is_empty());
    }
    #[test]
    fn table_alignment_is_shared_by_rows_and_reset_between_tables() {
        let (rows, _) = blocks("| left | center | right | default |\n| :--- | :---: | ---: | --- |\n| a | b | 42 | d |\n\nafter\n\n| next |\n| --- |\n| plain |\n");
        let mut expected = [Alignment::None; TABLE_VISIBLE_COLUMNS];
        expected[..3].copy_from_slice(&[Alignment::Left, Alignment::Center, Alignment::Right]);
        for row in &rows[..2] {
            assert!(matches!(row.kind, Kind::Table { columns: 4, alignment, .. } if alignment == expected));
        }
        assert_eq!(rows[2].kind, Kind::Paragraph);
        for row in &rows[3..] {
            assert!(matches!(row.kind, Kind::Table { columns: 1, alignment, .. } if alignment == [Alignment::None; TABLE_VISIBLE_COLUMNS]));
        }
        let source = format!("|{}\n|{}\n|{}\n", " h |".repeat(100), " ---: |".repeat(100), " 1 |".repeat(100));
        let (rows, _) = blocks(&source);
        assert!(matches!(rows[0].kind, Kind::Table { columns: 100, alignment, .. } if alignment == [Alignment::Right; TABLE_VISIBLE_COLUMNS]));
    }
    #[test]
    fn tables_preserve_rows_empty_cells_and_inline_ranges() {
        let (rows, limited) = blocks("| Name | Value |\n| --- | ---: |\n| **界** | `a\tb` |\n| | |\n\nafter");
        assert!(!limited);
        assert!(matches!(rows[0].kind, Kind::Table { header: true, columns: 2, .. }));
        assert!(matches!(rows[1].kind, Kind::Table { header: false, columns: 2, .. }));
        assert_eq!(rows[1].text, "界\ta b");
        assert_eq!(rows[2].text, "\t");
        assert_eq!(rows[3].kind, Kind::Paragraph);
        let cells = table_cells(&rows[1].text, &rows[1].spans, "");
        assert_eq!(cells.len(), 2);
        assert!(cells[0].1[0].style.bold);
        assert!(cells[1].1[0].style.code);
        assert_eq!(&cells[0].0[cells[0].1[0].range.clone()], "界");
        assert_eq!(table_cells(&"x\t".repeat(100), &[], "").len(), 8);
        let (capped, limited) = blocks(&format!("| h |\n| - |\n{}", "| row |\n".repeat(512)));
        assert_eq!(capped.len(), 512);
        assert!(limited);
    }
    #[test]
    fn inline_code_and_strike_preserve_literals_and_search_styles() {
        let (result, limited) = blocks("~~old **bold** `*literal*`~~ and `~~code~~`\n\nplain");
        assert!(!limited);
        let block = &result[0];
        assert_eq!(block.text, "old bold *literal* and ~~code~~");
        let literal = block.spans.iter().find(|span| &block.text[span.range.clone()] == "*literal*").unwrap();
        assert!(literal.style.code && literal.style.strike && !literal.style.italic);
        let bold = block.spans.iter().find(|span| &block.text[span.range.clone()] == "bold").unwrap();
        assert!(bold.style.bold && bold.style.strike);
        let code = block.spans.last().unwrap();
        assert!(code.style.code && !code.style.strike);
        assert!(result[1].spans.is_empty());
        let search = crate::file_browser::preview_highlights(&block.text, "literal");
        let runs = merge_highlights(&block.spans, &search);
        assert!(runs.windows(2).all(|pair| pair[0].0.end <= pair[1].0.start));
        assert!(runs.iter().any(|(range, style, matched)| &block.text[range.clone()] == "literal" && style.code && style.strike && *matched));
    }
    #[test]
    fn inline_emphasis_nests_and_resets_across_blocks() {
        let (result, _) = blocks("***both*** normal **bold** *italic*\n\nplain\n");
        assert_eq!(result[0].text, "both normal bold italic");
        let spans: Vec<_> = result[0].spans.iter().map(|span| (&result[0].text[span.range.clone()], span.style)).collect();
        assert_eq!(spans, [("both", InlineStyle { bold: true, italic: true, ..Default::default() }), ("bold", InlineStyle { bold: true, italic: false, ..Default::default() }), ("italic", InlineStyle { bold: false, italic: true, ..Default::default() })]);
        assert!(result[1].spans.is_empty());
    }
    #[test]
    fn inline_excerpt_ranges_handle_unicode_controls_and_search_overlap() {
        let text = format!("{}A\u{85}界 needle", "İ".repeat(1100));
        let start = text.find('A').unwrap();
        let spans = [Span { range: start..start + "A\u{85}界".len(), style: InlineStyle { bold: true, italic: true, ..Default::default() } }];
        let prefix = "☑ ";
        let title = format!("{prefix}{}", crate::file_browser::preview_line_title(&text, "needle"));
        let projected = excerpt_spans(&text, "needle", &spans, prefix.len());
        assert_eq!(projected.len(), 1);
        assert_eq!(&title[projected[0].range.clone()], "A 界");
        let search = crate::file_browser::preview_highlights(&title, "界");
        let runs = merge_highlights(&projected, &search);
        assert!(runs.windows(2).all(|pair| pair[0].0.end <= pair[1].0.start));
        assert!(runs.iter().any(|(range, style, matched)| &title[range.clone()] == "界" && style.bold && style.italic && *matched));
        assert!(excerpt_spans(&text, "", &spans, 0).is_empty());
    }
    #[test]
    fn task_lists_keep_checked_state_without_modifying_body_text() {
        let source = "- [ ] pending\n- [x] done\n  - [X] nested\n\n7. [x] ordered\n\n   continuation\n\nordinary [ ] text\n";
        let (result, limited) = blocks(source);
        assert!(!limited);
        assert_eq!(result[0].text, "pending");
        assert!(matches!(result[0].kind, Kind::List { checked: Some(false), depth: 1, marker: true, .. }));
        assert!(matches!(result[1].kind, Kind::List { checked: Some(true), depth: 1, .. }));
        assert!(matches!(result[2].kind, Kind::List { checked: Some(true), depth: 2, .. }));
        assert!(matches!(result[3].kind, Kind::List { checked: Some(true), number: Some(7), marker: true, .. }));
        assert!(matches!(result[4].kind, Kind::List { checked: Some(true), marker: false, .. }));
        assert_eq!(result.last().unwrap().text, "ordinary [ ] text");
    }
    #[test]
    fn heading_levels_preserve_atx_and_setext_structure() {
        let (result, limited) = blocks("# First\n\n## Second\n\n### Third\n\n#### Fourth\n\n##### Fifth\n\n###### Sixth\n\nSetext one\n==========\n\nSetext two\n----------\n");
        assert!(!limited);
        assert_eq!(result.iter().map(|block| block.kind).collect::<Vec<_>>(), [Kind::Heading(1), Kind::Heading(2), Kind::Heading(3), Kind::Heading(4), Kind::Heading(5), Kind::Heading(6), Kind::Heading(1), Kind::Heading(2)]);
        assert_eq!(result[6].text, "Setext one");
    }
    #[test]
    fn code_view_keeps_all_lines_and_long_unicode_text() {
        let source = "zero\n\tif ready {\n\t\tprintln!(\"世界\");\n\t}\nlast\n";
        assert_eq!(code_lines(source), ["zero", "    if ready {", "        println!(\"世界\");", "    }", "last"]);
        assert_eq!(code_match_line(source, "世界"), 2);
        assert_eq!(code_match_line(source, "missing"), 0);
        assert_eq!(code_match_line(source, ""), 0);
        assert_eq!(code_lines(&"界".repeat(300)), ["界".repeat(300)]);
        assert_eq!(code_lines("a\x1bb\n界\tx"), ["a b", "界  x"]);
        assert_eq!(code_lines(""), ["(empty code block)"]);
        assert_eq!(code_lines(&"line\n".repeat(512)).len(), 512);
        assert!(source.contains('\t'));
    }
    #[test]
    fn nested_lists_preserve_numbers_depth_and_loose_paragraphs() {
        let (result, limited) = blocks("3. first\n\n   continuation\n\n   - nested\n   - second nested\n\n4. last\n\nafter\n");
        assert!(!limited);
        let items: Vec<_> = result.iter().filter_map(|block| match block.kind {
            Kind::List { depth, number, .. } => Some((block.text.as_str(), depth, number)), _ => None,
        }).collect();
        assert_eq!(items, [("first", 1, Some(3)), ("continuation", 1, Some(3)), ("nested", 2, None), ("second nested", 2, None), ("last", 1, Some(4))]);
        assert!(matches!(result[0].kind, Kind::List { marker: true, .. }));
        assert!(matches!(result[1].kind, Kind::List { marker: false, .. }));
        assert_eq!(result.last().unwrap().kind, Kind::Paragraph);
    }
    #[test]
    fn exact_block_cap_does_not_label_closing_events_as_omitted_content() {
        for source in ["# heading\n\n".repeat(512), "- item\n".repeat(512), "> paragraph\n>\n".repeat(512)] {
            let (result, limited) = blocks(&source);
            assert_eq!(result.len(), 512);
            assert!(!limited);
        }
        let source = "# heading\n\n".repeat(512);
        for tail in ["extra", "\n---\n", "\n```\ncode\n```\n"] {
            let (result, limited) = blocks(&(source.clone() + tail));
            assert_eq!(result.len(), 512);
            assert!(limited);
        }
        let (result, limited) = blocks(&("# heading\n\n".repeat(511) + "---\n"));
        assert_eq!(result.len(), 512);
        assert!(!limited);
    }
    #[test]
    fn markdown_blocks_are_capped() {
        let (result, limited) = blocks(&"# heading\n\n".repeat(600));
        assert_eq!(result.len(), 512);
        assert!(limited);
    }
    #[test]
    fn reading_blocks_preserve_text_without_fetching_or_executing_assets() {
        let (blocks, truncated) = blocks("# Title\n\nHello **world** [link](https://example.invalid).\n\n- item\n\n> quote\n\n```sh\necho hello\n```\n\n![alt](file:///missing.png)\n\n<script>alert(1)</script>\n");
        assert!(!truncated);
        assert_eq!(blocks[0].kind, Kind::Heading(1));
        assert_eq!(blocks[1].text, "Hello world link.");
        assert!(blocks.iter().any(|b| matches!(b.kind, Kind::List { .. }) && b.text == "item"));
        assert!(blocks.iter().any(|b| b.kind == Kind::Quote && b.text == "quote"));
        assert!(blocks.iter().any(|b| b.kind == Kind::Code && b.text == "echo hello\n"));
        assert!(blocks.iter().any(|b| b.text == "alt"));
        assert!(blocks.iter().any(|b| b.text.contains("<script>")));
    }
}

/// Link focus belongs to a result identity, never to its changing search-row index.
pub fn link_index(previous: Option<(usize, usize)>, command: usize, count: usize, delta: i32) -> Option<usize> {
    if count == 0 { return None; }
    let index = previous.filter(|(key, _)| *key == command).map_or(0, |(_, index)| index.min(count - 1));
    Some((index as i64 + i64::from(delta.signum())).rem_euclid(count as i64) as usize)
}
