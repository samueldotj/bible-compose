//! `biblecompose` — the headless build.
//!
//! Built at M0, not post-MVP. NFR-009 requires the parser, configuration
//! resolver, style resolver and emitter to be testable without launching the
//! GUI, and §16.1 wants golden intermediate-generation tests and PDF smoke
//! tests. The mechanism for all of that is this binary, so it exists from the
//! first milestone and is never allowed to break. SRS-REVIEW F9.
//!
//! No GUI crate is linked here, which is how that guarantee is structural
//! rather than a matter of discipline.

use std::process::ExitCode;

use biblecompose_app::{
    backend_version, build, emit, project, BuildEvent, BuildReporter, BuildRequest, BuildState,
    CancelToken, CONTRACT_VERSION,
};
use biblecompose_config::Settings;
use biblecompose_diagnostics::Diagnostics;
use biblecompose_diagnostics::Severity;
use biblecompose_scripture::fixtures;
use camino::{Utf8Path, Utf8PathBuf};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "biblecompose",
    about = "Compose USFM Scripture into a PDF",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Emit backend input without invoking the backend.
    ///
    /// The golden-file path: fast, hermetic, and needs no typesetter.
    Emit {
        /// A folder of USFM. Takes precedence over `--fixture`.
        #[arg(long)]
        books: Option<Utf8PathBuf>,
        /// A built-in fixture, for testing the pipeline without a project.
        #[arg(long, default_value = "john_1_1_5")]
        fixture: String,
        /// Write to a file rather than stdout.
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
    },

    /// Parse and validate without emitting or typesetting.
    Validate {
        /// A folder of USFM. Takes precedence over `--fixture`.
        #[arg(long)]
        books: Option<Utf8PathBuf>,
        #[arg(long, default_value = "john_1_1_5")]
        fixture: String,
    },

    /// Full pipeline: validate, emit, typeset, publish.
    Build {
        /// A folder of USFM. Takes precedence over `--fixture`.
        #[arg(long)]
        books: Option<Utf8PathBuf>,
        #[arg(long, default_value = "john_1_1_5")]
        fixture: String,
        /// Where the PDF goes. Never written until the build succeeds.
        ///
        /// Defaults to the project's own answer — `output/` inside the folder,
        /// named after the publication (BLD-003). A *path* is an argument to
        /// one command rather than a property of the project, which is why
        /// this exists and `output.file` does not; `output.name` in the
        /// settings decides what the file is called.
        #[arg(long, short)]
        output: Option<Utf8PathBuf>,
        /// Project root — relative asset paths resolve against it.
        #[arg(long, default_value = ".")]
        project: Utf8PathBuf,
        /// Directories the backend resolves classes from.
        #[arg(long = "sile-path")]
        sile_path: Vec<Utf8PathBuf>,
        /// Retain the generated XML and the build directory (BLD-008).
        #[arg(long)]
        keep_intermediates: bool,
        /// One event per line as JSON, for tests that assert the state
        /// sequence.
        #[arg(long)]
        events: bool,
        /// A proof rather than the publication: stamped on every page, and
        /// written beside the real PDF rather than over it (P5.4).
        #[arg(long)]
        draft: bool,
        /// Run the backend even if nothing that reaches it has changed
        /// (BLD-007).
        #[arg(long)]
        clean: bool,
    },

    /// Report the backend version (SILE-002).
    Version,

    /// List the built-in fixtures.
    Fixtures,
}

