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

#[cfg(test)]
mod tests {
    use super::local_file;
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
