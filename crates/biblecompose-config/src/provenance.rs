//! Where a resolved value came from ([ADR-005]).
//!
//! Provenance is a property of the resolved types, not a feature that consumes
//! them. Adding it later means changing every field, every merge site, every
//! reader and every test; adding it at the start costs one wrapper. So it is
//! here from the first resolved value rather than from P2.6, which is left to
//! surface it.
//!
//! Four requirements read this rather than building their own answer:
//! STY-008's inspector is a read of it; CFG-007's reset-to-default is
//! "remove the key whose origin is a file"; STY-004 and CFG-004 use it as the
//! diagnostic's location.
//!
//! [ADR-005]: ../../../docs/adr/005-provenance.md

use std::fmt;
use std::ops::Deref;

use biblecompose_diagnostics::SourceLoc;

use crate::document::Located;

/// Where a value came from.
///
/// ADR-005 spells the file case as `File { path, line, col }`. It is a
/// [`SourceLoc`] here instead, for two reasons: the diagnostics panel already
/// has one location type and a second would have to be converted at every
/// reporting site, and `SourceLoc`'s optional line covers "in the file, but
/// the position is not known" — which the ADR's bare `u32` could only express
/// by fabricating a zero, the exact thing the ADR says not to do.
///
/// `Inherited { from: StyleSelector }` arrives with the style layer at M3. It
/// is a distinct case and not a flattening into `Builtin`: when `q2` takes its
/// indent from `q1`, "why does this look like this" is answered by the
/// inheritance and not by any file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// The embedded defaults. Also the answer where the origin is genuinely
    /// unknown — never a fabricated file location.
    Builtin,
    File(SourceLoc),
}

impl Origin {
    pub const fn is_builtin(&self) -> bool {
        matches!(self, Origin::Builtin)
    }

    /// The location to point a diagnostic at, if there is one.
    pub const fn location(&self) -> Option<&SourceLoc> {
        match self {
            Origin::Builtin => None,
            Origin::File(loc) => Some(loc),
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Origin::Builtin => f.write_str("built-in default"),
            Origin::File(loc) => write!(f, "{loc}"),
        }
    }
}

/// A resolved value and where it came from.
///
/// The fields are private and the only constructors state an origin, so
/// ADR-005's "a merge that fails to set an origin does not compile" is true of
/// this type rather than aspired to.
#[derive(Debug, Clone, PartialEq)]
pub struct Sourced<T> {
    value: T,
    origin: Origin,
}

impl<T> Sourced<T> {
    pub fn builtin(value: T) -> Self {
        Sourced {
            value,
            origin: Origin::Builtin,
        }
    }

    /// From a value read out of a project file, which knows where it was.
    pub fn from_file(located: Located<T>) -> Self {
        Sourced {
            value: located.value,
            origin: Origin::File(located.loc),
        }
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }

    /// The plain value.
    ///
    /// The emitter takes these and never a `Sourced`, so origin information
    /// cannot reach the output — a file path in a golden file is a golden file
    /// that fails on the next machine (SILE-005).
    pub fn into_value(self) -> T {
        self.value
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Sourced<U> {
        Sourced {
            value: f(self.value),
            origin: self.origin,
        }
    }

    /// Whether this value was written by the publisher — which is what
    /// CFG-007's "reset to default" needs to know.
    pub fn is_overridden(&self) -> bool {
        !self.origin.is_builtin()
    }
}

/// Reading a setting without caring where it came from is the common case, and
/// ADR-005 promises it stays readable.
impl<T> Deref for Sourced<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Display> fmt::Display for Sourced<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.value.fmt(f)
    }
}
