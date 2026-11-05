//! Splitting lanes at branch boundaries.
//!
//! A displayed commit is a boundary when an unmerged child points to it and it is
//! 1. on trunk, 2. has a local branch, or 3. has two or more displayed children.
//! At a boundary no child sits directly above it; every child is a side lane.
//! This is done by reserving the boundary's column with renderdag's `reserve`
//! before the top of each lane is drawn, nearest to trunk first.

use std::collections::{HashMap, HashSet};

use crate::CommitId;
use crate::edges::Edge;
use crate::select::Selection;

/// Displayed commits that are boundaries.
pub fn boundaries(
    selection: &Selection,
    edges: &HashMap<CommitId, Vec<Edge>>,
    branch_heads: &HashSet<CommitId>,
) -> HashSet<CommitId> {
    todo!()
}

/// For each row (in render order), the boundaries to `reserve` before drawing it.
/// Rows that need no reservation are absent.
pub fn reservations(
    order: &[CommitId],
    edges: &HashMap<CommitId, Vec<Edge>>,
    selection: &Selection,
    boundaries: &HashSet<CommitId>,
) -> HashMap<CommitId, Vec<CommitId>> {
    todo!()
}
