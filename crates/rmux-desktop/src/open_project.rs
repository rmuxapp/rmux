//! Native folder-open events (including VS Code's external terminal command).
use super::*;

pub(super) fn matches_workspace(name: &str, stored_project: Option<&std::path::Path>, requested_name: &str, project: Option<&std::path::Path>) -> bool {
    match project { Some(path) => stored_project == Some(path), None => name == requested_name }
}

pub(super) fn matches_legacy_workspace(name: &str, launch_directory: Option<&std::path::Path>, requested_name: &str, project: &std::path::Path) -> bool {
    name == requested_name && launch_directory == Some(project)
}

fn project(uri: &str) -> anyhow::Result<automation::WorkspaceCreate> {
    let path = terminal_link::local_file(uri, false)
        .ok_or_else(|| anyhow::anyhow!("Open a local folder in rmux"))?;
    anyhow::ensure!(path.is_dir(), "The opened item is not a directory: {}", path.display());
    let path = path.canonicalize()?;
    let label = path.file_name().and_then(|name| name.to_str()).unwrap_or("Project");
    let name = window_view::normalized_workspace_name(label)
        .ok_or_else(|| anyhow::anyhow!("The folder needs a nonempty workspace name"))?;
    Ok(automation::WorkspaceCreate {
        window_id: String::new(), reuse: true, ssh_destination: None, project_directory: Some(path.clone()),
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
    fn project_identity_survives_rename_and_distinguishes_equal_folder_names() {
        use super::matches_workspace;
        let first = std::path::Path::new("/work/client-a/api");
        let second = std::path::Path::new("/work/client-b/api");
        assert!(matches_workspace("Renamed project", Some(first), "api", Some(first)));
        assert!(!matches_workspace("api", Some(first), "api", Some(second)));
        assert!(!matches_workspace("api", None, "api", Some(first)));
        assert!(matches_workspace("api", Some(first), "api", None));
        assert!(!matches_workspace("Renamed project", Some(first), "api", None));
    }
    #[test]
    fn legacy_adoption_requires_both_name_and_launch_directory() {
        use super::matches_legacy_workspace;
        let first = std::path::Path::new("/work/client-a/api");
        let second = std::path::Path::new("/work/client-b/api");
        assert!(matches_legacy_workspace("api", Some(first), "api", first));
        assert!(!matches_legacy_workspace("api", Some(second), "api", first));
        assert!(!matches_legacy_workspace("Other workspace", Some(first), "api", first));
        assert!(!matches_legacy_workspace("api", None, "api", first));
    }
    #[cfg(unix)]
    #[test]
    fn folder_aliases_share_a_project_identity() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let real = root.path().join("real"); std::fs::create_dir(&real)?;
        let alias = root.path().join("alias"); std::os::unix::fs::symlink(&real, &alias)?;
        let request = |path: &std::path::Path| project(url::Url::from_directory_path(path).unwrap().as_str());
        assert_eq!(request(&real)?.project_directory, request(&alias)?.project_directory);
        Ok(())
    }
    #[test]
    fn native_folder_requests_decode_paths_and_reuse_project_workspaces() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("Project 世界 spaces");
        std::fs::create_dir(&path)?;
        let request = project(url::Url::from_directory_path(&path).unwrap().as_str())?;
        assert_eq!(request.name.as_deref(), Some("Project 世界 spaces"));
        assert_eq!(request.cwd.as_deref(), Some(path.canonicalize()?.as_path()));
        assert!(request.reuse && request.focus && request.activate);
        assert_eq!(request.project_directory, request.cwd);
        let file = root.path().join("file.txt");
        std::fs::write(&file, "not a folder")?;
        assert!(project(url::Url::from_file_path(file).unwrap().as_str()).is_err());
        assert!(project("file://remote.example/tmp/project").is_err());
        assert!(project("https://example.com/project").is_err());
        Ok(())
    }
}
