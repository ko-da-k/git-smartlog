//! git-smartlog: a jj-like commit graph for plain git repositories.
//!
//! Pipeline (see `docs/adr/0001-jj-like-smartlog-on-plain-git.md`):
//!
//! ```text
//! repo (gix) → dag → select → edges → lanes → order → render
//! ```
//!
//! Everything except `repo` is a pure function over [`dag::Dag`], so it can be
//! tested with small fake DAGs whose commit ids are names like `"a1"`.

pub mod config;
pub mod dag;
pub mod edges;
pub mod lanes;
pub mod order;
pub mod render;
pub mod repo;
pub mod select;

use std::fmt;

/// A commit id. Hex object id for real repositories; any name in tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommitId(pub String);

impl CommitId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Abbreviated form used in the graph (first 7 characters).
    pub fn short(&self) -> &str {
        let end = self.0.char_indices().nth(7).map_or(self.0.len(), |(i, _)| i);
        &self.0[..end]
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
