//! Edges between displayed commits.

use std::collections::{HashMap, HashSet, VecDeque};

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
        match self {
            Edge::Parent(id) | Edge::Ancestor(id) => Some(id),
            Edge::Anonymous => None,
        }
    }
}

/// Edges for every displayed commit, in parent order (first parent first).
pub fn build_edges(dag: &Dag, selection: &Selection) -> HashMap<CommitId, Vec<Edge>> {
    selection
        .trunk
        .iter()
        .chain(&selection.unmerged)
        .map(|id| {
            // Unmerged merges keep only the first-parent edge; other parents are
            // shown as references (see `merge_refs`) so no line crosses other lanes.
            let parents = dag.parents(id);
            let parents = if selection.unmerged.contains(id) {
                &parents[..parents.len().min(1)]
            } else {
                parents
            };
            let mut edges = Vec::new();
            for parent in parents {
                if selection.contains(parent) {
                    push_unique(&mut edges, Edge::Parent(parent.clone()));
                } else {
                    let ancestors = nearest_displayed(dag, selection, parent);
                    if ancestors.is_empty() {
                        push_unique(&mut edges, Edge::Anonymous);
                    }
                    for ancestor in ancestors {
                        push_unique(&mut edges, Edge::Ancestor(ancestor));
                    }
                }
            }
            (id.clone(), edges)
        })
        .collect()
}

/// Non-first parents of unmerged merge commits, shown as references under the row
/// instead of edges. Trunk merges have none. Commits without references are absent.
pub fn merge_refs(dag: &Dag, selection: &Selection) -> HashMap<CommitId, Vec<CommitId>> {
    selection
        .unmerged
        .iter()
        .filter_map(|id| match dag.parents(id) {
            [_, rest @ ..] if !rest.is_empty() => Some((id.clone(), rest.to_vec())),
            _ => None,
        })
        .collect()
}

/// Nearest displayed ancestors of the hidden commit `start`, in breadth-first
/// parent order. Walking stops at each displayed commit.
fn nearest_displayed(dag: &Dag, selection: &Selection, start: &CommitId) -> Vec<CommitId> {
    let mut found = Vec::new();
    let mut seen = HashSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(id) = queue.pop_front() {
        for parent in dag.parents(id) {
            if !seen.insert(parent) {
                continue;
            }
            if selection.contains(parent) {
                found.push(parent.clone());
            } else {
                queue.push_back(parent);
            }
        }
    }
    found
}

