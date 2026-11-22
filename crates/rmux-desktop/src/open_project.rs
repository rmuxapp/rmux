//! Native folder-open events (including VS Code's external terminal command).
use super::*;

fn project(uri: &str) -> anyhow::Result<automation::WorkspaceCreate> {
    let path = terminal_link::local_file(uri, false)
        .ok_or_else(|| anyhow::anyhow!("Open a local folder in rmux"))?;
    anyhow::ensure!(path.is_dir(), "The opened item is not a directory: {}", path.display());
    let label = path.file_name().and_then(|name| name.to_str()).unwrap_or("Project");
    let name = window_view::normalized_workspace_name(label)
        .ok_or_else(|| anyhow::anyhow!("The folder needs a nonempty workspace name"))?;
    Ok(automation::WorkspaceCreate {
        window_id: String::new(), reuse: true, ssh_destination: None,
        name: Some(name), description: None, cwd: Some(path), focus: true, activate: true,
    })
}

pub(super) fn listen(receiver: async_channel::Receiver<String>, cx: &mut App) {
    cx.spawn(async move |cx| {
        while let Ok(uri) = receiver.recv().await {
            let project = cx.background_executor().spawn(async move { project(&uri) }).await;
            let result = match project {
                Ok(project) => automation::open_workspace(&project, cx).await
                    .map(|_| ()).map_err(|error| anyhow::anyhow!(error.message)),
                Err(error) => Err(error),
            };
            if let Err(error) = result {
                let message = format!("Could not open project: {error:#}");
                eprintln!("rmux: {message}");
                let _ = cx.update(|cx| {
                    for handle in cx.windows() {
                        if let Some(handle) = handle.downcast::<window_view::WorkspaceWindow>() {
                            let _ = handle.update(cx, |view, _, cx| view.show_startup_notice(message.clone(), cx));
                        }
                    }
                });
            }
        }
    }).detach();
}

#[cfg(test)]
mod tests {
    use super::project;
    #[test]
    fn native_folder_requests_decode_paths_and_reuse_named_workspaces() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("Project 世界 spaces");
        std::fs::create_dir(&path)?;
        let request = project(url::Url::from_directory_path(&path).unwrap().as_str())?;
        assert_eq!(request.name.as_deref(), Some("Project 世界 spaces"));
        assert_eq!(request.cwd.as_deref(), Some(path.as_path()));
        assert!(request.reuse && request.focus && request.activate);
        let file = root.path().join("file.txt");
        std::fs::write(&file, "not a folder")?;
        assert!(project(url::Url::from_file_path(file).unwrap().as_str()).is_err());
        assert!(project("file://remote.example/tmp/project").is_err());
        assert!(project("https://example.com/project").is_err());
        Ok(())
    }
}
