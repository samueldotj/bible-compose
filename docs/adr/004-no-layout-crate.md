# ADR-004 — Keep the backend boundary, drop the backend-neutral layout model

**Status:** Proposed
**Relates to:** SRS §12.1, §12.2, SILE-001

## Context

SRS §12.1 proposes ten crates. Two of them exist to insulate the application from SILE:

```text
biblecompose-layout/   # backend-neutral layout model
biblecompose-sile/     # SILE emitter, runtime invocation, diagnostics
```

and §12.2 describes the layout layer as converting *"semantic content plus style/config into backend-neutral layout intentions where practical."*

The requirement behind it is SILE-001: *"BibleCompose shall invoke SILE only through a dedicated backend adapter interface"*, verified by *"no UI module shells out to SILE directly."*

Two distinct things are being asked for and they have very different costs. The first is a **boundary** — one place where the backend is reached. The second is an **intermediate model** — a full representation of layout intent, independent of any backend.

## Options

### A — Both, as specified (rejected)

Every construct gets modelled three times: in `ScriptureDocument`, in the layout model, and in the emitter. Every new feature — a heading variant, a note placement mode, a figure sizing rule — is threaded through all three. Every diagnostic that wants to name a source location carries provenance across two translations.

That price buys portability to a second backend. There is no second backend, none is planned, and SRS §2.3 and §17.2 are emphatic about scope. The SRS's own hedge — *"where practical"* — concedes that the model would be partial, and a partial neutrality layer is the worst of the three outcomes: full cost, and the backend still leaks through the gaps.

There is a further problem specific to typesetting. Backend-neutral layout means expressing intent that two engines could both honour, but the interesting decisions here — column balancing, note splitting across pages, verse-aware break penalties — are exactly where engines differ. A neutral model either omits them, in which case it is not carrying the layout, or encodes SILE's answers, in which case it is not neutral.

### B — Neither (rejected)

Emit SILE from wherever it is convenient. Violates SILE-001 outright and makes the backend unmockable, so nothing that touches emission can be tested without SILE installed.

### C — The boundary without the model (chosen)

## Decision

**A `Backend` trait is the only route to typesetting. There is no backend-neutral layout model. The SILE emitter consumes `ScriptureDocument`, `ResolvedSettings`, and `ResolvedStyles` directly.**

```rust
pub trait Backend {
    fn version(&self) -> Result<BackendVersion>;
    fn run(&self, job: &BackendJob, cancel: &CancelToken,
           log: &mut dyn FnMut(LogLine)) -> Result<BackendOutcome>;
}
```

SILE-001 is satisfied exactly as well as under option A: one trait, one implementation, one crate that knows SILE exists, and nothing above `biblecompose-app` able to reach it. The acceptance criterion — no UI module shells out to SILE — is a structural fact, not a review item.

**What replaces the neutrality claim** is that the three inputs to the emitter are already backend-neutral. `ScriptureDocument` knows nothing of pages. `ResolvedSettings` and `ResolvedStyles` are typed values with parsed units, not SILE strings — SRS §12.2's *"no raw SILE strings in project files"* is enforced at the configuration boundary, where it belongs, rather than by a layer downstream of it.

So the neutral boundary is `(ScriptureDocument, ResolvedSettings, ResolvedStyles)`. A second backend would be a second implementation of `Backend` reading the same three, which is what the layout crate was meant to enable and is a smaller change than adding a model that both must agree on.

**`biblecompose-core` also goes.** Its stated job, orchestration, has exactly one caller and one implementation; it lives in `biblecompose-app`. Seven crates plus a test kit ([ARCHITECTURE §3](../ARCHITECTURE.md#3-crates)).

## Consequences

**A second backend costs more, later.** Accepted, and the shape of the cost is known: a new emitter against the same three inputs. The alternative was paying most of that cost now, in every feature, for a backend that may never exist.

**Layout intent that has nowhere else to live goes in the Lua class** ([ADR-002](002-sile-interface.md)) — column frames, note placement, break penalties. That is the right home for it: it is SILE-specific by nature, and it can be iterated without recompiling.

**The emitter must be testable without SILE.** Golden XML tests run against the emitter alone; only the PDF tests need the binary. This is what keeps the per-push CI fast and is why the trait's `run` is separable from emission inside `biblecompose-sile`.

**A boundary this thin needs a rule to stay thin.** No type from `biblecompose-sile` appears in any other crate's public API, and `biblecompose-sile` is not a dependency of anything except `biblecompose-app`. Checked in CI by a dependency assertion, because this is the kind of rule that erodes one convenient import at a time.