fn push_unique(edges: &mut Vec<Edge>, edge: Edge) {
    if !edges.contains(&edge) {
        edges.push(edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::testing::{dag, id, ids};

    /// Selection with `trunk` and `unmerged` displayed (no fork points needed for edges).
    fn selection(trunk: &[&str], unmerged: &[&str]) -> Selection {
        Selection {
            trunk: ids(trunk),
            unmerged: ids(unmerged),
            fork_points: HashSet::new(),
        }
    }

    #[test]
    fn displayed_parent_is_a_parent_edge() {
        // Given: t0 ← a1 で、どちらも表示対象
        let dag = dag(&[("t0", &[]), ("a1", &["t0"])]);
        let selection = selection(&["t0"], &["a1"]);

        // When: 辺を作る
        let edges = build_edges(&dag, &selection);

        // Then: a1 から t0 への Parent の辺になる
        assert_eq!(edges[&id("a1")], vec![Edge::Parent(id("t0"))]);
    }

    #[test]
    fn hidden_parent_becomes_ancestor_edge_to_nearest_displayed() {
        // Given: trunk t0 ← t1 ← t2 ← t3 で、表示対象は t0 と t3 だけ（t1, t2 は省略）
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("t2", &["t1"]), ("t3", &["t2"])]);
        let selection = selection(&["t0", "t3"], &[]);

        // When: 辺を作る
        let edges = build_edges(&dag, &selection);

        // Then: t3 から、最も近い表示対象の祖先 t0 への Ancestor の辺になる
        assert_eq!(edges[&id("t3")], vec![Edge::Ancestor(id("t0"))]);
    }

    #[test]
    fn no_displayed_ancestor_is_an_anonymous_edge() {
        // Given: trunk t0 ← t1 ← t2 で、表示対象は t1（分岐点）と t2 だけ。t0 は省略
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("t2", &["t1"])]);
        let selection = selection(&["t1", "t2"], &[]);

        // When: 辺を作る
        let edges = build_edges(&dag, &selection);

        // Then: t1 の先に表示対象がないので Anonymous（~）の辺になる。親のない t0 相当のルートは辺なし
        assert_eq!(edges[&id("t1")], vec![Edge::Anonymous]);
        assert_eq!(edges[&id("t2")], vec![Edge::Parent(id("t1"))]);
    }

    #[test]
    fn root_commit_has_no_edges() {
        // Given: 親のないルートコミット t0 だけが表示対象
        let dag = dag(&[("t0", &[])]);
        let selection = selection(&["t0"], &[]);

        // When: 辺を作る
        let edges = build_edges(&dag, &selection);

        // Then: 辺はない（~ も付かない）
        assert!(edges[&id("t0")].is_empty());
    }

    #[test]
    fn trunk_merge_with_both_parents_displayed_has_two_edges_first_parent_first() {
        // Given: trunk の t2 は t1（第一親）と PR の p1（第二親）のマージで、全員が表示対象（--all 相当）
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("p1", &["t0"]), ("t2", &["t1", "p1"])]);
        let selection = selection(&["t0", "t1", "p1", "t2"], &[]);

        // When: 辺を作る
        let edges = build_edges(&dag, &selection);

        // Then: trunk 側のマージは、第一親 t1、第二親 p1 の順で Parent の辺が2本になる
        assert_eq!(
            edges[&id("t2")],
            vec![Edge::Parent(id("t1")), Edge::Parent(id("p1"))]
        );
    }

    #[test]
    fn unmerged_merge_has_only_first_parent_edge() {
        // Given: 未マージの a3 は a2（第一親）と trunk の t1（第二親）のマージ。全員が表示対象
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
            ("a3", &["a2", "t1"]),
        ]);
        let selection = selection(&["t0", "t1"], &["a1", "a2", "a3"]);

        // When: 辺を作る
        let edges = build_edges(&dag, &selection);

        // Then: 第一親 a2 への辺だけになり、t1 への線は引かない（参照の行で示す）
        assert_eq!(edges[&id("a3")], vec![Edge::Parent(id("a2"))]);
    }

    #[test]
    fn merge_refs_lists_non_first_parents_of_unmerged_merges_only() {
        // Given: 未マージの a3 は a2 と t1 のマージ。trunk の t2 は t1 と PR の p1 のマージ（p1 は非表示）
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("p1", &["t0"]),
            ("t2", &["t1", "p1"]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
            ("a3", &["a2", "t1"]),
        ]);
        let selection = selection(&["t0", "t1", "t2"], &["a1", "a2", "a3"]);

        // When: 参照を求める
        let refs = merge_refs(&dag, &selection);

        // Then: 未マージのマージ a3 だけが、第二親 t1 を参照として持つ。trunk のマージ t2 は持たない
        assert_eq!(refs, HashMap::from([(id("a3"), vec![id("t1")])]));
    }

    #[test]
    fn edge_target() {
        let cases = [
            // Given: Parent の辺 / When: 行き先を求める / Then: その親
            (Edge::Parent(id("t0")), Some(id("t0"))),
            // Given: Ancestor の辺 / When: 行き先を求める / Then: その祖先
            (Edge::Ancestor(id("t0")), Some(id("t0"))),
            // Given: Anonymous の辺 / When: 行き先を求める / Then: 行き先はない
            (Edge::Anonymous, None),
        ];

        for (edge, expected) in cases {
            assert_eq!(edge.target(), expected.as_ref(), "edge: {edge:?}");
        }
    }
}
