//! File-link resolution is bounded and performs no filesystem access on hover.
use std::path::PathBuf;

pub fn local_file(uri: &str, remote: bool) -> Option<PathBuf> {
    if remote || uri.len() > 8192 || uri.chars().any(|c| c.is_control() || c.is_whitespace())
        || !uri.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://")) { return None; }
    if uri[7..].starts_with("//") || uri.contains('\\') { return None; }
    let url = url::Url::parse(uri).ok()?;
    if url.scheme() != "file" || url.query().is_some() || url.fragment().is_some()
        || url.host_str().is_some_and(|host| !host.is_empty() && !host.eq_ignore_ascii_case("localhost"))
        || url.path().starts_with("//") { return None; }
    let path = url.to_file_path().ok()?;
    (path.is_absolute() && path.as_os_str().len() <= 8192
        && !path.as_os_str().as_encoded_bytes().starts_with(b"//")
        && !path.as_os_str().as_encoded_bytes().starts_with(b"\\\\")
        && !path.as_os_str().as_encoded_bytes().iter().any(u8::is_ascii_control)).then_some(path)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Location {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl Location {
    pub fn label(&self) -> String {
        let mut label = self.path.display().to_string();
        if let Some(line) = self.line { label.push_str(&format!(":{line}")); }
        if let Some(column) = self.column { label.push_str(&format!(":{column}")); }
        label
    }
}

pub fn local_target(text: &str, cwd: Option<&std::path::Path>, remote: bool) -> Option<Location> {
    if remote || text.len() > 8192 || text.chars().any(char::is_control) { return None; }
    if text.get(..7).is_some_and(|s| s.eq_ignore_ascii_case("file://")) {
        return local_file(text, false).map(|path| Location { path, ..Default::default() });
    }
    if text.contains("://") || text.starts_with("//") || text.starts_with("\\\\") { return None; }
    let mut path = text;
    let mut line = None;
    let mut column = None;
    if let Some((prefix, suffix)) = path.rsplit_once(':')
        && !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit()) {
        line = Some(suffix.parse::<u32>().ok().filter(|n| *n > 0)?);
        path = prefix;
        if let Some((prefix, suffix)) = path.rsplit_once(':')
            && !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit()) {
            column = line;
            line = Some(suffix.parse::<u32>().ok().filter(|n| *n > 0)?);
            path = prefix;
        }
    }
    if path.is_empty() || path.starts_with('~') { return None; }
    #[cfg(not(windows))]
    if path.contains('\\') || path.contains(':') { return None; }
    let path = PathBuf::from(path);
    let path = if path.is_absolute() { path } else { cwd.filter(|p| p.is_absolute())?.join(path) };
    (path.as_os_str().len() <= 8192).then_some(Location { path, line, column })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compiler_locations_resolve_without_io_and_require_local_context() {
        let cwd = std::env::temp_dir();
        let location = local_target("src/世界/main.rs:42:7", Some(&cwd), false).unwrap();
        assert_eq!(location.path, cwd.join("src/世界/main.rs"));
        assert_eq!((location.line, location.column), (Some(42), Some(7)));
        assert_eq!(location.label(), format!("{}:42:7", cwd.join("src/世界/main.rs").display()));
        let location = local_target("main.rs:12", Some(&cwd), false).unwrap();
        assert_eq!((location.line, location.column), (Some(12), None));
        for text in ["src/main.rs:0", "src/main.rs:1:0", "src/main.rs:999999999999", "//server/a", "~/a", "https://example.com/a", "src/a\n"] {
            assert!(local_target(text, Some(&cwd), false).is_none(), "{text}");
        }
        assert!(local_target("src/main.rs:2", None, false).is_none());
        assert!(local_target("src/main.rs:2", Some(&cwd), true).is_none());
        let path = cwd.join("absolute.rs");
        assert_eq!(local_target(path.to_str().unwrap(), None, false).unwrap().path, path);
    }
    #[test]
    fn local_file_links_decode_literal_paths_without_shell_interpretation() {
        let path = std::env::temp_dir().join("café 世界 'quotes' $(echo).txt");
        let uri = url::Url::from_file_path(&path).unwrap().to_string();
        assert_eq!(local_file(&uri, false), Some(path));
        assert_eq!(local_file(&uri, true), None);
    }
    #[test]
    fn remote_ambiguous_and_non_file_targets_stay_blocked() {
        for uri in ["https://example.com/a", "file://remote.example/tmp/a", "file:////server/share/a",
            "file:///%2Fserver/share/a", "file:///tmp/a?download", "file:///tmp/a#L10", "file:///tmp/a%00b", "file:///tmp/a%0Ab",
            "file:///tmp/a b", "file:relative", "javascript:alert(1)"] {
            assert!(local_file(uri, false).is_none(), "{uri}");
        }
        assert!(local_file(&format!("file:///{}", "a".repeat(8192)), false).is_none());
    }
}
