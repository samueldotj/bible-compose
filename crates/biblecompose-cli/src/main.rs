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
    backend_version, build, emit, BuildEvent, BuildReporter, BuildRequest, BuildState, CancelToken,
    CONTRACT_VERSION,
};
use biblecompose_diagnostics::Diagnostics;
use biblecompose_diagnostics::Severity;
use biblecompose_scripture::fixtures;
use biblecompose_scripture::plan::BookPlan;
use biblecompose_scripture::ScriptureDocument;
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
        #[arg(long, short, default_value = "output.pdf")]
        output: Utf8PathBuf,
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
fn document(
    books: Option<&Utf8Path>,
    fixture: &str,
) -> Result<(ScriptureDocument, Diagnostics), String> {
    match books {
        Some(root) => {
            let loaded = biblecompose_app::project::load(root, &BookPlan::canonical());
            if loaded.blocked() {
                for d in loaded.diagnostics.iter() {
                    eprintln!("{d}");
                }
                return Err(format!("{root} cannot be built"));
            }
            Ok((loaded.document, loaded.diagnostics))
        }
        None => Ok((load(fixture)?, Diagnostics::new())),
    }
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
                    eprintln!("{d}");
                    Ok(ExitCode::FAILURE)
                }
            }
        }

        Command::Emit {
            books,
            fixture,
            output,
        } => {
            let (doc, diagnostics) = document(books.as_deref(), &fixture)?;
            for d in diagnostics.iter() {
                eprintln!("{d}");
            }
            let emitted = emit(&doc);
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
            let (doc, diagnostics) = document(books.as_deref(), &fixture)?;
            for d in diagnostics.iter() {
                println!("{d}");
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
        } => {
            let (doc, load_diagnostics) = document(books.as_deref(), &fixture)?;
            for d in load_diagnostics.iter() {
                eprintln!("{d}");
            }
            let request = BuildRequest::new(project, output)
                .with_sile_path(sile_path)
                .keeping_intermediates(keep_intermediates);

            let (mut reporter, rx) = BuildReporter::new();
            let cancel = CancelToken::new();

            // The build runs on this thread; the reporter is drained after it
            // finishes. A GUI would drain concurrently — the point of the
            // event stream is that neither observer owns the build.
            let report = build(&doc, &request, &cancel, &mut reporter);
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
                    let _ = writeln!(stream, "{d}");
                    if let Some(h) = &d.help {
                        let _ = writeln!(stream, "  help: {h}");
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
        BuildEvent::Diagnostic(_) => {}
    }
}
