//! The build state machine and its event stream.
//!
//! These are exactly GUI-006's states. They are the single source of truth for
//! what the UI shows and what the Build button does, and every transition is
//! **reported as an event rather than polled** — which is what lets a CLI and
//! a window observe the same build without either one owning it.

use std::sync::mpsc;

use biblecompose_diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};

/// ```text
/// Idle → Loading → Loaded ──blocking errors?──→ Blocked
///                    │ no
///                    ▼
///               Validating → Emitting → Typesetting → Publishing → Succeeded
///                    └──────────┴───────────┴────────────┴────────→ Failed
///                                                        └────────→ Cancelled
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildState {
    Idle,
    Loading,
    Loaded,
    /// Validation found blocking errors, so no backend process starts
    /// (DIA-002).
    Blocked,
    Validating,
    /// "generating" in GUI-006's wording.
    Emitting,
    /// "running SILE".
    Typesetting,
    Publishing,
    Succeeded,
    Failed,
    Cancelled,
}

impl BuildState {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            BuildState::Succeeded
                | BuildState::Failed
                | BuildState::Cancelled
                | BuildState::Blocked
        )
    }

    /// The label GUI-006 uses, for a status line.
    pub const fn label(self) -> &'static str {
        match self {
            BuildState::Idle => "idle",
            BuildState::Loading => "loading",
            BuildState::Loaded => "loaded",
            BuildState::Blocked => "blocked",
            BuildState::Validating => "validating",
            BuildState::Emitting => "generating",
            BuildState::Typesetting => "running SILE",
            BuildState::Publishing => "publishing",
            BuildState::Succeeded => "completed",
            BuildState::Failed => "failed",
            BuildState::Cancelled => "canceled",
        }
    }

    /// Whether `next` may follow `self`.
    ///
    /// Enforced rather than documented, so an orchestration bug shows up as a
    /// failed transition instead of a UI that says "completed" after a
    /// failure.
    pub fn can_advance_to(self, next: BuildState) -> bool {
        use BuildState::*;
        match self {
            Idle => matches!(next, Loading),
            Loading => matches!(next, Loaded | Failed | Cancelled),
            Loaded => matches!(next, Validating | Blocked | Failed | Cancelled),
            Validating => matches!(next, Emitting | Blocked | Failed | Cancelled),
            // Every build typesets: there is no cached result that lets a
            // build succeed straight from `Emitting`.
            Emitting => matches!(next, Typesetting | Failed | Cancelled),
            Typesetting => matches!(next, Publishing | Failed | Cancelled),
            Publishing => matches!(next, Succeeded | Failed | Cancelled),
            // Terminal states rest until a new build resets the machine.
            Succeeded | Failed | Cancelled | Blocked => matches!(next, Idle),
        }
    }
}

impl std::fmt::Display for BuildState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Everything a build tells the outside world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildEvent {
    State(BuildState),
    Diagnostic(Diagnostic),
    /// A line of backend output, already tagged with its stream.
    Log {
        stream: String,
        text: String,
    },
    /// The backend version, once known (SILE-002).
    Backend(String),
    /// How many pages have been set, and how many the last successful build of
    /// this project needed.
    ///
    /// The estimate is the previous run's page count because there is no other
    /// honest source: a typesetter does not know how long a document is until
    /// it has set it. `None` on the first build of a project, which is the one
    /// time a bar cannot be more than "something is happening".
    Pages {
        done: u32,
        expected: Option<u32>,
    },
    /// The published PDF.
    Output(camino::Utf8PathBuf),
    /// Where the backend's output is being written, announced before the run
    /// starts so it is known even if the run never ends.
    LogFile(camino::Utf8PathBuf),
}

/// Drives the state machine and publishes its events.
pub struct BuildReporter {
    state: BuildState,
    tx: mpsc::Sender<BuildEvent>,
}

impl BuildReporter {
    pub fn new() -> (Self, mpsc::Receiver<BuildEvent>) {
        let (tx, rx) = mpsc::channel();
        (
            BuildReporter {
                state: BuildState::Idle,
                tx,
            },
            rx,
        )
    }

    pub fn state(&self) -> BuildState {
        self.state
    }

