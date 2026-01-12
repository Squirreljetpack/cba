//! Utilities for (spawning) processes

use crate::{bait::ResultExt, bog::BogOkExt, ebog};
use cfg_if::cfg_if;
use log::{debug, trace};
use std::{
    env,
    ffi::{OsStr, OsString},
    process::{Child, ChildStdout, Command, Stdio},
    sync::LazyLock,
};

#[easy_ext::ext(CommandExt)]
impl Command {
    /// Use [`SHELL`] to create a command from a shell script
    /// On unix, the empty string is given to $0 so that subsequent args are fed to the script directly.
    /// On windows (todo)
    pub fn from_script(script: &str) -> Self {
        let (shell, arg) = &*SHELL;

        let mut ret = Command::new(shell);

        ret.arg(arg).arg(script).arg(""); //

        ret
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
                        libc::setsid(); // continue even if setsid fails
                        Ok(())
                    });
                }
            } else if #[cfg(windows)] {
                use std::os::windows::process::CommandExt;

                const DETACHED_PROCESS: u32 = 0x00000008;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

                self.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
            } else {
                log::info!("Failed to detach: unsupported platform")
            }
        }

        self
    }

    /// One-off spawn executable
    /// Logs the command in debug builds
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

    /// Spawn command with piped stdout
    /// Debug logs the command
    pub fn spawn_piped(&mut self) -> Result<ChildStdout, String> {
        debug!("Spawning piped: {self:?}");

        match self
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .prefix(&format!("Failed to spawn: {}", self.display()))?
            .stdout
            .take()
        {
            Some(s) => Ok(s),
            None => Err(format!("No stdout for {}.", self.display())), // stdout failure has no reason suffix
        }
    }

    /// Naive check of whether a command succeeds. (i.e. health check)
    pub fn success(&mut self) -> bool {
        self.stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Platform-agnostic exec the command
    ///
    /// Logs and displays errors.
    pub fn _exec(&mut self) -> ! {
        debug!("Becoming: {self:?}");

        #[cfg(not(windows))]
        {
            // replace current process
            use std::os::unix::process::CommandExt;
            let err = self.exec();
            use std::process::exit;

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
                    ebog!("Could not exec {}: {err}", , self.display());
                    exit(1)
                }
            }
        }
    }

    /// Spawn the command, logging errors.
    pub fn _spawn(&mut self) -> Option<Child> {
        trace!("Spawning: {self:?}");
        self.spawn()
            .prefix(&format!("Could not spawn: {}", self.display()))
            ._elog()
    }
}

/// Join arguments into a single string
/// Non-UTF-8 arguments are not escaped
/// Todo: support windows
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
            None => {
                // no shell-escape if not valid UTF-8 since there is no safe way to do it.
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

pub fn tty_or_inherit() -> Stdio {
    if let Ok(mut tty) = std::fs::File::open("/dev/tty") {
        let _ = std::io::Write::flush(&mut tty); // does nothing but seems logical
        Stdio::from(tty)
    } else {
        log::error!("Failed to open /dev/tty");
        Stdio::inherit()
    }
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
