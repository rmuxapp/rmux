//! Launch OpenSSH directly; authentication/configuration remain client-owned.
pub const INPUT_HINT: &str = "SSH alias, user@host or ssh://user@host:port · Authenticate in the terminal";
pub struct Aliases { pub names: Vec<String>, pub limited: bool }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetSource { Session, Saved, Config }
pub struct HostTarget { pub destination: String, pub source: TargetSource }

/// Exact deduplication preserves case-sensitive usernames in explicit destinations.
pub fn host_targets(session_targets: Vec<HostTarget>, aliases: &[String]) -> Vec<HostTarget> {
    let mut targets = std::collections::BTreeMap::<String, TargetSource>::new();
    for target in session_targets.into_iter().take(256).chain(aliases.iter().take(256).map(|destination|
        HostTarget { destination: destination.clone(), source: TargetSource::Config })) {
        if command(&target.destination).is_err() { continue; }
        targets.entry(target.destination).and_modify(|source| *source = (*source).min(target.source)).or_insert(target.source);
    }
    let mut targets: Vec<_> = targets.into_iter().map(|(destination, source)| HostTarget { destination, source }).collect();
    targets.sort_by(|a, b| a.source.cmp(&b.source).then_with(|| a.destination.cmp(&b.destination)));
    targets
}

pub fn read_aliases(path: &std::path::Path, canceled: &std::sync::atomic::AtomicBool) -> anyhow::Result<Aliases> {
    let base = path.parent().unwrap_or(std::path::Path::new("."));
    let mut discovery = Discovery {
        canceled, base, remaining: 256 * 1024, entries: 2048, attempts: 64,
        visited: Default::default(), names: Default::default(), limited: false,
    };
    discovery.visit(path, 0)?;
    Ok(Aliases { names: discovery.names.into_values().collect(), limited: discovery.limited })
}

struct Discovery<'a> {
    canceled: &'a std::sync::atomic::AtomicBool,
    base: &'a std::path::Path,
    remaining: usize,
    entries: usize,
    attempts: usize,
    visited: std::collections::HashSet<std::path::PathBuf>,
    names: std::collections::BTreeMap<String, String>,
    limited: bool,
}

impl Discovery<'_> {
    fn check(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.canceled.load(std::sync::atomic::Ordering::Acquire), "SSH host loading canceled");
        Ok(())
    }

    fn visit(&mut self, path: &std::path::Path, depth: usize) -> anyhow::Result<()> {
        self.check()?;
        if depth > 8 || self.attempts == 0 || self.remaining == 0 {
            self.limited = true;
            return Ok(());
        }
        self.attempts -= 1;
        // Canonical identities stop symlink cycles; missing literal includes are harmless.
        let identity = match path.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if !self.visited.insert(identity) { return Ok(()); }
        let (text, limited) = read_config(path, self.canceled, &mut self.remaining)?;
        self.limited |= limited;
        let aliases = parse_aliases(&text);
        self.limited |= aliases.limited;
        for name in aliases.names {
            let key = name.to_ascii_lowercase();
            if self.names.contains_key(&key) { continue; }
            if self.names.len() == 256 { self.limited = true; break; }
            self.names.insert(key, name);
        }
        for line in text.lines() {
            self.check()?;
            let Some(words) = config_words(line) else { continue; };
            if !words.first().is_some_and(|word| word.eq_ignore_ascii_case("Include")) { continue; }
            for pattern in words.iter().skip(1) {
                for include in self.expand(pattern)? {
                    if self.visit(&include, depth + 1).is_err() {
                        self.check()?;
                        // Keep readable aliases when a single included file is unavailable.
                        self.limited = true;
                    }
                }
            }
        }
        Ok(())
    }

    fn expand(&mut self, pattern: &str) -> anyhow::Result<Vec<std::path::PathBuf>> {
        use std::path::{Component, Path, PathBuf};
        self.check()?;
        if self.attempts == 0 || self.remaining == 0 { self.limited = true; return Ok(Vec::new()); }
        // Context-dependent tokens/environment/named-user homes are not guessed.
        if pattern.len() > 4096 || pattern.contains(['%', '$']) || (pattern.starts_with('~') && !pattern.starts_with("~/")) {
            self.limited = true;
            return Ok(Vec::new());
        }
        let path = if let Some(tail) = pattern.strip_prefix("~/") {
            self.base.parent().unwrap_or(self.base).join(tail)
        } else if Path::new(pattern).is_absolute() { PathBuf::from(pattern) }
        else { self.base.join(pattern) };
        let mut paths = vec![PathBuf::new()];
        for component in path.components() {
            self.check()?;
            let Component::Normal(name) = component else {
                for path in &mut paths { path.push(component.as_os_str()); }
                continue;
            };
            let name = name.to_string_lossy();
            if !name.contains(['*', '?', '[']) {
                for path in &mut paths { path.push(component.as_os_str()); }
                continue;
            }
            if self.entries == 0 { self.limited = true; return Ok(Vec::new()); }
            let matcher = match glob::Pattern::new(&name) {
                Ok(matcher) => matcher,
                Err(_) => { self.limited = true; return Ok(Vec::new()); }
            };
            let mut matches = Vec::new();
            for parent in paths {
                let entries = match std::fs::read_dir(&parent) {
                    Ok(entries) => entries,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(_) => { self.limited = true; continue; }
                };
                for entry in entries {
                    self.check()?;
                    if self.entries == 0 { self.limited = true; break; }
                    self.entries -= 1;
                    match entry {
                        Ok(entry) if matcher.matches_path_with(&PathBuf::from(entry.file_name()), glob::MatchOptions {
                            case_sensitive: true, require_literal_separator: true, require_literal_leading_dot: true,
                        }) => matches.push(entry.path()),
                        Err(_) => self.limited = true,
                        _ => {},
                    }
                }
                if self.entries == 0 { break; }
            }
            matches.sort();
            paths = matches;
        }
        Ok(paths)
    }
}

