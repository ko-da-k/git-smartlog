//! Drawing the graph with sapling-renderdag.

use std::collections::HashMap;

use crate::CommitId;
use crate::dag::Dag;
use crate::edges::Edge;

/// What kind of node a row is; decides the glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// `@`
    Head,
    /// `○`
    Unmerged,
    /// `◆`
    Trunk,
}

/// A name shown next to a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    /// Local branch; `current` is true when HEAD is on it (shown with `ᐅ`).
    Branch { name: String, current: bool },
    /// Linked worktree's HEAD, shown as `<name>@`.
    Worktree(String),
    Tag(String),
    Remote(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Always,
    Never,
}

/// Everything needed to draw, already computed by the earlier stages.
pub struct RenderInput<'a> {
    pub dag: &'a Dag,
    pub order: &'a [CommitId],
    pub edges: &'a HashMap<CommitId, Vec<Edge>>,
    pub reservations: &'a HashMap<CommitId, Vec<CommitId>>,
    pub kinds: &'a HashMap<CommitId, NodeKind>,
    pub labels: &'a HashMap<CommitId, Vec<Label>>,
    /// "Now" for relative dates, in seconds since the Unix epoch (fixed in tests).
    pub now: i64,
}

pub fn render(input: &RenderInput<'_>, color: Color) -> String {
    todo!()
}
