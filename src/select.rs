//! Choosing which commits to display.
//!
//! ```text
//! unmerged    = ancestors(anchors) − ancestors(trunk)
//! fork points = parents(unmerged) ∩ ancestors(trunk)
//! displayed   = unmerged ∪ fork points ∪ trunk tip ∪ anchors on trunk   (Default)
//!             + all ancestors of trunk                                  (All)
//! ```

use std::collections::HashSet;

use crate::CommitId;
use crate::dag::Dag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Only what relates to trunk and our own branches.
    Default,
    /// Also every ancestor of trunk (`--all`).
    All,
}

/// Starting points of the selection.
#[derive(Clone, Debug)]
pub struct Anchors {
    pub trunk: CommitId,
    /// Local branches (`refs/heads/*`), trunk included or not.
    pub branches: Vec<CommitId>,
    pub head: Option<CommitId>,
    /// HEADs of linked worktrees.
    pub worktree_heads: Vec<CommitId>,
}

/// The displayed set, split by kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    /// Displayed commits reachable from trunk (drawn as `◆`).
    pub trunk: HashSet<CommitId>,
    /// Displayed commits reachable from an anchor but not from trunk (drawn as `○`).
    pub unmerged: HashSet<CommitId>,
    /// Trunk commits included because an unmerged commit is cut from them.
    /// A subset of `trunk`; a reason for inclusion, not a separate kind.
    pub fork_points: HashSet<CommitId>,
}

impl Selection {
    pub fn contains(&self, id: &CommitId) -> bool {
        todo!()
    }

    pub fn len(&self) -> usize {
        todo!()
    }

    pub fn is_empty(&self) -> bool {
        todo!()
    }
}

pub fn select(dag: &Dag, anchors: &Anchors, mode: Mode) -> Selection {
    todo!()
}
