//! Project folder discovery, book identification and asset resolution.
//!
//! Discovery is P1.3: a recursive scan that identifies each book from its
//! `\id` marker rather than its filename, refuses to guess between duplicates,
//! and never descends into a directory a previous build wrote.
//!
//! **Nothing in this crate opens a `.usfm` file for writing.** BLD-004 asks
//! that a build not overwrite its source, and an architecture with no write
//! path is a stronger answer than a rule someone has to remember.

pub mod discovery;

pub use discovery::{discover, identify, DiscoveredBook, Discovery};
