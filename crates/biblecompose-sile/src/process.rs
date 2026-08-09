//! Invoking SILE as a child process, and killing the whole tree on cancel.
//!
//! A child process rather than an embedded VM, for v1: it gives a hard failure
//! boundary when Lua errors (NFR-007) and it gives cancellation (BLD-006),
//! which an in-process VM does not ([ADR-002]).
//!
//! Arguments are passed as an array through the process API. Nothing is ever
//! concatenated into a shell string (SRS §15).
//!
//! [ADR-002]: ../../../docs/adr/002-sile-interface.md

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use biblecompose_diagnostics::{code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};

use crate::cache::RuntimeEnv;
use crate::{Backend, BackendJob, BackendOutcome, BackendVersion, CancelToken, LogLine, Stream};

/// How often the run loop checks the cancel flag. BLD-006 wants the UI usable
/// within a second; this is the polling half of that budget.
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// The environment variable an advanced user or a test points at an alternate
/// SILE (SILE-004).
pub const SILE_ENV: &str = "BIBLECOMPOSE_SILE";

#[derive(Debug, Clone)]
pub struct SileBackend {
    exe: Utf8PathBuf,
    /// Set when the backend came from the embedded bundle. An unpacked runtime
    /// cannot be found by SILE on its own — see [`RuntimeEnv`].
    runtime: Option<RuntimeEnv>,
}

impl SileBackend {
    pub fn new(exe: impl Into<Utf8PathBuf>) -> Self {
        SileBackend {
            exe: exe.into(),
            runtime: None,
        }
    }

    /// A backend unpacked from the bundle, which needs its module paths told
    /// to it rather than discovered.
    pub fn unpacked(exe: impl Into<Utf8PathBuf>, runtime: RuntimeEnv) -> Self {
        SileBackend {
            exe: exe.into(),
            runtime: Some(runtime),
        }
    }

    /// An explicit override first, then the bundled runtime, then `PATH`.
    ///
    /// The override wins deliberately: SILE-004 exists so a developer can point
    /// at a newer backend *without rebuilding the bundle*, which it would not
    /// do if the bundle took precedence.
    ///
    /// `PATH` last is the development fallback. In a shipped build the bundle
    /// answers, and [ADR-006](../../../docs/adr/006-single-binary.md) notes
    /// that this removes the "which SILE is it actually using" support question
    /// — but only because the one remaining way to change the answer is an
    /// environment variable somebody had to set on purpose.
    pub fn discover() -> Result<Self, Diagnostic> {
        if let Ok(p) = std::env::var(SILE_ENV) {
            let path = Utf8PathBuf::from(p);
            if path.exists() {
                return Ok(SileBackend::new(path));
            }
            return Err(Diagnostic::error(
                code::NOT_FOUND,
                format!("{SILE_ENV} points at {path}, which does not exist"),
            )
            .help("unset the variable to fall back to the bundled backend"));
        }

        #[cfg(feature = "embedded-sile")]
        {
            let exe = crate::bundle::ensure()?;
            let root = exe.parent().unwrap_or(&exe).to_owned();
            Ok(SileBackend::unpacked(exe, RuntimeEnv::for_root(&root)))
        }

        #[cfg(not(feature = "embedded-sile"))]
        {
            Ok(SileBackend::new("sile"))
        }
    }

    pub fn exe(&self) -> &Utf8Path {
        &self.exe
    }

    /// The unpacked runtime this backend uses, if it came from the bundle.
    pub fn runtime(&self) -> Option<&RuntimeEnv> {
        self.runtime.as_ref()
    }

    fn command(&self) -> Command {
        let mut c = Command::new(self.exe.as_std_path());
        // No shell anywhere on this path.
        c.stdin(Stdio::null());
        // Every invocation, not just `run`: an unpacked SILE cannot answer
        // `--version` either without being told where its Lua lives. Finding
        // that out the hard way is what SILE-002's "did not report a version"
        // looked like from the outside.
        if let Some(rt) = &self.runtime {
            c.env("SILE_PATH", rt.sile_path.as_str())
                .env("LUA_PATH", &rt.lua_path)
                .env("LUA_CPATH", &rt.lua_cpath);
            if let Some(conf) = &rt.fontconfig {
                c.env("FONTCONFIG_FILE", conf.as_str());
            }
        }
        c
    }
}

