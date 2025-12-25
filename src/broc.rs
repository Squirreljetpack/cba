//! Utilities for (spawning) processes

use crate::{bog::BogOkExt, ebog, misc::ResultExt};
use cfg_if::cfg_if;
use std::{
    env,
    ffi::{OsStr, OsString},
    process::{Child, ChildStdout, Command, Stdio},
    sync::LazyLock,
};

/// Execute script using shell and display error
pub fn spawn_script(
    script: &str,
    vars: impl IntoIterator<Item = (String, String)>,
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
) -> Option<Child> {
    let (shell, arg) = &*SHELL;

    Command::new(shell)
        .arg(arg)
        .arg(script)
        .envs(vars)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .prefix_err(&format!("Could not spawn: {script}"))
        .or_err()
}

pub fn exec_script(script: &str, vars: impl IntoIterator<Item = (String, String)>) -> ! {
    let (shell, arg) = &*SHELL;

    let mut cmd = Command::new(shell);
    cmd.arg(arg).arg(script).envs(vars);

    #[cfg(not(windows))]
    {
        // replace current process

        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        use std::process::exit;

        ebog!("Could not exec {script:?}: {err}");
        exit(1);
    }

    #[cfg(windows)]
    {
        match command.status() {
            Ok(status) => {
                exit(
                    status
                        .code()
                        .unwrap_or(if status.success() { 0 } else { 1 }),
                );
            }
            Err(err) => {
                ebog!("Could not exec {cmd:?}: {err}");
                exit(1);
            }
        }
    }
}

/// One-off spawn executable
pub fn spawn_detached(cmd: &mut Command) -> Option<Child> {
    let err_prefix = format!(
        "Failed to spawn: {}",
        format_sh_command({
            let mut inputs = vec![cmd.get_program()];
            inputs.extend(cmd.get_args());
            inputs
        })
        .to_string_lossy()
    );

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    cfg_if! {
        if #[cfg(unix)] {
            use std::os::unix::process::CommandExt;

            unsafe {
                cmd.pre_exec(|| {
                    libc::setsid(); // continue even if setsid fails
                    Ok(())
                });
            }
        } else if #[cfg(windows)] {
            use std::os::windows::process::CommandExt;

            const DETACHED_PROCESS: u32 = 0x00000008;
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

            cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        } else {
            return None;
        }
    }

    cmd.spawn().prefix_err(&err_prefix).or_err()
}

pub fn spawn_piped(cmd: &mut Command) -> Result<ChildStdout, String> {
    let err_prefix = format!(
        "Failed to spawn: {}",
        format_sh_command({
            let mut inputs = vec![cmd.get_program()];
            inputs.extend(cmd.get_args());
            inputs
        })
        .to_string_lossy()
    );

    match cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .prefix_err(&err_prefix)?
        .stdout
        .take()
    {
        Some(s) => Ok(s),
        None => Err(err_prefix), // stdout failure has no reason suffix
    }
}

/// Join arguments into a single string
/// Non-UTF-8 arguments are not escaped
/// Todo: support windows
pub fn format_sh_command(inputs: Vec<impl AsRef<OsStr>>) -> OsString {
    let mut cmd = OsString::new();
    let mut first = true;

    for arg in inputs {
        if !first {
            cmd.push(" ");
        }
        first = false;

        let os = arg.as_ref();

        match os.to_str() {
            Some(s) => {
                // shell-escape only when valid UTF-8
                let escaped = s.replace('\'', "'\\''");
                cmd.push("'");
                cmd.push(escaped);
                cmd.push("'");
            }
            None => {
                cmd.push(os);
            }
        }
    }

    cmd
}

// SHELL
pub static SHELL: LazyLock<(String, String)> = LazyLock::new(|| {
    #[cfg(windows)]
    {
        let path = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
        let flag = if path.to_lowercase().contains("powershell") {
            "-Command".to_string()
        } else {
            "/C".to_string()
        };
        (path, flag)
    }
    #[cfg(unix)]
    {
        let path = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let flag = "-c".to_string();
        log::debug!("SHELL: {}, {}", path, flag);
        (path, flag)
    }
});

// ENV VARS
pub type EnvVars = Vec<(String, String)>;

#[macro_export]
macro_rules! env_vars {
    ($( $name:expr => $value:expr ),* $(,)?) => {
        Vec::<(String, String)>::from([
            $( ($name.into(), $value.into()) ),*
            ]
        )
    };
}
