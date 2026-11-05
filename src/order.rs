//! Row order: newest first, children before parents, stacks kept contiguous.

use std::cmp::Reverse;
use std::collections::HashMap;

use crate::CommitId;
use crate::dag::Dag;
use crate::edges::Edge;

/// Displayed commits in render order (top to bottom).
///
/// Ties between stacks are broken by commit time (newer first), then by id.
pub fn row_order(dag: &Dag, edges: &HashMap<CommitId, Vec<Edge>>) -> Vec<CommitId> {
    // Displayed children not yet emitted, per commit. A commit is ready at zero.
    let mut pending: HashMap<&CommitId, usize> = edges.keys().map(|id| (id, 0)).collect();
    for target in edges.values().flatten().filter_map(Edge::target) {
        *pending.entry(target).or_default() += 1;
    }

    let mut ready: Vec<&CommitId> = pending
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(id, _)| *id)
        .collect();
    let mut order = Vec::with_capacity(edges.len());
    // First parent of the last emitted row: continuing with it keeps a stack contiguous.
    let mut continue_with: Option<&CommitId> = None;

    while !ready.is_empty() {
        let index = continue_with
            .and_then(|c| ready.iter().position(|r| *r == c))
            .unwrap_or_else(|| newest(dag, &ready));
        let next = ready.swap_remove(index);
        order.push(next.clone());
        continue_with = edges[next].first().and_then(Edge::target);
        for parent in edges[next].iter().filter_map(Edge::target) {
            let n = pending.get_mut(parent).expect("edge target is displayed");
            *n -= 1;
            if *n == 0 {
                ready.push(parent);
            }
        }
    }
    order
}

/// Index of the newest commit in `candidates` (ties: smaller id first).
fn newest(dag: &Dag, candidates: &[&CommitId]) -> usize {
    let key = |id: &CommitId| {
        (
            Reverse(dag.meta(id).map_or(i64::MIN, |m| m.time)),
            id.clone(),
        )
    };
    (0..candidates.len())
        .min_by_key(|&i| key(candidates[i]))
        .expect("candidates is not empty")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::CommitMeta;
    use crate::dag::testing::{dag, id};
    use crate::edges::build_edges;
    use crate::lanes::testing::select_default;

    fn position(order: &[CommitId], name: &str) -> usize {
        order
            .iter()
            .position(|c| *c == id(name))
            .unwrap_or_else(|| panic!("{name} not in order"))
    }

    #[test]
    fn children_come_before_parents() {
        // Given: trunk t0 ← t1 ← t2、stack-a（a1 ← a2）は t1 を取り込んだマージ a3 を持ち、stack-b（b1）は t0 から切った
        let dag = dag(&[
            ("t0", &[]),
            ("a1", &["t0"]),
            ("t1", &["t0"]),
            ("b1", &["t0"]),
            ("a2", &["a1"]),
            ("t2", &["t1"]),
            ("a3", &["a2", "t1"]),
        ]);
        let selection = select_default(&dag, "t2", &["t2", "a3", "b1"]);
        let edges = build_edges(&dag, &selection);

        // When: 行の並び順を求める
        let order = row_order(&dag, &edges);

        // Then: 表示対象がすべて1回ずつ並び、どの辺でも子が親より上にある
        assert_eq!(order.len(), selection.len());
        for (child, child_edges) in &edges {
            for parent in child_edges.iter().filter_map(Edge::target) {
                assert!(
                    position(&order, &child.0) < position(&order, &parent.0),
                    "{child} must come before {parent}: {order:?}"
                );
            }
        }
    }

    #[test]
    fn each_stack_stays_contiguous_even_if_commit_times_interleave() {
        // Given: t0 から切った stack-a と stack-b を交互に作業した（a1 → b1 → a2 → b2 の順に作成）
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("b1", &["t0"]),
            ("a2", &["a1"]),
            ("b2", &["b1"]),
        ]);
        let selection = select_default(&dag, "t1", &["t1", "a2", "b2"]);
        let edges = build_edges(&dag, &selection);

        // When: 行の並び順を求める
        let order = row_order(&dag, &edges);

        // Then: 各 stack のコミットが隣り合い、ほかの stack と混ざらない
        assert_eq!(
            position(&order, "b1"),
            position(&order, "b2") + 1,
            "{order:?}"
        );
        assert_eq!(
            position(&order, "a1"),
            position(&order, "a2") + 1,
            "{order:?}"
        );
    }

    #[test]
    fn stacks_are_ordered_by_newest_commit_then_id() {
        let cases: &[(&str, i64, i64, &[&str])] = &[
            // Given: stack-b の先頭（時刻 20）が stack-a の先頭（時刻 10）より新しい / When: 並べる / Then: stack-b が上
            ("newer stack first", 10, 20, &["b1", "a1"]),
            // Given: 両方の先頭が同じ時刻 / When: 並べる / Then: ID の小さい stack-a が上
            ("same time, smaller id first", 10, 10, &["a1", "b1"]),
        ];

        for (name, a_time, b_time, expected) in cases {
            let mut dag = Dag::new();
            let meta = |time| CommitMeta {
                summary: String::new(),
                time,
            };
            dag.insert(id("t0"), vec![], meta(0));
            dag.insert(id("t1"), vec![id("t0")], meta(1));
            dag.insert(id("a1"), vec![id("t0")], meta(*a_time));
            dag.insert(id("b1"), vec![id("t0")], meta(*b_time));
            let selection = select_default(&dag, "t1", &["t1", "a1", "b1"]);
            let edges = build_edges(&dag, &selection);

            let order = row_order(&dag, &edges);

            let stacks: Vec<&str> = order
                .iter()
                .map(|c| c.0.as_str())
                .filter(|c| c.ends_with('1') && !c.starts_with('t'))
                .collect();
            assert_eq!(stacks, *expected, "case: {name}");
        }
    }
}