fn read_config(path: &std::path::Path, canceled: &std::sync::atomic::AtomicBool, remaining: &mut usize) -> anyhow::Result<(String, bool)> {
    use std::io::Read;
    let check = || -> anyhow::Result<()> {
        anyhow::ensure!(!canceled.load(std::sync::atomic::Ordering::Acquire), "SSH host loading canceled");
        Ok(())
    };
    check()?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)] {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok((String::new(), false)),
        Err(error) => return Err(error.into()),
    };
    anyhow::ensure!(file.metadata()?.is_file(), "SSH configuration is not a regular file");
    let cap = (*remaining).min(65536);
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    while bytes.len() < (cap + 1) {
        check()?;
        let limit = chunk.len().min((cap + 1) - bytes.len());
        let count = match file.read(&mut chunk[..limit]) {
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if count == 0 { break; }
        bytes.extend_from_slice(&chunk[..count]);
    }
    check()?;
    let limited = bytes.len() > cap;
    *remaining = remaining.saturating_sub(bytes.len().min(cap));
    if limited {
        bytes.truncate(cap);
        // Never offer a host name cut in half at the read limit.
        bytes.truncate(bytes.iter().rposition(|byte| *byte == b'\n').map_or(0, |index| index + 1));
    }
    let text = std::str::from_utf8(&bytes)?;
    Ok((text.to_owned(), limited))
}

fn config_words(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped { word.push(ch); escaped = false; continue; }
        match ch {
            '\\' => escaped = true,
            '"' => quoted = !quoted,
            '#' if !quoted => break,
            ch if !quoted && (ch.is_whitespace() || ch == '=') => {
                if !word.is_empty() { words.push(std::mem::take(&mut word)); }
            }
            ch => word.push(ch),
        }
    }
    if quoted || escaped { return None; }
    if !word.is_empty() { words.push(word); }
    Some(words)
}

fn parse_aliases(text: &str) -> Aliases {
    let mut names = std::collections::BTreeMap::new();
    let mut limited = false;
    for line in text.lines() {
        let Some(words) = config_words(line) else { continue; };
        if !words.first().is_some_and(|word| word.eq_ignore_ascii_case("Host")) { continue; }
        for name in words.into_iter().skip(1) {
            if name.is_empty() || name.len() > 255 || name.starts_with('-')
                || !name.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"._-:%".contains(&ch)) { continue; }
            let key = name.to_ascii_lowercase();
            if names.contains_key(&key) { continue; }
            if names.len() == 256 { limited = true; break; }
            names.insert(key, name);
        }
        if limited { break; }
    }
    Aliases { names: names.into_values().collect(), limited }
}