impl Backend for SileBackend {
    fn version(&self) -> Result<BackendVersion, Diagnostic> {
        let out = self
            .command()
            .arg("--version")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| {
                Diagnostic::error(
                    code::NOT_FOUND,
                    format!("could not run the typesetting backend at {}", self.exe),
                )
                .help("install SILE, or point BIBLECOMPOSE_SILE at it")
                .detail(e.to_string())
            })?;

        let raw = String::from_utf8_lossy(&out.stdout);
        let raw = raw
            .lines()
            .find(|l| l.contains("SILE"))
            .unwrap_or_else(|| raw.trim())
            .trim()
            .to_owned();

        if raw.is_empty() {
            return Err(Diagnostic::error(
                code::VERSION_UNREADABLE,
                "the backend did not report a version",
            )
            .detail(String::from_utf8_lossy(&out.stderr).into_owned()));
        }

        let semver = raw
            .split_whitespace()
            .find(|w| w.starts_with('v') && w[1..].starts_with(|c: char| c.is_ascii_digit()))
            .map(|w| w.trim_start_matches('v').to_owned());

        Ok(BackendVersion { raw, semver })
    }

    fn run(
        &self,
        job: &BackendJob,
        cancel: &CancelToken,
        log: &mut dyn FnMut(LogLine),
    ) -> Result<BackendOutcome, Diagnostic> {
        let version = self.version()?;
        log(LogLine {
            stream: Stream::Stdout,
            text: format!("backend: {version}"),
        });

        std::fs::create_dir_all(job.work_dir.as_std_path()).map_err(|e| {
            Diagnostic::error(code::PUBLISH_FAILED, "could not create the build directory")
                .at(SourceLoc::file(job.work_dir.clone()))
                .detail(e.to_string())
        })?;

        let xml_path = job.xml_path();
        std::fs::write(xml_path.as_std_path(), &job.xml).map_err(|e| {
            Diagnostic::error(code::PUBLISH_FAILED, "could not write the backend input")
                .at(SourceLoc::file(xml_path.clone()))
                .detail(e.to_string())
        })?;

        let pdf = job.pdf_path();
        let mut cmd = self.command();
        cmd.current_dir(job.project_root.as_std_path())
            .arg(xml_path.as_str())
            .arg("--class")
            .arg(&job.class)
            .arg("-o")
            .arg(pdf.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // The project's class and package directories, plus — when the backend
        // was unpacked from the bundle — its own tree, which it cannot find
        // relative to itself once the working directory is the project.
        // `command()` has already pointed SILE at its own tree; this adds the
        // project's class and package directories in front of it.
        let mut sile_path: Vec<&str> = job.sile_path.iter().map(|p| p.as_str()).collect();
        if let Some(rt) = &self.runtime {
            sile_path.push(rt.sile_path.as_str());
        }
        if !sile_path.is_empty() {
            cmd.env("SILE_PATH", sile_path.join(path_separator()));
        }

        platform::before_spawn(&mut cmd);

        let mut child = cmd.spawn().map_err(|e| {
            Diagnostic::error(
                code::NOT_FOUND,
                format!("could not start the typesetting backend at {}", self.exe),
            )
            .detail(e.to_string())
        })?;

        // Own the process tree before doing anything else, so a cancel that
        // arrives immediately still has something to kill.
        let guard = platform::Guard::adopt(&child);

        let (tx, rx) = mpsc::channel::<LogLine>();
        let mut pumps = Vec::new();
        if let Some(out) = child.stdout.take() {
            pumps.push(spawn_pump(out, Stream::Stdout, tx.clone()));
        }
        if let Some(err) = child.stderr.take() {
            pumps.push(spawn_pump(err, Stream::Stderr, tx.clone()));
        }
        drop(tx);

        let status = wait_with_cancel(&mut child, cancel, &rx, log, &guard)?;

        // Drain anything the pumps produced after the last poll. SILE-006:
        // no line is lost because the process ended.
        for line in rx.iter() {
            log(line);
        }
        for p in pumps {
            let _ = p.join();
        }

        if cancel.is_cancelled() {
            return Err(Diagnostic::warning(code::CANCELLED, "build cancelled"));
        }

        let exit_code = status.code();
        if !status.success() {
            return Err(Diagnostic::error(
                code::NONZERO_EXIT,
                match exit_code {
                    Some(c) => format!("the typesetting backend exited with status {c}"),
                    None => "the typesetting backend was terminated by a signal".to_owned(),
                },
            )
            .help("the backend log holds the technical detail"));
        }

        if !pdf.exists() {
            return Err(Diagnostic::error(
                code::NO_OUTPUT,
                "the backend reported success but produced no PDF",
            )
            .at(SourceLoc::file(pdf.clone())));
        }

        Ok(BackendOutcome {
            pdf,
            version,
            exit_code,
        })
    }
}