    /// Advance, or panic — an illegal transition is a bug in the orchestrator
    /// and silently allowing it would make the event log a work of fiction.
    pub fn advance(&mut self, next: BuildState) {
        assert!(
            self.state.can_advance_to(next),
            "illegal build transition {} → {}",
            self.state,
            next
        );
        self.state = next;
        let _ = self.tx.send(BuildEvent::State(next));
    }

    pub fn diagnostic(&self, d: Diagnostic) {
        let _ = self.tx.send(BuildEvent::Diagnostic(d));
    }

    pub fn log(&self, stream: &str, text: String) {
        let _ = self.tx.send(BuildEvent::Log {
            stream: stream.to_owned(),
            text,
        });
    }

    pub fn backend(&self, version: String) {
        let _ = self.tx.send(BuildEvent::Backend(version));
    }

    pub fn log_file(&self, path: camino::Utf8PathBuf) {
        let _ = self.tx.send(BuildEvent::LogFile(path));
    }

    pub fn pages(&self, done: u32, expected: Option<u32>) {
        let _ = self.tx.send(BuildEvent::Pages { done, expected });
    }

    pub fn output(&self, path: camino::Utf8PathBuf) {
        let _ = self.tx.send(BuildEvent::Output(path));
    }
}

#[cfg(test)]
mod tests {
    use super::BuildState::*;
    use super::*;

    #[test]
    fn the_happy_path_is_the_one_gui_006_describes() {
        let (mut r, rx) = BuildReporter::new();
        for s in [
            Loading,
            Loaded,
            Validating,
            Emitting,
            Typesetting,
            Publishing,
            Succeeded,
        ] {
            r.advance(s);
        }
        drop(r);

        let states: Vec<BuildState> = rx
            .iter()
            .filter_map(|e| match e {
                BuildEvent::State(s) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(
            states,
            [
                Loading,
                Loaded,
                Validating,
                Emitting,
                Typesetting,
                Publishing,
                Succeeded
            ]
        );
    }

    #[test]
    fn the_blocked_path_never_reaches_the_backend() {
        let (mut r, rx) = BuildReporter::new();
        r.advance(Loading);
        r.advance(Loaded);
        r.advance(Validating);
        r.advance(Blocked);
        drop(r);

        let states: Vec<BuildState> = rx
            .iter()
            .filter_map(|e| match e {
                BuildEvent::State(s) => Some(s),
                _ => None,
            })
            .collect();
        assert!(
            !states.contains(&Typesetting),
            "DIA-002: no backend process starts when validation blocks"
        );
        assert_eq!(*states.last().unwrap(), Blocked);
    }

    #[test]
    fn cancellation_is_reachable_from_every_running_state() {
        for s in [
            Loading,
            Loaded,
            Validating,
            Emitting,
            Typesetting,
            Publishing,
        ] {
            assert!(s.can_advance_to(Cancelled), "{s} should be cancellable");
        }
    }

    #[test]
    fn terminal_states_only_reset_to_idle() {
        for s in [Succeeded, Failed, Cancelled, Blocked] {
            assert!(s.is_terminal());
            assert!(s.can_advance_to(Idle));
            assert!(!s.can_advance_to(Typesetting));
        }
    }

    #[test]
    fn typesetting_cannot_be_reached_without_emitting() {
        assert!(!Loaded.can_advance_to(Typesetting));
        assert!(!Validating.can_advance_to(Typesetting));
        assert!(Emitting.can_advance_to(Typesetting));
    }

    #[test]
    #[should_panic(expected = "illegal build transition")]
    fn an_illegal_transition_is_a_bug_not_a_silent_relabel() {
        let (mut r, _rx) = BuildReporter::new();
        r.advance(Loading);
        r.advance(Succeeded);
    }

    #[test]
    fn every_state_has_a_distinct_label() {
        let all = [
            Idle,
            Loading,
            Loaded,
            Blocked,
            Validating,
            Emitting,
            Typesetting,
            Publishing,
            Succeeded,
            Failed,
            Cancelled,
        ];
        let mut labels: Vec<&str> = all.iter().map(|s| s.label()).collect();
        labels.sort_unstable();
        let before = labels.len();
        labels.dedup();
        assert_eq!(before, labels.len());
    }
}
