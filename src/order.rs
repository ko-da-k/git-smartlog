//! Row order: newest first, children before parents, stacks kept contiguous.

use std::collections::HashMap;

use crate::CommitId;
use crate::dag::Dag;
use crate::edges::Edge;

/// Displayed commits in render order (top to bottom).
///
/// Ties between stacks are broken by commit time (newer first), then by id.
pub fn row_order(dag: &Dag, edges: &HashMap<CommitId, Vec<Edge>>) -> Vec<CommitId> {
    todo!()
}