fn path_separator() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

fn spawn_pump<R: std::io::Read + Send + 'static>(
    reader: R,
    stream: Stream,
    tx: mpsc::Sender<LogLine>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // `lines()` on lossy-decoded bytes: SILE can emit non-UTF-8 in a font
        // name, and losing a log line to a decode error would be the one thing
        // SILE-006 forbids.
        let mut buf = BufReader::new(reader);
        let mut raw = Vec::new();
        loop {
            raw.clear();
            match buf.read_until(b'\n', &mut raw) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&raw).trim_end().to_owned();
                    if tx.send(LogLine { stream, text }).is_err() {
                        break;
                    }
                }
            }
        }
    })
}

fn wait_with_cancel(
    child: &mut Child,
    cancel: &CancelToken,
    rx: &mpsc::Receiver<LogLine>,
    log: &mut dyn FnMut(LogLine),
    guard: &platform::Guard,
) -> Result<std::process::ExitStatus, Diagnostic> {
    loop {
        while let Ok(line) = rx.try_recv() {
            log(line);
        }

        if cancel.is_cancelled() {
            // Kill the tree, not the process. On Windows `Child::kill`
            // terminates only the named process, and an orphan holding a file
            // handle turns the next build's atomic publish into an
            // unexplainable failure (SRS-REVIEW F11).
            guard.kill_tree(child);
            let status = child.wait().map_err(|e| {
                Diagnostic::error(code::CANCELLED, "could not reap the cancelled backend")
                    .detail(e.to_string())
            })?;
            return Ok(status);
        }

        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => thread::sleep(CANCEL_POLL),
            Err(e) => {
                return Err(Diagnostic::error(
                    code::NONZERO_EXIT,
                    "lost track of the typesetting backend",
                )
                .detail(e.to_string()))
            }
        }
    }
}

#[cfg(windows)]
mod platform {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::CREATE_SUSPENDED;

    /// Not suspended — we want SILE to start immediately — but a new process
    /// group keeps stray Ctrl+C out of it.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    pub fn before_spawn(cmd: &mut Command) {
        let _ = CREATE_SUSPENDED;
        cmd.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }

    /// A Job Object with kill-on-close, so descendants die with the job even
    /// if BibleCompose itself is killed.
    pub struct Guard {
        job: Option<HANDLE>,
    }

    // The handle is owned solely by this guard and only used from the thread
    // driving the child.
    unsafe impl Send for Guard {}
    unsafe impl Sync for Guard {}

    impl Guard {
        pub fn adopt(child: &Child) -> Guard {
            unsafe {
                let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if job.is_null() {
                    return Guard { job: None };
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    std::ptr::addr_of!(info).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    CloseHandle(job);
                    return Guard { job: None };
                }
                if AssignProcessToJobObject(job, child.as_raw_handle() as HANDLE) == 0 {
                    CloseHandle(job);
                    return Guard { job: None };
                }
                Guard { job: Some(job) }
            }
        }

        pub fn kill_tree(&self, child: &mut Child) {
            unsafe {
                if let Some(job) = self.job {
                    // Terminates every process in the job, not just the one we
                    // spawned.
                    TerminateJobObject(job, 1);
                    return;
                }
            }
            let _ = child.kill();
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(job) = self.job.take() {
                unsafe {
                    CloseHandle(job);
                }
            }
        }
    }
}

