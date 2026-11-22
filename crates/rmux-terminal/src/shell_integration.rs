//! Private startup files, never modifications to the user's shell configuration.
use anyhow::Result;
use portable_pty::CommandBuilder;
use std::ffi::OsStr;

/// Encode history data in a private startup directory. Never executable.
pub(crate) fn write_history_import(directory: &std::path::Path, shell: &str, history: &rmux_core::shell_history::History) -> Result<std::path::PathBuf> {
    use std::io::Write;
    anyhow::ensure!(matches!(shell, "bash" | "zsh"), "history import requires Bash or zsh");
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    for command in history.entries() {
        if shell == "bash" {
            // Bash 3.2 splits multiline native files. NUL records are read
            // into history -s as a single quoted argument, never executed.
            file.write_all(command.as_bytes())?;
            file.write_all(&[0])?;
        } else {
            file.write_all(b": 1:0;")?;
            // zsh's history reader unescapes its internal Meta byte (0x83).
            // Escape zsh internal token bytes so UTF-8 survives unchanged.
            for byte in command.bytes() {
                if (0x83..=0xa2).contains(&byte) { file.write_all(&[0x83, byte ^ 32])?; }
                else if byte == b'\n' { file.write_all(b"\\\n")?; }
                else { file.write_all(&[byte])?; }
            }
            file.write_all(b"\n")?;
        }
    }
    let path = directory.join("session-history");
    file.persist_noclobber(&path)?;
    Ok(path)
}

const RESTORE: &str = r#"
if [[ $RMUX_ZSH_ORIGINAL_SET == 1 ]]; then
    export ZDOTDIR=$RMUX_ZSH_ORIGINAL
else
    unset ZDOTDIR
fi
"#;

const REDIRECT: &str = r#"
if [[ -o rcs ]]; then
    export RMUX_ZSH_ORIGINAL_SET=${+ZDOTDIR}
    export RMUX_ZSH_ORIGINAL=${ZDOTDIR-}
    export ZDOTDIR=$RMUX_ZSH_WRAPPER
else
    unset RMUX_ZSH_ORIGINAL_SET RMUX_ZSH_ORIGINAL RMUX_ZSH_WRAPPER
fi
"#;

