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

#[cfg(test)]
fn local_target(text: &str, cwd: Option<&std::path::Path>, remote: bool) -> Option<Location> {
    resolve_local_target(text, cwd, remote).ok()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinkError { Remote, MissingDirectory, MissingHome, Unsupported }
impl LinkError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Remote => "Remote files cannot be opened in a local editor",
            Self::MissingDirectory => "Working directory unknown. Enable shell integration or open a terminal with an explicit directory",
            Self::MissingHome => "Home directory unknown. Use an absolute file path instead",
            Self::Unsupported => "Unsupported file target. Use a local path or file URL",
        }
    }
}

pub fn resolve_local_target(text: &str, cwd: Option<&std::path::Path>, remote: bool) -> Result<Location, LinkError> {
    let home = if text.starts_with('~') {
        #[cfg(windows)]
        let root = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
        #[cfg(not(windows))]
        let root = std::env::var_os("HOME");
        root.map(PathBuf::from)
    } else { None };
    resolve_with_home(text, cwd, remote, home.as_deref())
}

fn resolve_with_home(text: &str, cwd: Option<&std::path::Path>, remote: bool,
    home: Option<&std::path::Path>) -> Result<Location, LinkError> {
    use LinkError::*;
    if remote { return Err(Remote); }
    if text.len() > 8192 || text.chars().any(char::is_control) { return Err(Unsupported); }
    if text.get(..7).is_some_and(|s| s.eq_ignore_ascii_case("file://")) {
        return local_file(text, false).map(|path| Location { path, ..Default::default() }).ok_or(Unsupported);
    }
    if text.contains("://") || text.starts_with("//") || text.starts_with("\\\\") { return Err(Unsupported); }
    let mut path = text;
    let mut line = None;
    let mut column = None;
    if let Some((prefix, suffix)) = path.rsplit_once(':')
        && !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit()) {
        line = Some(suffix.parse::<u32>().ok().filter(|n| *n > 0).ok_or(Unsupported)?);
        path = prefix;
        if let Some((prefix, suffix)) = path.rsplit_once(':')
            && !suffix.is_empty() && suffix.bytes().all(|c| c.is_ascii_digit()) {
            column = line;
            line = Some(suffix.parse::<u32>().ok().filter(|n| *n > 0).ok_or(Unsupported)?);
            path = prefix;
        }
    }
    if path.is_empty() { return Err(Unsupported); }
    #[cfg(not(windows))]
    if path.contains('\\') || path.contains(':') { return Err(Unsupported); }
    let path = if path == "~" || path.starts_with("~/") {
        let home = home.filter(|path| path.is_absolute()).ok_or(MissingHome)?;
        let suffix = path.strip_prefix("~/").unwrap_or("").trim_start_matches('/');
        let suffix = std::path::Path::new(suffix);
        // A second root/drive must never replace the chosen home directory.
        if suffix.components().any(|part| matches!(part, std::path::Component::Prefix(_) | std::path::Component::RootDir)) {
            return Err(Unsupported);
        }
        home.join(suffix)
    } else {
        if path.starts_with('~') { return Err(Unsupported); }
        PathBuf::from(path)
    };
    let path = if path.is_absolute() { path } else { cwd.filter(|p| p.is_absolute()).ok_or(MissingDirectory)?.join(path) };
    (path.as_os_str().len() <= 8192).then_some(Location { path, line, column }).ok_or(Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn home_paths_resolve_without_cwd_or_shell_expansion() {
        let home = std::env::temp_dir().join("local home 世界");
        let target = resolve_with_home("~/my project/世界.rs:12:3", None, false, Some(&home)).unwrap();
        assert_eq!(target.path, home.join("my project/世界.rs"));
        assert_eq!((target.line, target.column), (Some(12), Some(3)));
        assert_eq!(resolve_with_home("~//notes.txt", None, false, Some(&home)).unwrap().path, home.join("notes.txt"));
        assert_eq!(resolve_with_home("~/$(touch nope)/$NAME.rs", None, false, Some(&home)).unwrap().path,
            home.join("$(touch nope)/$NAME.rs"));
        assert_eq!(resolve_with_home("~/a", None, true, Some(&home)), Err(LinkError::Remote));
        assert_eq!(resolve_with_home("~/a", None, false, None), Err(LinkError::MissingHome));
        assert_eq!(resolve_with_home("~/a", None, false, Some(std::path::Path::new("relative"))), Err(LinkError::MissingHome));
        assert_eq!(resolve_with_home("~someone/a", None, false, Some(&home)), Err(LinkError::Unsupported));
    }
    #[test]
    fn unresolved_links_explain_the_missing_requirement() {
        assert_eq!(resolve_local_target("src/main.rs:12", None, false), Err(LinkError::MissingDirectory));
        assert_eq!(resolve_local_target("src/main.rs:12", Some(std::path::Path::new("relative")), false), Err(LinkError::MissingDirectory));
        assert_eq!(resolve_local_target("src/main.rs:12", None, true), Err(LinkError::Remote));
        assert_eq!(resolve_local_target("src/main.rs:0", None, false), Err(LinkError::Unsupported));
        assert_eq!(resolve_local_target("file://remote.example/tmp/file", None, false), Err(LinkError::Unsupported));
    }
    #[test]
    fn compiler_locations_resolve_without_io_and_require_local_context() {
        let cwd = std::env::temp_dir();
        let location = local_target("src/世界/main.rs:42:7", Some(&cwd), false).unwrap();
        assert_eq!(location.path, cwd.join("src/世界/main.rs"));
        assert_eq!((location.line, location.column), (Some(42), Some(7)));
        assert_eq!(location.label(), format!("{}:42:7", cwd.join("src/世界/main.rs").display()));
        let location = local_target("main.rs:12", Some(&cwd), false).unwrap();
        assert_eq!((location.line, location.column), (Some(12), None));
        for text in ["src/main.rs:0", "src/main.rs:1:0", "src/main.rs:999999999999", "//server/a", "~other/a", "https://example.com/a", "src/a\n"] {
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
