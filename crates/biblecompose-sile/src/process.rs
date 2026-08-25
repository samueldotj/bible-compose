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

use std::collections::VecDeque;
use std::io::{BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use biblecompose_diagnostics::{code, Diagnostic, SourceLoc};
use camino::{Utf8Path, Utf8PathBuf};

use crate::cache::RuntimeEnv;
use crate::{
    Backend, BackendEvent, BackendJob, BackendOutcome, BackendVersion, CancelToken, LogLine, Stream,
};

/// How often the run loop checks the cancel flag. BLD-006 wants the UI usable
/// within a second; this is the polling half of that budget.
const CANCEL_POLL: Duration = Duration::from_millis(50);

/// The environment variable an advanced user or a test points at an alternate
/// SILE (SILE-004).
/// How many lines of backend output are kept for classifying a failure.
///
/// Generous: SILE's stack traces are long and the useful line can be well
/// above the last one. Bounded all the same, because a run that fails after a
/// hundred thousand lines should not also exhaust memory.
const KEEP_LINES: usize = 200;

pub const SILE_ENV: &str = "BIBLECOMPOSE_SILE";

/// Extra class and package directories, path-separated.
///
/// The companion to [`SILE_ENV`], and needed for the same reason: SILE-004
/// lets a developer swap the *executable* without rebuilding the bundle, and
/// the class is released with the application, so swapping one without the
/// other leaves a new class option being read by an old class — which SILE
/// reports as "attempted to set an undeclared class option", a message about
/// the wrong thing entirely.
///
/// Highest priority of everything, because that is what an override means.
pub const SILE_PATH_ENV: &str = "BIBLECOMPOSE_SILE_PATH";

#[derive(Debug, Clone)]
pub struct SileBackend {
    exe: Utf8PathBuf,
    /// Set when the backend came from the embedded bundle. An unpacked runtime
    /// cannot be found by SILE on its own — see [`RuntimeEnv`].
    runtime: Option<RuntimeEnv>,
}

/// Whether a directory holds SILE's own Lua tree rather than just an
/// executable.
///
/// `core/pathsetup.lua` is the file SILE loads before anything else, and the
/// one whose absence produces "cannot open ./core/pathsetup.lua" — so it is
/// both the marker and the thing being fixed.
fn self_contained(dir: &Utf8Path) -> bool {
    dir.join("core").join("pathsetup.lua").exists()
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
                // A SILE that carries its own tree — an unpacked bundle, or a
                // build directory — cannot find its Lua unless it is told
                // where. Without this the override answers SILE-002's "the
                // backend did not report a version", which is a true statement
                // about a backend that is sitting right there and works.
                //
                // A system install has nothing beside it and gets no
                // environment, which is what it wants: its paths are compiled
                // in and inventing some would override them.
                let dir = path.parent().unwrap_or(&path).to_owned();
                return Ok(match self_contained(&dir) {
                    true => SileBackend::unpacked(path, RuntimeEnv::for_root(&dir)),
                    false => SileBackend::new(path),
                });
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
    fn font_dirs(&self) -> Vec<Utf8PathBuf> {
        // The unpacked runtime's own font directory, which is also the first
        // entry in the fontconfig file it is given — so this is the same
        // preference order the backend itself applies.
        self.runtime
            .as_ref()
            .map(|rt| vec![rt.sile_path.join("fonts")])
            .unwrap_or_default()
    }

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
        report: &mut dyn FnMut(BackendEvent),
    ) -> Result<BackendOutcome, Diagnostic> {
        let version = self.version()?;
        report(BackendEvent::Log(LogLine {
            stream: Stream::Stdout,
            text: format!("backend: {version}"),
        }));

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

        // **Absolute, because the backend does not run where this does.**
        //
        // The working directory below is the *project*, so that a relative
        // asset path in the document means what the project means by it. Every
        // other path handed over is therefore relative to something the
        // backend has already stopped standing in: a project given as
        // `--project smoke` puts the build directory at
        // `smoke/.biblecompose/…`, and the backend, already inside `smoke`,
        // looks for `smoke/smoke/.biblecompose/…` and reports a file it cannot
        // find. Absolute on a machine where the project was named absolutely,
        // which is why this went unseen until a release ran from a parent
        // directory.
        //
        // `absolute` rather than `canonicalize`: the PDF does not exist yet, so
        // there is nothing to resolve, and on Windows `canonicalize` returns a
        // `Y:` path that not every tool reads.
        let here = |path: &Utf8Path| -> Utf8PathBuf {
            std::path::absolute(path.as_std_path())
                .ok()
                .and_then(|p| Utf8PathBuf::from_path_buf(p).ok())
                .unwrap_or_else(|| path.to_owned())
        };
        let xml_arg = here(&xml_path);
        let pdf_arg = here(&pdf);

        let mut cmd = self.command();
        cmd.current_dir(job.project_root.as_std_path())
            .arg(xml_arg.as_str())
            .arg("--class")
            .arg(&job.class)
            .arg("-o")
            .arg(pdf_arg.as_str())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Resolved settings, in the order the job listed them.
        for (key, value) in &job.class_options {
            cmd.arg("-O").arg(format!("{key}={value}"));
        }

        // The project's class and package directories, plus — when the backend
        // was unpacked from the bundle — its own tree, which it cannot find
        // relative to itself once the working directory is the project.
        //
        // **The runtime goes first and the project's directories last, because
        // SILE's `SILE_PATH` is a last-wins list, not a first-wins one.** Its
        // `core/pathsetup.lua` *prepends* each entry in turn, so the entry
        // written last ends up with the highest priority. Written the intuitive
        // way round, the runtime's own copy of a class silently shadowed the
        // project's — which is how this was found: a class option added to
        // `sile/classes/biblecompose.lua` was rejected as undeclared while the
        // unpacked copy was in force.
        let mut sile_path: Vec<&str> = Vec::new();
        if let Some(rt) = &self.runtime {
            sile_path.push(rt.sile_path.as_str());
        }
        sile_path.extend(job.sile_path.iter().map(|p| p.as_str()));

        // Last, so it outranks both.
        let override_path = std::env::var(SILE_PATH_ENV).unwrap_or_default();
        sile_path.extend(
            override_path
                .split(path_separator())
                .map(str::trim)
                .filter(|s| !s.is_empty()),
        );
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

        // What the backend said, kept so a failure can be read rather than
        // guessed at (P5.8). Bounded: a run that fails after writing a hundred
        // thousand lines should not also exhaust memory, and only the tail is
        // ever used — SILE writes the error and then unwinds, so the end is
        // where the failure is.
        let mut said: VecDeque<String> = VecDeque::new();
        let keep = |said: &mut VecDeque<String>, line: &str| {
            if said.len() >= KEEP_LINES {
                said.pop_front();
            }
            said.push_back(line.to_owned());
        };

        let (tx, rx) = mpsc::channel::<BackendEvent>();
        let mut pumps = Vec::new();
        if let Some(out) = child.stdout.take() {
            pumps.push(spawn_pump(out, Stream::Stdout, tx.clone()));
        }
        if let Some(err) = child.stderr.take() {
            pumps.push(spawn_pump(err, Stream::Stderr, tx.clone()));
        }
        drop(tx);

        let status = {
            let said = &mut said;
            let report = &mut |event: BackendEvent| {
                if let BackendEvent::Log(line) = &event {
                    keep(said, &line.text);
                }
                report(event);
            };
            wait_with_cancel(&mut child, cancel, &rx, report, &guard)?
        };

        // Drain anything the pumps produced after the last poll. SILE-006:
        // no line is lost because the process ended.
        for event in rx.iter() {
            if let BackendEvent::Log(line) = &event {
                keep(&mut said, &line.text);
            }
            report(event);
        }
        for p in pumps {
            let _ = p.join();
        }

        if cancel.is_cancelled() {
            return Err(Diagnostic::warning(code::CANCELLED, "build cancelled"));
        }

        let exit_code = status.code();
        if !status.success() {
            let log: Vec<&str> = said.iter().map(String::as_str).collect();
            let log = log.join("\n");
            // A failure the table knows, said in terms a publisher can act on
            // (SILE-007). Anything it does not know still surfaces, with the
            // same raw tail attached — the table lists what has been seen and
            // does not decide what is worth reporting.
            if let Some(mapped) = crate::failure::classify(&log) {
                return Err(mapped);
            }
            return Err(Diagnostic::error(
                code::NONZERO_EXIT,
                match exit_code {
                    Some(c) => format!("the typesetting backend exited with status {c}"),
                    None => "the typesetting backend was terminated by a signal".to_owned(),
                },
            )
            .help("the backend log holds the technical detail")
            .detail(crate::failure::tail(&log)));
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
    tx: mpsc::Sender<BackendEvent>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buf = BufReader::new(reader);
        let mut chunk = [0u8; 4096];
        // What has arrived since the last newline. Bytes rather than a string:
        // SILE can emit non-UTF-8 in a font name, and losing a log line to a
        // decode error would be the one thing SILE-006 forbids.
        let mut pending: Vec<u8> = Vec::new();
        // How much of `pending` has already been searched for page markers, so
        // a growing tail is not rescanned from the start on every read.
        let mut scanned = 0usize;

        loop {
            let read = match buf.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            pending.extend_from_slice(&chunk[..read]);

            // Progress first, because it is the whole reason this reads bytes
            // rather than lines: SILE writes `[12] ` for each finished page
            // *without a newline*, so a line-buffered reader delivers the
            // entire run's worth in one go when the document ends — which is
            // to say, never, from the point of view of someone watching.
            if let Some((page, upto)) = last_page_marker(&pending[scanned..]) {
                scanned += upto;
                if tx.send(BackendEvent::Page(page)).is_err() {
                    break;
                }
            }

            // Then whole lines, unchanged: the log is still exactly what the
            // backend wrote, page markers included.
            while let Some(at) = pending.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = pending.drain(..=at).collect();
                scanned = 0;
                let text = String::from_utf8_lossy(&line).trim_end().to_owned();
                if tx
                    .send(BackendEvent::Log(LogLine { stream, text }))
                    .is_err()
                {
                    return;
                }
            }
        }

        // Whatever the process left without a trailing newline.
        if !pending.is_empty() {
            let text = String::from_utf8_lossy(&pending).trim_end().to_owned();
            if !text.is_empty() {
                let _ = tx.send(BackendEvent::Log(LogLine { stream, text }));
            }
        }
    })
}

/// The highest complete `[n] ` marker in `bytes`, and how far to skip.
///
/// The *highest* rather than each in turn: a slow reader can take one chunk
/// holding twenty pages, and twenty events would be twenty repaints of a bar
/// that only needs its last position. A marker is only counted once its
/// closing bracket has arrived, so a page number is never reported half-read.
fn last_page_marker(bytes: &[u8]) -> Option<(u32, usize)> {
    let mut found = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        let mut value: u32 = 0;
        let mut digits = 0;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            value = value
                .saturating_mul(10)
                .saturating_add(u32::from(bytes[j] - b'0'));
            digits += 1;
            j += 1;
        }
        if digits > 0 && j < bytes.len() && bytes[j] == b']' {
            found = Some((value, j + 1));
            i = j + 1;
        } else {
            i += 1;
        }
    }
    found
}

fn wait_with_cancel(
    child: &mut Child,
    cancel: &CancelToken,
    rx: &mpsc::Receiver<BackendEvent>,
    report: &mut dyn FnMut(BackendEvent),
    guard: &platform::Guard,
) -> Result<std::process::ExitStatus, Diagnostic> {
    loop {
        while let Ok(event) = rx.try_recv() {
            report(event);
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