/// A folder if one was given, otherwise a built-in fixture.
///
/// The fixture path stays because it is how the pipeline is exercised with no
/// project and no parser — which is what M0 was built on and what the golden
/// tests still use.
fn document(books: Option<&Utf8Path>, fixture: &str) -> Result<project::Opened, String> {
    let Some(root) = books else {
        // The fixture path has no folder to read settings or styles from, so
        // it gets the built-in ones — which is also what CFG-001 and STY-001
        // promise a folder that has neither file.
        return Ok(project::Opened {
            root: Utf8PathBuf::from("."),
            settings: Settings::builtin(),
            styles: biblecompose_config::cascade::resolve(None, false).0,
            document: load(fixture)?,
            diagnostics: Diagnostics::new(),
            // A fixture has no folder, so there is nothing on disk to leave
            // out of it.
            left_out: Vec::new(),
        });
    };

    // Through `open` rather than calling settings, plan and load in order:
    // the window does the same, and two places composing those four steps is
    // two places that can end up disagreeing about what opening a project
    // means. It is also how the style sheet gets read at all.
    let opened = project::open(root);

    if opened.blocked() {
        for d in opened.diagnostics.iter() {
            print_diagnostic(&mut std::io::stderr(), d);
        }
        return Err(format!("{root} cannot be built"));
    }
    Ok(opened)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("biblecompose: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, String> {
    match cli.command {
        Command::Fixtures => {
            for name in fixtures::names() {
                println!("{name}");
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Version => {
            println!("biblecompose {}", env!("CARGO_PKG_VERSION"));
            println!("contract {CONTRACT_VERSION}");
            match backend_version() {
                Ok(v) => {
                    println!("backend {v}");
                    Ok(ExitCode::SUCCESS)
                }
                Err(d) => {
                    // A missing backend is reported, not fatal to `version`:
                    // knowing the application version is exactly what you want
                    // when the backend is what is broken.
                    print_diagnostic(&mut std::io::stderr(), &d);
                    Ok(ExitCode::FAILURE)
                }
            }
        }

        Command::Emit {
            books,
            fixture,
            output,
        } => {
            let opened = document(books.as_deref(), &fixture)?;
            let (doc, diagnostics) = (&opened.document, &opened.diagnostics);
            for d in diagnostics.iter() {
                print_diagnostic(&mut std::io::stderr(), d);
            }
            let emitted = emit(doc, &opened.styles);
            match output {
                Some(path) => std::fs::write(path.as_std_path(), emitted.xml.as_bytes())
                    .map_err(|e| format!("could not write {path}: {e}"))?,
                None => print!("{}", emitted.xml),
            }
            for marker in &emitted.unsupported {
                eprintln!("warning: \\{marker} is not supported and was not rendered");
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Validate { books, fixture } => {
            let opened = document(books.as_deref(), &fixture)?;
            let (doc, diagnostics) = (&opened.document, &opened.diagnostics);
            for d in diagnostics.iter() {
                print_diagnostic(&mut std::io::stdout(), d);
            }
            println!(
                "{} book(s), {} character(s) of Scripture",
                doc.books.len(),
                doc.text().len()
            );
            let dropped = doc.unsupported();
            for u in &dropped {
                println!("warning: unsupported marker \\{}", u.marker);
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Build {
            books,
            fixture,
            output,
            project,
            sile_path,
            keep_intermediates,
            events,
            draft,
            clean,
        } => {
            let opened = document(books.as_deref(), &fixture)?;
            let (doc, settings, load_diagnostics) = (
                &opened.document,
                opened.settings.clone(),
                &opened.diagnostics,
            );
            for d in load_diagnostics.iter() {
                print_diagnostic(&mut std::io::stderr(), d);
            }

            // `--output` wins; otherwise wherever the project says, which is
            // derived from its name when it has not said (BLD-003). Asked of
            // `Opened` rather than rebuilt here, so the CLI and the window
            // cannot disagree about where a publisher's PDF goes.
            let output = output.unwrap_or_else(|| opened.output());
            // The flag can turn keeping on but not off: a project that has
            // asked for intermediates is debugging something.
            let keep = keep_intermediates || *settings.output.keep_intermediates;

            let mut request = BuildRequest::new(project, output)
                .with_sile_path(sile_path)
                .keeping_intermediates(keep)
                .with_settings(settings)
                .with_styles(opened.styles.clone());
            request.clean = clean;
            request.prior = opened.diagnostics.clone();
            if draft {
                request.draft = Some(biblecompose_app::draft_note(doc.books.len()));
            }

            let (mut reporter, rx) = BuildReporter::new();
            let cancel = CancelToken::new();

            // The build runs on this thread; the reporter is drained after it
            // finishes. A GUI would drain concurrently — the point of the
            // event stream is that neither observer owns the build.
            let report = build(doc, &request, &cancel, &mut reporter);
            drop(reporter);

            for event in rx.iter() {
                print_event(&event, events);
            }

            if !events {
                for d in report.diagnostics.iter() {
                    let stream: &mut dyn std::io::Write = match d.severity {
                        Severity::Error => &mut std::io::stderr(),
                        _ => &mut std::io::stdout(),
                    };
                    print_diagnostic(stream, d);
                    // **The detail is the evidence**, and it was going
                    // nowhere. A diagnostic carries one exactly when the
                    // message is not enough by itself — a linker's own words,
                    // the signal a process died from, the line a parser
                    // stopped at — and the window shows it in a panel while
                    // the command line was dropping it on the floor. Two
                    // remote build failures were diagnosed without it before
                    // anyone noticed it was never printed.
                    if let Some(detail) = &d.detail {
                        for line in detail.lines() {
                            let _ = writeln!(stream, "  {line}");
                        }
                    }
                }
                if let Some(path) = &report.output {
                    println!("wrote {path}");
                }
            }

            Ok(match report.state {
                BuildState::Succeeded => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            })
        }
    }
}

fn load(name: &str) -> Result<biblecompose_scripture::ScriptureDocument, String> {
    fixtures::by_name(name).ok_or_else(|| {
        format!(
            "unknown fixture {name:?} — try one of: {}",
            fixtures::names().join(", ")
        )
    })
}

/// One diagnostic, with everything it carries.
///
/// **The detail was going nowhere.** A diagnostic carries one exactly when the
/// message is not enough by itself — a linker's own words, the signal a
/// process died from, the position a parser stopped at — and the window shows
/// it in a panel while the command line dropped it. Two remote build failures
/// were diagnosed without it before anyone noticed it was never printed.
///
/// Indented under the message rather than run together, because a detail is
/// often several lines and the ones that matter are usually the last.
fn print_diagnostic(stream: &mut dyn std::io::Write, d: &biblecompose_diagnostics::Diagnostic) {
    let _ = writeln!(stream, "{d}");
    if let Some(help) = &d.help {
        let _ = writeln!(stream, "  help: {help}");
    }
    if let Some(detail) = &d.detail {
        for line in detail.lines() {
            let _ = writeln!(stream, "  {line}");
        }
    }
}

fn print_event(event: &BuildEvent, as_json: bool) {
    if as_json {
        if let Ok(line) = serde_json::to_string(event) {
            println!("{line}");
        }
        return;
    }
    match event {
        BuildEvent::State(s) => println!("[{s}]"),
        BuildEvent::Log { stream, text } => println!("  {stream}: {text}"),
        BuildEvent::Backend(v) => println!("  backend: {v}"),
        BuildEvent::Output(p) => println!("  output: {p}"),
        BuildEvent::LogFile(p) => println!("  log: {p}"),
        // One line per page would be a wall of them on a whole Bible; the
        // window draws a bar from these, and a terminal already has the
        // backend's own `[1] [2]` in the log.
        BuildEvent::Pages { .. } => {}
        BuildEvent::Diagnostic(_) => {}
    }
}