/// Start a sibling terminal without treating a remote OSC directory as a local cwd.
/// Callers supply only a session's explicit launch identity, never its saved target.
pub fn launch_sibling(destination: Option<&str>, directory: Option<&std::path::Path>, scrollback: usize) -> anyhow::Result<crate::Session> {
    use crate::Session;
    let Some(destination) = destination else {
        return Session::shell_in_with_scrollback(80, 24, directory, scrollback);
    };
    let command = command(destination).map_err(anyhow::Error::msg)?;
    let argv = command.argv.iter().map(std::ffi::OsString::from).collect::<Vec<_>>();
    let mut session = Session::command_in_with_scrollback(&argv, 80, 24, directory, scrollback)?;
    session.set_remote_destination(destination)?;
    Ok(session)
}

pub fn command(destination: &str) -> Result<crate::preferences::CustomCommand, &'static str> {
    let authority = destination.strip_prefix("ssh://").unwrap_or(destination);
    if destination.is_empty() || destination.len() > 255 || destination.starts_with('-')
        || !authority.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"._-@:%[]".contains(&ch)) {
        return Err("Enter an SSH destination (up to 255 bytes), without spaces, paths or options");
    }
    let mut parts = authority.split('@');
    let first = parts.next().unwrap_or_default();
    let host = parts.next().unwrap_or(first);
    if first.is_empty() || host.is_empty() || host.starts_with('-') || parts.next().is_some() {
        return Err("Enter one host, optionally prefixed with user@");
    }
    let argv = if destination.starts_with("ssh://") {
        if authority.contains('@') && !first.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"._-".contains(&ch)) {
            return Err("SSH URLs accept a username, not passwords or encoded user information");
        }
        let (host, port) = validate_url_host(host)?;
        let mut argv = vec!["ssh".into()];
        if authority.contains('@') { argv.extend(["-l".into(), first.into()]); }
        if let Some(port) = port { argv.extend(["-p".into(), port.into()]); }
        argv.extend(["--".into(), host.into()]);
        argv
    } else { vec!["ssh".into(), "--".into(), destination.into()] };
    Ok(crate::preferences::CustomCommand {
        mode: crate::preferences::CommandMode::Terminal,
        name: format!("SSH · {destination}"),
        argv,
        env: Default::default(), cwd: None,
    })
}

fn validate_url_host(host: &str) -> Result<(&str, Option<&str>), &'static str> {
    let (hostname, port) = if let Some(bracketed) = host.strip_prefix('[') {
        let (address, suffix) = bracketed.split_once(']').ok_or("Close the IPv6 address with ]")?;
        let hostname = address;
        let (address, zone) = address.split_once('%').map_or((address, None), |(address, zone)| (address, Some(zone)));
        if address.parse::<std::net::Ipv6Addr>().is_err() || zone.is_some_and(|zone| zone.is_empty() || !zone.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"._-".contains(&ch))) {
            return Err("Enter a valid bracketed IPv6 address");
        }
        (hostname, if suffix.is_empty() { None } else { Some(suffix.strip_prefix(':').ok_or("Use :port after the IPv6 address")?) })
    } else {
        let (name, port) = host.split_once(':').map_or((host, None), |(name, port)| (name, Some(port)));
        if name.is_empty() || !name.bytes().all(|ch| ch.is_ascii_alphanumeric() || b"._-".contains(&ch)) {
            return Err("Enter a host name, or put an IPv6 address in brackets");
        }
        (name, port)
    };
    if port.is_some_and(|port| port.is_empty() || !port.bytes().all(|ch| ch.is_ascii_digit()) || port.parse::<u16>().map_or(true, |port| port == 0)) {
        return Err("SSH port must be a number from 1 to 65535; bracket IPv6 addresses");
    }
    Ok((hostname, port))
}

