//! Utilities for (spawning) processes

use crate::{StringError, bait::ResultExt, bog::BogOkExt, define_collection_wrapper, ebog};
use cfg_if::cfg_if;
use log::{debug, trace};
use std::{
    env,
    ffi::{OsStr, OsString},
    path::Path,
    process::{Child, ChildStdout, Command, Stdio, exit},
    sync::LazyLock,
};

#[easy_ext::ext(ChildExt)]
impl Child {
    pub fn wait_for_code(&mut self) -> i32 {
        if let Some(status) = self.wait()._elog() {
            status.code().unwrap_or(1)
        } else {
            1
        }
    }
}

#[easy_ext::ext(CommandExt)]
impl Command {
    /// Use [`SHELL`] to create a command from a shell script
    /// On unix, the empty string is given to $0 so that subsequent args are fed to the script directly.
    pub fn from_script(script: &str) -> Self {
        let (shell, arg) = &*SHELL;

        let mut ret = Command::new(shell);

        #[cfg(unix)]
        ret.arg(arg).arg(script).arg(""); // the first argument is the program name which we leave empty
        #[cfg(not(unix))]
        ret.arg(arg).arg(script);

        ret
    }

    pub fn with_arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.arg(arg);
        self
    }

    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.args(args);
        self
    }

    /// Display the command.
    /// Does not escape arguments.
    pub fn display(&self) -> String {
        std::iter::once(self.get_program())
            .chain(self.get_args())
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Detach the command.
    /// Does nothing on unsupported platforms (not windows or unix).
    pub fn detach(&mut self) -> &mut Self {
        cfg_if! {
            if #[cfg(unix)] {
                use std::os::unix::process::CommandExt;

                unsafe {
                    self.pre_exec(|| {
                        // POSIX.1-2017 describes `fork` as async-signal-safe with the following note:
                        //
                        // > While the fork() function is async-signal-safe, there is no way for
                        // > an implementation to determine whether the fork handlers established by
                        // > pthread_atfork() are async-signal-safe. [...] It is therefore undefined for the
                        // > fork handlers to execute functions that are not async-signal-safe when fork()
                        // > is called from a signal handler.
                        //
                        // POSIX.1-2024 removes this guarantee and introduces an async-signal-safe
                        // replacement `_Fork`, which we'd like to use, but macOS doesn't support it yet.
                        //
                        // Since we aren't registering any fork handlers, and hopefully the OS doesn't
                        // either, we're fine on systems compatible with POSIX.1-2017, which should be
                        // enough for a long while. If this ever becomes a problem in the future, we should
                        // be able to switch to `_Fork`.
                        match libc::fork() {
                            -1 => (),
                            0 => (),
                            _ => libc::_exit(0),
                        }

                        if libc::setsid() == -1 {
                            // continue even if setsid fails
                        }
                        Ok(())
                    });
                }
            } else if #[cfg(windows)] {
                use std::os::windows::process::CommandExt;

                const DETACHED_PROCESS: u32 = 0x00000008;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

                self.creation_flags(
                    DETACHED_PROCESS // detaches console like CREATE_NO_WINDOW, but also eliminates io
                    | CREATE_NEW_PROCESS_GROUP
                );
            } else {
                log::info!("Failed to detach: unsupported platform")
            }
        }

        self
    }

    /// One-off spawn executable.
    /// Logs the command in debug builds.
    /// Prints error.
    pub fn spawn_detached(&mut self) -> Option<Child> {
        let ep = format!("Failed to spawn: {}", self.display());
        debug!("Spawning detached: {self:?}");

        self.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .detach();

        self.spawn().prefix(&ep)._ebog()
    }

    /// Spawn command with piped stdout and null stderr.
    /// Debug logs the command.
    pub fn spawn_piped(&mut self) -> Result<(Child, ChildStdout), StringError> {
        trace!("Spawning piped: {self:?}");

        let mut child = self
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .prefix(&format!("Failed to spawn: {}", self.display()))?;

        match child.stdout.take() {
            Some(stdout) => Ok((child, stdout)),

            None => Err(format!("No stdout for {}.", self.display()).into()),
        }
    }

    /// Run command and return full stdout as a String.
    /// If the command exits non-zero, returns an error including exit code and stderr.
    pub fn read_to_string(&mut self) -> Result<String, StringError> {
        trace!("Collecting output: {self:?}");

        let output = self
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .prefix(&format!("Failed to spawn: {}", self.display()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            let code = output
                .status
                .code()
                .map_or("None".to_string(), |c| c.to_string());

            return Err(format!(
                "{} exited with code {}: {}",
                self.display(),
                code,
                stderr.trim()
            )
            .into());
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|_| format!("{} produced non-UTF8 stdout", self.display()))?;

        Ok(stdout)
    }

    /// Naive check of whether a command succeeds. (i.e. health check).
    pub fn success(&mut self) -> bool {
        self.stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Platform-agnostic exec the command.
    ///
    /// Logs and displays errors.
    pub fn _exec(&mut self) -> ! {
        debug!("Becoming: {self:?}");

        #[cfg(not(windows))]
        {
            // replace current process
            use std::os::unix::process::CommandExt;
            let err = self.exec();

            ebog!("Could not exec {}: {err}", self.display());
            exit(1)
        }

        #[cfg(windows)]
        {
            match self.status() {
                Ok(status) => exit(
                    status
                        .code()
                        .unwrap_or(if status.success() { 0 } else { 1 }),
                ),
                Err(err) => {
                    ebog!("Could not exec {}: {err}", self.display());
                    exit(1)
                }
            }
        }
    }

    /// Spawn the command, with trace and error logging.
    pub fn _spawn(&mut self) -> Option<Child> {
        trace!("Spawning: {self:?}");
        self.spawn()
            .prefix(&format!("Could not spawn: {}", self.display()))
            ._elog()
    }
}

/// Quotes a argument for placement inside a string submitted to a shell.
/// Returns None if and only if not valid UTF-8.
/// Supports POSIX, pwsh, and cmd.exe.
///
/// ```rust,ignore
/// use std::process::Command;
/// use cba::broc::shell_quote;
///
/// let dangerous_input = "Tom & Jerry %PATH%";
/// let escaped_cmd_str = shell_quote(dangerous_input).unwrap();
/// let output = Command::new("cmd")
///    .arg("/c")
///    .arg(format!("echo {}", escaped_cmd_str))
///    .output()
///    .expect("failed to execute process");
/// ```
pub fn shell_quote(s: impl AsRef<OsStr>) -> Option<String> {
    s.as_ref().to_str().map(shell_quote_impl)
}

fn shell_quote_impl(s: &str) -> String {
    if cfg!(windows) {
        if &SHELL.0 == "cmd.exe" {
            let mut quoted = String::new();
            quoted.push('"');

            for c in s.chars() {
                match c {
                    '"' => quoted.push_str(r#""""#),

                    '&' | '|' | '<' | '>' | '^' => {
                        quoted.push('^');
                        quoted.push(c);
                    }

                    '%' => quoted.push_str("%%"),

                    _ => quoted.push(c),
                }
            }

            quoted.push('"');
            quoted.into()
        } else {
            // PowerShell: wrap in single quotes, escape internal single quotes by doubling them.
            // e.g., C:\Path's "With" Space -> 'C:\Path''s "With" Space'
            let escaped = s.replace('\'', "''");
            format!("'{}'", escaped).into()
        }
    } else if cfg!(unix) {
        // Unix shells: wrap in single quotes, escape internal single quotes
        // e.g., /path/it's/here -> '/path/it'\''s/here'
        let escaped = s.replace('\'', r"'\''");
        format!("'{}'", escaped).into()
    } else {
        s.into()
    }
}

/// Join arguments into a single string
/// Non-UTF-8 arguments are not escaped
/// cmd.exe is not supported
pub fn format_sh_command(inputs: &[impl AsRef<OsStr>], double: bool) -> OsString {
    let mut cmd = OsString::new();
    let mut first = true;

    for arg in inputs {
        if !first {
            cmd.push(" ");
        }
        first = false;

        let os = arg.as_ref();

        match os.to_str() {
            #[cfg(not(target_os = "windows"))]
            Some(s) => {
                if double {
                    let escaped = s
                        .replace("\\", "\\\\")
                        .replace('\"', "\\\"")
                        .replace("$", "\\$");
                    cmd.push("\"");
                    cmd.push(escaped);
                    cmd.push("\"");
                } else {
                    let escaped = s.replace('\'', "'\\''");
                    cmd.push("'");
                    cmd.push(escaped);
                    cmd.push("'");
                }
            }
            #[cfg(not(target_os = "windows"))]
            None => {
                // no shell-escape if not valid UTF-8 since there is no safe way to do it.
                cmd.push(os);
            }
            #[cfg(target_os = "windows")]
            Some(s) if &SHELL.0 != "cmd.exe" => {
                let escaped = s.replace('\'', "''");
                cmd.push("'");
                cmd.push(escaped);
                cmd.push("'");
            }
            #[cfg(target_os = "windows")]
            _ => {
                cmd.push(os);
            }
        }
    }

    cmd
}

pub fn display_sh_prog_and_args(prog: impl AsRef<OsStr>, args: &[impl AsRef<OsStr>]) -> String {
    format_sh_command(
        &{
            let mut i = vec![prog.as_ref()];
            i.extend(args.iter().map(|x| x.as_ref()));
            i
        },
        false,
    )
    .to_string_lossy()
    .to_string()
}

/// (shell path, shell arg)
pub static SHELL: LazyLock<(String, String)> = LazyLock::new(|| {
    #[cfg(windows)]
    {
        // PSModulePath is the most reliable indicator that we are in a PowerShell-configured environment
        if env::var("PSModulePath").is_ok() {
            // 1. Distribution channel is the official marker
            // 2. pwsh usually sets execution policy env vars or specific versioning
            let is_core = env::var("POWERSHELL_DISTRIBUTION_CHANNEL").is_ok()
                || env::var("PSExecutionPolicyPreference").is_ok();

            let exe = if is_core {
                "pwsh.exe"
            } else {
                "powershell.exe"
            };
            (exe.to_string(), "-Command".to_string())
        } else {
            // Fallback to COMSPEC for CMD
            let path = env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string());
            (path, "/C".to_string())
        }
    }
    #[cfg(unix)]
    {
        let path = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let flag = "-c".to_string();
        (path, flag)
    }
});

/// lowercase filename of [`SHELL`]
pub fn current_shell() -> String {
    if let Some(name) = Path::new(&SHELL.0).file_name().and_then(|n| n.to_str()) {
        return name.to_ascii_lowercase();
    }
    String::new()
}

pub static TTY_HANDLE: LazyLock<Option<std::fs::File>> = LazyLock::new(|| {
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .map_err(|e| {
            log::error!("Failed to open global /dev/tty: {e}");
            e
        })
        .ok()
});

/// Returns a single interactive Stdio handle cloned from the global TTY cache.
/// Falls back to inheriting standard streams if /dev/tty is unavailable.
pub fn tty_or_inherit() -> Stdio {
    if let Some(tty_file) = &*TTY_HANDLE {
        if let Ok(tty_clone) = tty_file.try_clone() {
            return Stdio::from(tty_clone);
        }
        log::error!("Failed to clone cached /dev/tty file descriptor");
    }

    Stdio::inherit()
}

use std::{cell::RefCell, collections::HashMap};
thread_local! {
    static HAS_CACHE: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
}

pub fn has(name: &str) -> bool {
    HAS_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(&found) = cache.get(name) {
            found
        } else {
            let found = which::which(name).is_ok();
            cache.insert(name.to_owned(), found);
            found
        }
    })
}

// ENV VARS
define_collection_wrapper!(
    #[cfg_attr(feature = "serde", derive(Debug, serde::Serialize, serde::Deserialize, Clone, PartialEq))]
    EnvVars: Vec<(String, String)>
);

#[macro_export]
macro_rules! env_vars {
    ($( $name:expr => $value:expr ),* $(,)?) => {
        $crate::broc::EnvVars::from(vec![
            $( ($name.into(), $value.to_string()) ),*
        ])
    };

}
impl EnvVars {
    pub fn set(&mut self, key: impl Into<String>, value: impl ToString) {
        let key = key.into();
        if let Some((_, v)) = self.iter_mut().find(|(k, _)| k == &key) {
            *v = value.to_string();
        } else {
            self.push((key, value.to_string()));
        }
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<&str> {
        let key = key.as_ref();

        self.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}
