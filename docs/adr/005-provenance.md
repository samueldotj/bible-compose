# ADR-005 — Resolved settings and styles carry where each value came from

**Status:** Proposed
**Relates to:** STY-008, STY-004, CFG-004, CFG-006, CFG-007

## Context

SRS §8.4 lists STY-008 as a SHOULD:

> The application should provide an inspector showing the resolved style and source of each property (default vs project override). — *"User can diagnose why an element looks a certain way."*

It sits among genuinely deferrable items, and reads like one. It is not, because of *when* it has to be decided rather than *how much* it is wanted.

Provenance is not a feature that consumes resolved configuration; it is a property of the resolved types. Adding it later means changing every field of `ResolvedSettings` and `ResolvedStyles`, every merge site, every reader, and every test. Adding it at the start costs one wrapper type and a merge function written once.

Three other requirements need the same information and are all MUSTs:

- **STY-004** — an unsupported style property produces a diagnostic. A diagnostic without a file and line is a diagnostic the user cannot act on.
- **CFG-004** — an unknown settings key is visible. `page.wdith` must be reported *where it was written*.
- **CFG-007** — a user can reset a setting to inherited behaviour. That requires knowing whether the current value came from the project file at all.

And **CFG-006** — save without destroying comments and formatting — needs the same document that supplies the spans.

## Options

### A — Values only, provenance later (rejected)

`ResolvedStyles` is a map of plain values. Fast to write, and it defers a decision that gets three times harder each milestone. By M3 the style GUI reads resolved values everywhere; by M4 diagnostics are wired to them. Retrofitting then touches most of two crates and every test, for a SHOULD, which is exactly when it gets dropped.

Dropping it means STY-004 and CFG-004 report *what* is wrong without *where*, and STY-008 never ships.

### B — A side table mapping key to origin (considered)

Values stay plain; a parallel map records where each came from. Cheap and reversible.

Rejected because a parallel structure keyed by a stringly-typed path is exactly the thing that drifts out of sync — nothing fails if a merge forgets to update it, and the failure surfaces as an inspector quietly showing "default" for an overridden value. It also reintroduces string keys into a design that made selectors typed on purpose ([ARCHITECTURE §6](../ARCHITECTURE.md#6-configuration-and-style-resolution)).

### C — Provenance on the value (chosen)

## Decision

**Resolution produces values that carry their origin. The type system makes it impossible to merge a value without saying where it came from.**

```rust
pub struct Sourced<T> { pub value: T, pub origin: Origin }

pub enum Origin {
    Builtin,
    File { path: Utf8PathBuf, line: u32, col: u32 },
    Inherited { from: StyleSelector },
}
```

Spans come from `toml_edit`, which is already the single parse of both files ([ARCHITECTURE §6](../ARCHITECTURE.md#6-configuration-and-style-resolution)) — so provenance costs a field copy during merge rather than a second pass over the source.

Four requirements then fall out rather than being built:

| Requirement | How it is met |
|---|---|
| STY-008 inspector | A read of the resolved map. No new machinery |
| CFG-007 reset to default | Remove the key whose origin is `File`; re-resolve |
| STY-004 / CFG-004 diagnostics | The origin is the diagnostic's location |
| GUI-010 unsaved-edit protection | Pending edits are a fourth origin held in the session, so the close prompt can name which settings would be lost |

**`Inherited` is a distinct origin, not a flattening into `Builtin`.** STY-007 allows a style to inherit from another; when `q2` gets its indent from `q1`, the inspector must say so, because "why does this look like this" is usually answered by the inheritance and not by the file.

## Consequences

**Every resolved field is wrapped**, which is visible at every read site. Mitigated by `Deref` to the inner value for the common case, so consumers that do not care about origin read `style.font_size` and get a length.

**The emitter must not see provenance.** It takes plain values, so that origin information cannot influence output and cannot vary between two runs that resolved the same values by different routes. This matters for determinism (SILE-005): a `Sourced<T>` reaching the emitter is a way for a file path to end up in a golden file. Enforced by the emitter's input type.

**Memory grows by a path and two integers per resolved value.** Irrelevant at this scale — hundreds of values, not millions.

**Where the origin is genuinely unknown, it is `Builtin`, never a fabricated file location.** A zero line number that renders as `styles.toml:0` in an inspector is worse than no location at all, because it reads as a bug in the file rather than a gap in the tooling.