// PowerShell loads its ordinary profiles before -Command. The private path is
// data in the environment; never interpolate filesystem paths as command code.
pub(crate) fn prepare_powershell(
    command: &mut CommandBuilder,
    login: bool,
) -> Result<tempfile::TempDir> {
    let directory = tempfile::Builder::new()
        .prefix("rmux-powershell-")
        .tempdir()?;
    std::fs::write(
        directory.path().join("rmux.ps1"),
        include_str!("../../../shell-integration/powershell/rmux.ps1"),
    )?;
    if cfg!(unix) && login {
        command.arg("-Login");
    }
    command.args(["-NoExit", "-Command", r#"try { . (Join-Path $env:RMUX_POWERSHELL_WRAPPER 'rmux.ps1') } finally { Remove-Item Env:\RMUX_POWERSHELL_WRAPPER -ErrorAction SilentlyContinue }"#]);
    command.env("RMUX_POWERSHELL_WRAPPER", directory.path());
    Ok(directory)
}

// Fish evaluates init commands after config.fish for both login modes. Pass the
// private path through the environment so spaces or shell syntax are never code.
pub(crate) fn prepare_fish(command: &mut CommandBuilder) -> Result<tempfile::TempDir> {
    let directory = tempfile::Builder::new().prefix("rmux-fish-").tempdir()?;
    std::fs::write(
        directory.path().join("rmux.fish"),
        include_str!("../../../shell-integration/fish/rmux.fish"),
    )?;
    command.env("RMUX_FISH_WRAPPER", directory.path());
    command.arg("--init-command");
    command.arg(r#"source "$RMUX_FISH_WRAPPER/rmux.fish"; set -e RMUX_FISH_WRAPPER"#);
    Ok(directory)
}

pub(crate) fn prepare_bash(command: &mut CommandBuilder) -> Result<tempfile::TempDir> {
    let directory = tempfile::Builder::new().prefix("rmux-bash-").tempdir()?;
    std::fs::write(
        directory.path().join("bashrc"),
        r#"
[[ ! -r $HOME/.bashrc ]] || source "$HOME/.bashrc"
source "$RMUX_BASH_WRAPPER/rmux.bash"
unset RMUX_BASH_WRAPPER
"#,
    )?;
    std::fs::write(
        directory.path().join("rmux.bash"),
        include_str!("../../../shell-integration/bash/rmux.bash"),
    )?;
    command.arg("--rcfile");
    command.arg(directory.path().join("bashrc"));
    command.env("RMUX_BASH_WRAPPER", directory.path());
    Ok(directory)
}

pub(crate) fn prepare_zsh(
    command: &mut CommandBuilder,
    original: Option<&OsStr>,
) -> Result<tempfile::TempDir> {
    let directory = tempfile::Builder::new().prefix("rmux-zsh-").tempdir()?;
    for file in [".zshenv", ".zprofile"] {
        let script = format!(
            "{RESTORE}\n[[ ! -r ${{ZDOTDIR-$HOME}}/{file} ]] || source \"${{ZDOTDIR-$HOME}}/{file}\"\n{REDIRECT}"
        );
        std::fs::write(directory.path().join(file), script)?;
    }
    std::fs::write(
        directory.path().join(".zshrc"),
        format!(
            r#"{RESTORE}
[[ ! -r ${{ZDOTDIR-$HOME}}/.zshrc ]] || source "${{ZDOTDIR-$HOME}}/.zshrc"
source "$RMUX_ZSH_WRAPPER/rmux.zsh"
unset RMUX_ZSH_ORIGINAL_SET RMUX_ZSH_ORIGINAL RMUX_ZSH_WRAPPER
"#
        ),
    )?;
    std::fs::write(
        directory.path().join("rmux.zsh"),
        include_str!("../../../shell-integration/zsh/rmux.zsh"),
    )?;
    command.env(
        "RMUX_ZSH_ORIGINAL_SET",
        if original.is_some() { "1" } else { "0" },
    );
    command.env(
        "RMUX_ZSH_ORIGINAL",
        original.unwrap_or_else(|| OsStr::new("")),
    );
    command.env("RMUX_ZSH_WRAPPER", directory.path());
    command.env("ZDOTDIR", directory.path());
    Ok(directory)
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    #[test]
    fn restored_history_is_data_and_does_not_duplicate_global_history() -> Result<()> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        for shell in ["bash", "zsh"] {
            let temporary = tempfile::tempdir()?;
            let root = temporary.path();
            let mut history = rmux_core::shell_history::History::default();
            for entry in ["touch \"$RMUX_TEST_EXECUTED\"", "printf '%s\\n' '世界\nsecond line'", "echo BACK\\SLASH", "echo Ã 🎃", "echo \\\ncontinued"] {
                history.record(entry);
            }
            let import = write_history_import(root, shell, &history)?;
            let global = root.join("global");
            std::fs::write(&global, "echo GLOBAL_SENTINEL\n")?;
            let mut command = Command::new(format!("/bin/{shell}"));
            if shell == "bash" { command.args(["--noprofile", "--norc", "--noediting", "-i"]); }
            else { command.args(["-d", "-f", "-i"]); }
            let integration = if shell == "bash" { include_str!("../../../shell-integration/bash/rmux.bash") }
                else { include_str!("../../../shell-integration/zsh/rmux.zsh") };
            std::fs::write(root.join("integration"), integration)?;
            let dump = if shell == "bash" { "HISTTIMEFORMAT= history > \"$HOME/dump\"" } else { "print -rl -- \"${(@v)history}\" > \"$HOME/dump\"" };
            let startup = if shell == "zsh" { "setopt rcs" } else { ":" };
            let input = format!("HISTSIZE=200; SAVEHIST=100; {startup}\nsource \"$HOME/integration\"\n{dump}\necho NEW_SENTINEL\nexit 0\n");
            let mut child = command.env("HOME", root).env("HISTFILE", &global)
                .env("RMUX_TEST_EXECUTED", root.join("executed"))
                .env("RMUX_HISTORY_IMPORT", import).env("RMUX_SESSION_HISTORY", "1")
                .env("TERM_PROGRAM", "rmux").env("RMUX_SHELL_INTEGRATION", "1")
                .env_remove("BASH_ENV").env_remove("ZDOTDIR")
                .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
            child.stdin.take().unwrap().write_all(input.as_bytes())?;
            let output = child.wait_with_output()?;
            assert!(output.status.success(), "{shell}: {:?}", output.stderr);
            assert!(!root.join("executed").exists(), "{shell} executed imported history");
            let recalled = std::fs::read_to_string(root.join("dump"))?;
            assert!(recalled.contains("touch \"$RMUX_TEST_EXECUTED\""), "{shell}: {recalled}");
            assert!(recalled.contains("世界\nsecond line"), "{shell}: {recalled}");
            for entry in history.entries() {
                assert!(recalled.contains(entry), "{shell} lost {entry:?}: {recalled}");
            }
            let saved_bytes = std::fs::read(global)?;
            let saved = String::from_utf8_lossy(&saved_bytes);
            assert!(!saved.contains("RMUX_TEST_EXECUTED"), "{shell} duplicated imported history: {saved}");
            assert!(saved.contains("GLOBAL_SENTINEL"), "{shell}: {saved}");
            assert!(saved.contains("NEW_SENTINEL"), "{shell} lost new global history: {saved}");
        }
        Ok(())
    }
    #[test]
    fn automatic_nonlogin_bash_preserves_user_rc_and_cleans_wrapper() -> Result<()> {
        let home = tempfile::tempdir()?;
        std::fs::write(
            home.path().join(".bashrc"),
            "export RMUX_TEST_USER_RC=loaded\nPROMPT_COMMAND='printf user-hook'\n",
        )?;
        let mut builder = CommandBuilder::new("/bin/bash");
        let wrapper = prepare_bash(&mut builder)?;
        let output = std::process::Command::new("/bin/bash")
            .arg("--noprofile").arg("--rcfile").arg(wrapper.path().join("bashrc"))
            .args(["-i", "-c", "printf '%s|%s|%s|%s' \"$RMUX_TEST_USER_RC\" \"$_rmux_bash_loaded\" \"${RMUX_BASH_WRAPPER-unset}\" \"$PROMPT_COMMAND\""])
            .env("HOME", home.path()).env("TERM_PROGRAM", "rmux")
            .env("RMUX_SHELL_INTEGRATION", "1")
            .env("RMUX_BASH_WRAPPER", builder.get_env("RMUX_BASH_WRAPPER").unwrap()).output()?;
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout)?,
            "loaded|1|unset|_rmux_bash_capture_status\nprintf user-hook\n_rmux_bash_precmd"
        );
        let path = wrapper.path().to_owned();
        drop(wrapper);
        assert!(!path.exists());
        Ok(())
    }
    #[test]
    fn automatic_zsh_reports_directory_and_navigable_prompts_through_pty() -> Result<()> {
        check_prompt_navigation("zsh")
    }
    #[test]
    fn automatic_nonlogin_bash_reports_directory_and_navigable_prompts_through_pty() -> Result<()> {
        check_prompt_navigation("bash")
    }
    #[test]
    #[ignore = "requires mise-managed PowerShell"]
    fn automatic_powershell_reports_directory_and_navigable_prompts_through_pty() -> Result<()> {
        check_prompt_navigation("pwsh")
    }
    #[test]
    #[ignore = "requires mise-managed fish"]
    fn automatic_fish_reports_directory_and_navigable_prompts_through_pty() -> Result<()> {
        check_prompt_navigation("fish")
    }
    fn check_prompt_navigation(shell: &str) -> Result<()> {
        let home = tempfile::tempdir()?;
        let home_path = home.path().canonicalize()?;
        let destination = home_path.join("project 世界 % space");
        std::fs::create_dir(&destination)?;
        std::fs::write(home.path().join(format!(".{shell}rc")), "PS1='fixture> '\n")?;
        if shell == "fish" {
            let config = home.path().join(".config/fish");
            std::fs::create_dir_all(&config)?;
            std::fs::write(config.join("config.fish"), "function fish_prompt; printf 'fixture> '; end\n")?;
        }
        let profile_dir = home.path().join(".config/powershell");
        if shell == "pwsh" {
            std::fs::create_dir_all(&profile_dir)?;
            std::fs::write(
                profile_dir.join("Microsoft.PowerShell_profile.ps1"),
                "function global:prompt { 'fixture> ' }\n$env:RMUX_TEST_USER_RC='loaded'\n",
            )?;
        }
        let mut command = CommandBuilder::new(if matches!(shell, "pwsh" | "fish") {
            shell.to_owned()
        } else {
            format!("/bin/{shell}")
        });
        let _wrapper = if shell == "zsh" {
            command.args(["-d", "-i", "-l"]);
            prepare_zsh(&mut command, None)?
        } else if shell == "fish" {
            let wrapper = prepare_fish(&mut command)?;
            command.args(["-i", "-l"]);
            wrapper
        } else if shell == "pwsh" {
            prepare_powershell(&mut command, false)?
        } else {
            let wrapper = prepare_bash(&mut command)?;
            command.arg("-i");
            wrapper
        };
        command.env("HOME", home.path());
        command.env("XDG_CONFIG_HOME", home.path().join(".config"));
        command.env("TERM", "xterm-256color");
        command.env("TERM_PROGRAM", "rmux");
        command.env("RMUX_SHELL_INTEGRATION", "1");
        command.env("RMUX_TEST_DESTINATION", &destination);
        command.cwd(home.path());
        let session = crate::session::Session::spawn(command, 80, 4)?;
        let wait = |condition: &dyn Fn() -> Result<bool>| -> Result<()> {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
            while !condition()? {
                anyhow::ensure!(
                    std::time::Instant::now() < deadline,
                    "{shell} PTY fixture timed out: directory={:?}, frame={:?}, exit={:?}",
                    session.current_directory()?,
                    session.frame()?.text(),
                    session.exit_status()
                );
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Ok(())
        };
        wait(&|| Ok(session.current_directory()?.as_deref() == Some(home_path.as_path())))?;
        if shell == "pwsh" {
            session.input(b"Set-Location -LiteralPath $env:RMUX_TEST_DESTINATION; if ($env:RMUX_TEST_USER_RC -eq 'loaded' -and !$env:RMUX_POWERSHELL_WRAPPER -and $global:_rmux_command_running) { Write-Output ('changed-okay' + '-done') }\r")?;
        } else {
            session.input(
                b"cd -- \"$RMUX_TEST_DESTINATION\"; printf '%s%s\\n' changed-okay -done\r",
            )?;
        }
        wait(&|| {
            Ok(
                session.current_directory()?.as_deref() == Some(destination.as_path())
                    && session.frame()?.text().contains("changed-okay-done"),
            )
        })?;
        if shell == "pwsh" {
            session.input(b"1..12 | ForEach-Object { Write-Output (\"line-\" + $_) }\r")?;
        } else if shell == "fish" {
            session.input(b"printf 'line-%s\\n' (seq 1 12)\r")?;
        } else {
            session.input(b"printf 'line-%s\\n' {1..12}\r")?;
        }
        wait(&|| {
            let frame = session.frame()?;
            Ok(frame.text().contains("line-12") && frame.text().contains("fixture>"))
        })?;
        session.scroll(crate::Scroll::Top)?;
        let first = session.frame()?.scrollback_offset;
        session.scroll(crate::Scroll::Prompts(1))?;
        let second = session.frame()?.scrollback_offset;
        assert!(second < first, "shell markers must provide a later prompt");
        session.input(b"exit 0\r")?;
        wait(&|| Ok(session.exit_status().is_some()))?;
        Ok(())
    }
    #[test]
    fn zsh_startup_preserves_order_redirects_and_rcs_opt_out() -> Result<()> {
        for (explicit, redirect, disable) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            let home = tempfile::tempdir()?;
            let original = tempfile::tempdir()?;
            let redirected = tempfile::tempdir()?;
            let initial = if explicit {
                original.path()
            } else {
                home.path()
            };
            let final_directory = if redirect { redirected.path() } else { initial };
            let mut env_script = "typeset -g startup_order=env\n".to_owned();
            if redirect {
                env_script.push_str("export ZDOTDIR=$RMUX_TEST_REDIRECT\n");
            }
            if disable {
                env_script.push_str("unsetopt rcs\n");
            }
            std::fs::write(initial.join(".zshenv"), env_script)?;
            for (file, label) in [
                (".zprofile", "profile"),
                (".zshrc", "rc"),
                (".zlogin", "login"),
            ] {
                std::fs::write(
                    final_directory.join(file),
                    format!("startup_order+=:{label}\n"),
                )?;
            }
            let mut builder = CommandBuilder::new("/bin/zsh");
            let wrapper = prepare_zsh(
                &mut builder,
                explicit.then_some(original.path().as_os_str()),
            )?;
            let mut shell = std::process::Command::new("/bin/zsh");
            shell.args(["-d", "-i", "-l", "-c", "printf '%s|%s|%s|%s' $startup_order ${_rmux_integration_loaded-0} ${ZDOTDIR-unset} ${RMUX_ZSH_WRAPPER-unset}"]);
            shell
                .env("HOME", home.path())
                .env("TERM_PROGRAM", "rmux")
                .env("RMUX_SHELL_INTEGRATION", "1")
                .env("RMUX_TEST_REDIRECT", redirected.path());
            for key in [
                "ZDOTDIR",
                "RMUX_ZSH_ORIGINAL",
                "RMUX_ZSH_ORIGINAL_SET",
                "RMUX_ZSH_WRAPPER",
            ] {
                shell.env(key, builder.get_env(key).unwrap());
            }
            let output = shell.output()?;
            assert!(output.status.success(), "{:?}", output.stderr);
            let directory = if explicit || redirect {
                final_directory.display().to_string()
            } else {
                "unset".into()
            };
            let expected = format!(
                "{}|{}|{directory}|unset",
                if disable {
                    "env"
                } else {
                    "env:profile:rc:login"
                },
                if disable { 0 } else { 1 }
            );
            assert_eq!(String::from_utf8(output.stdout)?, expected);
            let path = wrapper.path().to_owned();
            drop(wrapper);
            assert!(!path.exists());
        }
        Ok(())
    }
}