#[cfg(test)]
mod tests {
    #[test]
    fn host_picker_merges_sources_without_folding_usernames() {
        use super::{HostTarget, TargetSource, host_targets};
        let targets = host_targets(vec![
            HostTarget { destination: "prod".into(), source: TargetSource::Saved },
            HostTarget { destination: "prod".into(), source: TargetSource::Session },
            HostTarget { destination: "Alice@host".into(), source: TargetSource::Saved },
            HostTarget { destination: "alice@host".into(), source: TargetSource::Saved },
            HostTarget { destination: "-option".into(), source: TargetSource::Saved },
        ], &["prod".into(), "dev".into()]);
        assert_eq!(targets.iter().map(|target| (target.destination.as_str(), target.source)).collect::<Vec<_>>(), [
            ("prod", TargetSource::Session), ("Alice@host", TargetSource::Saved),
            ("alice@host", TargetSource::Saved), ("dev", TargetSource::Config),
        ]);
        let targets = host_targets((0..300).map(|i| HostTarget { destination: format!("session{i}"), source: TargetSource::Session }).collect(),
            &(0..300).map(|i| format!("config{i}")).collect::<Vec<_>>());
        assert_eq!(targets.len(), 512);
    }

    #[test]
    fn host_aliases_are_literal_sorted_deduplicated_and_bounded() {
        let aliases = super::parse_aliases("# Hosts\nHost \"Production\" dev *.example !blocked\n HostName internal\nhOsT=DEV staging # comment\nInclude elsewhere\nMatch exec something\nHost \"unterminated\nHost user@host -option\n");
        assert_eq!(aliases.names, ["dev", "Production", "staging"]);
        assert!(!aliases.limited);
        let many: String = (0..300).map(|index| format!("Host alias-{index}\n")).collect();
        let aliases = super::parse_aliases(&many);
        assert_eq!(aliases.names.len(), 256);
        assert!(aliases.limited);
    }
    #[test]
    fn host_config_reads_drop_partial_lines_and_honor_cancellation() -> anyhow::Result<()> {
        use std::sync::atomic::AtomicBool;
        let root = tempfile::tempdir()?;
        let path = root.path().join("config");
        assert!(super::read_aliases(&path, &AtomicBool::new(false))?.names.is_empty());
        assert!(super::read_aliases(&path, &AtomicBool::new(true)).is_err());
        let mut text = "Host complete\n".to_owned();
        text.push_str(&format!("#{}\n", "x".repeat(65500)));
        text.push_str(&format!("Host {}", "partial".repeat(20)));
        std::fs::write(&path, text)?;
        let aliases = super::read_aliases(&path, &AtomicBool::new(false))?;
        assert_eq!(aliases.names, ["complete"]);
        assert!(aliases.limited);
        Ok(())
    }
    #[test]
    fn includes_expand_from_user_config_root_and_stop_cycles() -> anyhow::Result<()> {
        use std::sync::atomic::AtomicBool;
        let root = tempfile::tempdir()?;
        let base = root.path().join(".ssh");
        std::fs::create_dir_all(base.join("config.d/nested"))?;
        std::fs::write(base.join("config"), "Host root\nInclude config.d/*.conf\nInclude ~/.ssh/extra\n")?;
        std::fs::write(base.join("config.d/a.conf"), "Host alpha duplicate\nInclude config.d/nested/leaf\n")?;
        std::fs::write(base.join("config.d/b.conf"), "Host beta DUPLICATE\nInclude config\n")?;
        std::fs::write(base.join("config.d/.hidden.conf"), "Host hidden\n")?;
        std::fs::write(base.join("config.d/nested/leaf"), "Host leaf\n")?;
        std::fs::write(base.join("extra"), "Host extra\n")?;
        let aliases = super::read_aliases(&base.join("config"), &AtomicBool::new(false))?;
        assert_eq!(aliases.names, ["alpha", "beta", "duplicate", "extra", "leaf", "root"]);
        assert!(!aliases.limited);
        Ok(())
    }

