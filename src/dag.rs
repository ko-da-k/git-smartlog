//! In-memory commit DAG: commit id → parents, plus commit metadata.

use std::collections::{HashMap, HashSet};

use crate::CommitId;

/// Metadata shown on each row and used for ordering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitMeta {
    /// First line of the commit message.
    pub summary: String,
    /// Committer time in seconds since the Unix epoch.
    pub time: i64,
}

/// The part of the commit graph reachable from the anchors and trunk.
#[derive(Clone, Debug, Default)]
pub struct Dag {
    parents: HashMap<CommitId, Vec<CommitId>>,
    meta: HashMap<CommitId, CommitMeta>,
}

impl Dag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a commit. Parents are kept in order (first parent first).
    pub fn insert(&mut self, id: CommitId, parents: Vec<CommitId>, meta: CommitMeta) {
        todo!()
    }

    pub fn contains(&self, id: &CommitId) -> bool {
        todo!()
    }

    /// Parents in order. Empty for root commits and unknown ids.
    pub fn parents(&self, id: &CommitId) -> &[CommitId] {
        todo!()
    }

    pub fn meta(&self, id: &CommitId) -> Option<&CommitMeta> {
        todo!()
    }

    /// All ancestors of `heads`, including the heads themselves.
    pub fn ancestors<'a>(&self, heads: impl IntoIterator<Item = &'a CommitId>) -> HashSet<CommitId> {
        todo!()
    }
}