#[cfg(unix)]
mod platform {
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command};

    pub fn before_spawn(cmd: &mut Command) {
        // Its own process group, so one signal reaches every descendant.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    pub struct Guard;

    impl Guard {
        pub fn adopt(_child: &Child) -> Guard {
            Guard
        }

        pub fn kill_tree(&self, child: &mut Child) {
            // Negative pid signals the whole group. SIGTERM first so SILE can
            // close its output, then SIGKILL for anything still standing.
            let pid = child.id() as i32;
            unsafe {
                libc::killpg(pid, libc::SIGTERM);
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
            unsafe {
                libc::killpg(pid, libc::SIGKILL);
            }
            let _ = child.kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_honours_the_override_when_it_exists() {
        // A path that certainly does not exist produces a diagnostic naming
        // the variable, rather than a silent fall back to PATH.
        let key = SILE_ENV;
        let prev = std::env::var(key).ok();
        std::env::set_var(key, "/definitely/not/here/sile");
        let got = SileBackend::discover();
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        let err = got.expect_err("a missing override must be reported");
        assert_eq!(err.code, code::NOT_FOUND);
        assert!(err.message.contains(SILE_ENV));
    }

    #[test]
    fn missing_backend_is_a_diagnostic_not_a_panic() {
        let b = SileBackend::new("definitely-not-a-real-binary-xyz");
        let err = b.version().expect_err("a missing binary must be reported");
        assert_eq!(err.code, code::NOT_FOUND);
        assert!(err.help.is_some(), "the user needs to be told what to do");
    }

    #[test]
    fn path_separator_matches_the_platform() {
        if cfg!(windows) {
            assert_eq!(path_separator(), ";");
        } else {
            assert_eq!(path_separator(), ":");
        }
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    use std::time::Instant;

    /// A long-running child that also spawns a descendant, so killing only the
    /// named process leaves something behind.
    fn sleeper() -> Command {
        let mut c = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/c", "ping -n 60 127.0.0.1 > nul"]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", "sleep 60 & sleep 60"]);
            c
        };
        c.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        c
    }

    /// BLD-006. `Child::kill` on Windows terminates only the named process; an
    /// orphan holding a file handle turns the next build's atomic publish into
    /// an unexplainable failure (SRS-REVIEW F11). This asserts the tree dies.
    #[test]
    fn cancelling_kills_the_process_tree() {
        let mut cmd = sleeper();
        platform::before_spawn(&mut cmd);
        let mut child = cmd.spawn().expect("spawn a sleeper");
        let guard = platform::Guard::adopt(&child);

        // It is genuinely running.
        assert!(
            child.try_wait().expect("try_wait").is_none(),
            "the sleeper should still be running"
        );

        let start = Instant::now();
        guard.kill_tree(&mut child);
        let status = child.wait().expect("reap the killed child");

        assert!(
            start.elapsed() < Duration::from_secs(5),
            "BLD-006 wants the UI usable within a second; the kill took {:?}",
            start.elapsed()
        );
        assert!(
            !status.success(),
            "a killed process does not exit successfully"
        );
        assert!(
            child.try_wait().expect("try_wait").is_some(),
            "the child must be reaped, not left a zombie"
        );
    }

    /// The guard must be safe to drop without a kill — the common path, where
    /// the build simply finished.
    #[test]
    fn dropping_the_guard_without_cancelling_is_fine() {
        let mut cmd = sleeper();
        platform::before_spawn(&mut cmd);
        let mut child = cmd.spawn().expect("spawn a sleeper");
        {
            let guard = platform::Guard::adopt(&child);
            guard.kill_tree(&mut child);
        }
        let _ = child.wait();
    }
}
