//! Edges between displayed commits.

use std::collections::HashMap;

use crate::CommitId;
use crate::dag::Dag;
use crate::select::Selection;

/// An edge from a displayed commit toward its parent.
/// Variants correspond one-to-one to sapling-renderdag's `Ancestor`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Edge {
    /// The parent itself is displayed.
    Parent(CommitId),
    /// The parent is hidden; this is the nearest displayed ancestor.
    Ancestor(CommitId),
    /// No displayed ancestor on this path (drawn as `~`).
    Anonymous,
}

impl Edge {
    pub fn target(&self) -> Option<&CommitId> {
        todo!()
    }
}

/// Edges for every displayed commit, in parent order (first parent first).
pub fn build_edges(dag: &Dag, selection: &Selection) -> HashMap<CommitId, Vec<Edge>> {
    todo!()
}
