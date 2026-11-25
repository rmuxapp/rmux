//! Opt-in Unix socket transport; shared RPC framing and dispatcher live in automation.
use std::os::unix::{
    fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
    net::{UnixListener, UnixStream},
};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

pub struct Endpoint {
    directory: PathBuf,
    identity: (u64, u64),
    socket_identity: (u64, u64),
}
impl Endpoint {
    pub fn bind(directory: &Path) -> io::Result<(Self, UnixListener)> {
        if !directory.is_absolute() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "socket directory must be absolute",
            ));
        }
        // Exclusive creation prevents replacing a live endpoint or following a
        // user-provided final symlink. No stale directory is removed implicitly.
        std::fs::DirBuilder::new().mode(0o700).create(directory).map_err(|error| {
            let detail = if error.kind() == io::ErrorKind::AlreadyExists {
                format!(
                    "Automation directory {} already exists. Another rmux instance may be using it. Quit that instance or choose a different, unused directory. Existing files have been preserved.",
                    directory.display()
                )
            } else {
                format!("Could not create automation directory {}: {error}", directory.display())
            };
            io::Error::new(error.kind(), detail)
        })?;
        let result = (|| {
            let metadata = std::fs::symlink_metadata(directory)?;
            let path = directory.join("rpc.sock");
            let listener = UnixListener::bind(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            let socket = std::fs::symlink_metadata(&path)?;
            Ok((
                Self {
                    directory: directory.into(),
                    identity: (metadata.dev(), metadata.ino()),
                    socket_identity: (socket.dev(), socket.ino()),
                },
                listener,
            ))
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(directory.join("rpc.sock"));
            let _ = std::fs::remove_dir(directory);
        }
        result
    }
    pub fn cleanup(&self) {
        let Ok(metadata) = std::fs::symlink_metadata(&self.directory) else {
            return;
        };
        if !metadata.is_dir() || (metadata.dev(), metadata.ino()) != self.identity {
            return;
        }
        let path = self.directory.join("rpc.sock");
        if let Ok(socket) = std::fs::symlink_metadata(&path) {
            if socket.file_type().is_socket()
                && (socket.dev(), socket.ino()) == self.socket_identity
            {
                let _ = std::fs::remove_file(path);
            }
        }
        let _ = std::fs::remove_dir(&self.directory);
    }
}
impl Drop for Endpoint {
    fn drop(&mut self) {
        self.cleanup();
    }
}

pub fn listen(
    listener: UnixListener,
    serve: impl Fn(UnixStream) + Send + Sync + 'static,
) -> io::Result<()> {
    let serve = Arc::new(serve);
    let clients = Arc::new(AtomicUsize::new(0));
    struct Permit(Arc<AtomicUsize>);
    impl Drop for Permit {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::AcqRel);
        }
    }
    std::thread::Builder::new()
        .name("rmux-socket".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    break;
                };
                if clients
                    .try_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                        (n < 8).then_some(n + 1)
                    })
                    .is_err()
                {
                    continue;
                }
                let permit = Permit(clients.clone());
                let serve = serve.clone();
                let _ = std::thread::Builder::new()
                    .name("rmux-rpc-client".into())
                    .spawn(move || {
                        let _permit = permit;
                        if stream
                            .set_read_timeout(Some(Duration::from_secs(10)))
                            .is_err()
                            || stream
                                .set_write_timeout(Some(Duration::from_secs(5)))
                                .is_err()
                        {
                            return;
                        }
                        serve(stream);
                    });
            }
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_endpoint_refuses_existing_paths_and_preserves_replacements() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("socket");
        let (endpoint, listener) = Endpoint::bind(&dir).unwrap();
        assert_eq!(std::fs::metadata(&dir).unwrap().mode() & 0o777, 0o700);
        assert_eq!(
            std::fs::metadata(dir.join("rpc.sock")).unwrap().mode() & 0o777,
            0o600
        );
        let error = Endpoint::bind(&dir).err().expect("existing endpoint must be refused");
        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert!(error.to_string().contains(&dir.display().to_string()));
        assert!(error.to_string().contains("Another rmux instance"));
        let connection = UnixStream::connect(dir.join("rpc.sock")).unwrap();
        let _ = listener.accept().unwrap();
        drop(connection);
        std::fs::remove_file(dir.join("rpc.sock")).unwrap();
        std::fs::write(dir.join("rpc.sock"), "replacement").unwrap();
        drop(endpoint);
        assert_eq!(
            std::fs::read_to_string(dir.join("rpc.sock")).unwrap(),
            "replacement"
        );
        let clean = root.path().join("clean");
        let (endpoint, _) = Endpoint::bind(&clean).unwrap();
        drop(endpoint);
        assert!(!clean.exists());
    }

    #[test]
    fn existing_files_and_symlinks_are_preserved_on_startup_failure() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("file");
        std::fs::write(&file, "keep").unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        for path in [&file, &link] {
            let error = Endpoint::bind(path).err().expect("existing paths must be refused");
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
            assert!(error.to_string().contains(&path.display().to_string()));
        }
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "keep");
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    }
}