    #[test]
    fn include_limits_and_unreadable_files_keep_available_aliases() -> anyhow::Result<()> {
        use std::sync::atomic::AtomicBool;
        let root = tempfile::tempdir()?;
        let path = root.path().join("config");
        std::fs::write(&path, "Host available\nInclude invalid missing %h/config\nInclude chain0\n")?;
        std::fs::write(root.path().join("invalid"), [0xff])?;
        for index in 0..12 {
            std::fs::write(root.path().join(format!("chain{index}")), format!("Host host{index}\nInclude chain{}\n", index + 1))?;
        }
        let aliases = super::read_aliases(&path, &AtomicBool::new(false))?;
        assert!(aliases.limited);
        assert!(aliases.names.contains(&"available".to_owned()));
        assert!(aliases.names.contains(&"host7".to_owned()));
        assert!(!aliases.names.contains(&"host8".to_owned()));
        // The file-attempt budget also bounds a wide fanout.
        let includes: String = (0..100).map(|index| format!("Include file{index}\n")).collect();
        std::fs::write(&path, includes)?;
        for index in 0..100 { std::fs::write(root.path().join(format!("file{index}")), format!("Host wide{index}\n"))?; }
        let aliases = super::read_aliases(&path, &AtomicBool::new(false))?;
        assert!(aliases.limited);
        assert_eq!(aliases.names.len(), 63);
        Ok(())
    }

    #[test]
    fn include_directory_and_total_byte_budgets_are_shared() -> anyhow::Result<()> {
        use std::sync::atomic::AtomicBool;
        let root = tempfile::tempdir()?;
        for index in 0..6 { std::fs::write(root.path().join(format!("file{index}")), "Host retained\n")?; }
        let canceled = AtomicBool::new(false);
        let mut discovery = super::Discovery {
            canceled: &canceled, base: root.path(), remaining: 20, entries: 3, attempts: 64,
            visited: Default::default(), names: Default::default(), limited: false,
        };
        assert_eq!(discovery.expand("file*")?.len(), 3);
        assert_eq!(discovery.entries, 0);
        assert!(discovery.limited);
        discovery.limited = false;
        discovery.visit(&root.path().join("file0"), 0)?;
        discovery.visit(&root.path().join("file1"), 0)?;
        assert_eq!(discovery.remaining, 0);
        assert!(discovery.limited);
        assert_eq!(discovery.names.into_values().collect::<Vec<_>>(), ["retained"]);
        Ok(())
    }

    #[test]
    fn ssh_urls_validate_ports_ipv6_and_exclude_passwords_or_paths() {
        for (destination, expected) in [
            ("ssh://user@example.org:2222", vec!["ssh", "-l", "user", "-p", "2222", "--", "example.org"]),
            ("ssh://example.org", vec!["ssh", "--", "example.org"]),
            ("ssh://user@[2001:db8::1]:65535", vec!["ssh", "-l", "user", "-p", "65535", "--", "2001:db8::1"]),
            ("ssh://[::1]", vec!["ssh", "--", "::1"]),
            ("ssh://[fe80::1%en0]:22", vec!["ssh", "-p", "22", "--", "fe80::1%en0"]),
        ] {
            assert_eq!(super::command(destination).unwrap().argv, expected);
        }
        for destination in ["ssh://", "ssh://host:", "ssh://host:0", "ssh://host:65536", "ssh://host:+22", "ssh://host/path", "ssh://user:password@host", "ssh://[bad]:22", "ssh://[::1]extra", "ssh://::1", "ssh://[::1", "ssh://[::1%]:22", "ssh://-host"] {
            assert!(super::command(destination).is_err(), "{destination:?}");
        }
    }
    #[test]
    fn destinations_are_one_literal_argument_without_shell_options() {
        for destination in ["production", "user@example.org", "user@2001:db8::1", "host-name", "user@fe80::1%en0"] {
            let command = super::command(destination).unwrap();
            assert_eq!(command.argv, ["ssh", "--", destination]);
            assert!(command.env.is_empty());
        }
        for destination in ["", "-oProxyCommand=bad", "host command", "$(command)", "a;b", "a\nb", "@host", "user@", "a@b@c", "user@-option"] {
            assert!(super::command(destination).is_err(), "{destination:?}");
        }
        assert!(super::command(&"x".repeat(256)).is_err());
    }
}
