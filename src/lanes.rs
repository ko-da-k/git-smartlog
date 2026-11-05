//! Splitting lanes at branch boundaries.
//!
//! A displayed commit is a boundary when an unmerged child points to it and it
//!
//! 1. is on trunk,
//! 2. has a local branch, or
//! 3. has two or more displayed children.
//!
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
    // Number of displayed children pointing to each commit.
    let mut child_count: HashMap<&CommitId, usize> = HashMap::new();
    for target in edges.values().flatten().filter_map(Edge::target) {
        *child_count.entry(target).or_default() += 1;
    }

    // Parents that an unmerged child points to, filtered by the three rules.
    selection
        .unmerged
        .iter()
        .flat_map(|child| &edges[child])
        .filter_map(Edge::target)
        .filter(|parent| {
            selection.trunk.contains(*parent)
                || branch_heads.contains(*parent)
                || child_count.get(parent).is_some_and(|&n| n >= 2)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
pub(crate) mod testing {
    use std::collections::HashSet;

    use crate::CommitId;
    use crate::dag::Dag;
    use crate::dag::testing::id;
    use crate::select::{Anchors, Mode, Selection, select};

    /// Selection for `dag` with trunk `trunk`, local branches `branches` and HEAD on trunk.
    pub fn select_default(dag: &Dag, trunk: &str, branches: &[&str]) -> Selection {
        let anchors = Anchors {
            trunk: id(trunk),
            branches: branches.iter().map(|b| id(b)).collect(),
            head: Some(id(trunk)),
            worktree_heads: vec![],
        };
        select(dag, &anchors, Mode::Default)
    }

    pub fn heads(branches: &[&str]) -> HashSet<CommitId> {
        branches.iter().map(|b| id(b)).collect()
    }
}

/// For each row (in render order), the boundaries to `reserve` before drawing it.
/// Rows that need no reservation are absent.
pub fn reservations(
    order: &[CommitId],
    edges: &HashMap<CommitId, Vec<Edge>>,
    selection: &Selection,
    boundaries: &HashSet<CommitId>,
) -> HashMap<CommitId, Vec<CommitId>> {
    // Commits that already hold a column: edge targets of emitted rows, or reserved.
    let mut placed: HashSet<&CommitId> = HashSet::new();
    let mut result = HashMap::new();

    for row in order {
        let is_lane_top = selection.unmerged.contains(row) && !placed.contains(row);
        if is_lane_top {
            let to_reserve: Vec<CommitId> = boundaries_below(row, edges, selection, boundaries)
                .into_iter()
                .rev() // nearest to trunk first
                .filter(|b| placed.insert(*b))
                .cloned()
                .collect();
            if !to_reserve.is_empty() {
                result.insert(row.clone(), to_reserve);
            }
        }
        placed.insert(row);
        placed.extend(edges[row].iter().filter_map(Edge::target));
    }
    result
}

/// Boundaries on the first-parent path from `top` down to (and including) the
/// first trunk commit, top first.
fn boundaries_below<'a>(
    top: &'a CommitId,
    edges: &'a HashMap<CommitId, Vec<Edge>>,
    selection: &Selection,
    boundaries: &HashSet<CommitId>,
) -> Vec<&'a CommitId> {
    let mut found = Vec::new();
    let mut current = top;
    while let Some(parent) = edges[current].first().and_then(Edge::target) {
        if boundaries.contains(parent) {
            found.push(parent);
        }
        if !selection.unmerged.contains(parent) {
            break;
        }
        current = parent;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::testing::{heads, select_default};
    use super::*;
    use crate::dag::testing::id;
    use crate::dag::testing::{dag, ids};
    use crate::edges::build_edges;
    use crate::order::row_order;
    use crate::select::{Anchors, Mode, select};

    #[test]
    fn trunk_commit_with_unmerged_child_is_a_boundary() {
        // Given: trunk t0 ← t1、t0 から切った stack-a（a1）
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("a1", &["t0"])]);
        let selection = select_default(&dag, "t1", &["t1", "a1"]);
        let edges = build_edges(&dag, &selection);

        // When: 境目を求める
        let result = boundaries(&selection, &edges, &heads(&["t1", "a1"]));

        // Then: 未マージの子 a1 を持つ trunk 側の t0 が境目になる
        assert_eq!(result, ids(&["t0"]));
    }

    #[test]
    fn branch_head_with_unmerged_child_is_a_boundary() {
        // Given: stack-a（a1 ← a2）の上に stack-b（b1）を積んだ
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
            ("b1", &["a2"]),
        ]);
        let selection = select_default(&dag, "t1", &["t1", "a2", "b1"]);
        let edges = build_edges(&dag, &selection);

        // When: 境目を求める
        let result = boundaries(&selection, &edges, &heads(&["t1", "a2", "b1"]));

        // Then: trunk 側の t0 に加え、branch stack-a の付いた a2 が境目になる。branch のない a1 はならない
        assert_eq!(result, ids(&["t0", "a2"]));
    }

    #[test]
    fn unnamed_commit_with_two_displayed_children_is_a_boundary() {
        // Given: stack-b は b3 に進み、名前のない b2 から stack-c（c2）が切られている
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("b1", &["t0"]),
            ("b2", &["b1"]),
            ("b3", &["b2"]),
            ("c2", &["b2"]),
        ]);
        let selection = select_default(&dag, "t1", &["t1", "b3", "c2"]);
        let edges = build_edges(&dag, &selection);

        // When: 境目を求める
        let result = boundaries(&selection, &edges, &heads(&["t1", "b3", "c2"]));

        // Then: 子が2つ（b3, c2）ある b2 が境目になる。子が1つの b1 はならない
        assert_eq!(result, ids(&["t0", "b2"]));
    }

    #[test]
    fn commit_with_only_trunk_children_is_not_a_boundary() {
        // Given: --all で trunk 全体を表示。t2 は t1 と PR の p1 のマージで、t0 には trunk 側の子が2つ（t1, p1）ある
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("p1", &["t0"]),
            ("t2", &["t1", "p1"]),
        ]);
        let anchors = Anchors {
            trunk: id("t2"),
            branches: vec![id("t2")],
            head: Some(id("t2")),
            worktree_heads: vec![],
        };
        let selection = select(&dag, &anchors, Mode::All);
        let edges = build_edges(&dag, &selection);

        // When: 境目を求める
        let result = boundaries(&selection, &edges, &heads(&["t2"]));

        // Then: 未マージの子がいないので、子が2つある t0 も含めて境目はない
        assert!(result.is_empty());
    }

    #[test]
    fn lane_top_reserves_boundaries_below_nearest_trunk_first() {
        // Given: t0 から切った stack-a（a1 ← a2）の上に stack-b（b1）を積んだ
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
            ("b1", &["a2"]),
        ]);
        let selection = select_default(&dag, "t1", &["t1", "a2", "b1"]);
        let edges = build_edges(&dag, &selection);
        let order = row_order(&dag, &edges);
        let bounds = boundaries(&selection, &edges, &heads(&["t1", "a2", "b1"]));

        // When: 予約を求める
        let result = reservations(&order, &edges, &selection, &bounds);

        // Then: 枝の先頭 b1 の行で、trunk に近い t0、次に stack-a の a2 の順に予約する
        assert_eq!(
            result,
            HashMap::from([(id("b1"), vec![id("t0"), id("a2")])])
        );
    }

    #[test]
    fn reserved_boundary_is_not_reserved_again() {
        // Given: t0 から stack-a（a1）と stack-b（b1）を切った。b1 の方が新しい
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("b1", &["t0"]),
        ]);
        let selection = select_default(&dag, "t1", &["t1", "a1", "b1"]);
        let edges = build_edges(&dag, &selection);
        let order = row_order(&dag, &edges);
        let bounds = boundaries(&selection, &edges, &heads(&["t1", "a1", "b1"]));

        // When: 予約を求める
        let result = reservations(&order, &edges, &selection, &bounds);

        // Then: 先に描く b1 の行で t0 を予約し、後の a1 の行では予約しない
        assert_eq!(order[0], id("b1"), "{order:?}");
        assert_eq!(result, HashMap::from([(id("b1"), vec![id("t0")])]));
    }
}
