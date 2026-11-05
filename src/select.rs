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
use crate::repo::Refs;

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
        self.trunk.contains(id) || self.unmerged.contains(id)
    }

    pub fn len(&self) -> usize {
        self.trunk.len() + self.unmerged.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Anchors {
    /// Anchors from repository refs: local branches, HEAD and linked worktree HEADs.
    /// Remote branches and tags are labels only and never become anchors.
    pub fn from_refs(refs: &Refs, trunk: CommitId) -> Self {
        Self {
            trunk,
            branches: refs.local_branches.iter().map(|(_, c)| c.clone()).collect(),
            head: refs.head.as_ref().and_then(|h| h.commit()).cloned(),
            worktree_heads: refs
                .worktrees
                .iter()
                .filter_map(|w| w.head.clone())
                .collect(),
        }
    }

    fn all(&self) -> impl Iterator<Item = &CommitId> {
        self.branches
            .iter()
            .chain(&self.head)
            .chain(&self.worktree_heads)
    }
}

pub fn select(dag: &Dag, anchors: &Anchors, mode: Mode) -> Selection {
    let trunk_ancestors = dag.ancestors([&anchors.trunk]);

    let unmerged: HashSet<CommitId> = dag
        .ancestors(anchors.all())
        .into_iter()
        .filter(|id| !trunk_ancestors.contains(id))
        .collect();

    let fork_points: HashSet<CommitId> = unmerged
        .iter()
        .flat_map(|id| dag.parents(id))
        .filter(|parent| trunk_ancestors.contains(*parent))
        .cloned()
        .collect();

    let trunk = match mode {
        Mode::All => trunk_ancestors,
        Mode::Default => {
            let anchors_on_trunk = anchors.all().filter(|id| trunk_ancestors.contains(*id));
            std::iter::once(&anchors.trunk)
                .chain(&fork_points)
                .chain(anchors_on_trunk)
                .cloned()
                .collect()
        }
    };

    Selection {
        trunk,
        unmerged,
        fork_points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::testing::{dag, id, ids};
    use crate::repo::{Head, Worktree};

    /// Anchors with trunk `trunk`, local branches `branches` and HEAD `head`.
    fn anchors(trunk: &str, branches: &[&str], head: Option<&str>) -> Anchors {
        Anchors {
            trunk: id(trunk),
            branches: branches.iter().map(|b| id(b)).collect(),
            head: head.map(id),
            worktree_heads: vec![],
        }
    }

    #[test]
    fn only_trunk_tip_when_trunk_is_the_only_anchor() {
        // Given: 直列の trunk t0 ← t1 ← t2。起点は trunk（main = t2）と、そこにいる HEAD だけ
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("t2", &["t1"])]);
        let anchors = anchors("t2", &["t2"], Some("t2"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: trunk の先端だけが表示され、未マージも分岐点もない
        assert_eq!(result.trunk, ids(&["t2"]));
        assert!(result.unmerged.is_empty());
        assert!(result.fork_points.is_empty());
    }

    #[test]
    fn one_branch_shows_unmerged_fork_point_and_trunk_tip() {
        // Given: trunk t0 ← t1、t0 から切った stack-a（a1 ← a2）。HEAD は a2
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
        ]);
        let anchors = anchors("t1", &["t1", "a2"], Some("a2"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: 未マージ a1, a2、分岐点 t0、trunk の先端 t1 が表示される
        assert_eq!(result.unmerged, ids(&["a1", "a2"]));
        assert_eq!(result.fork_points, ids(&["t0"]));
        assert_eq!(result.trunk, ids(&["t0", "t1"]));
    }

    #[test]
    fn each_branch_has_its_own_fork_point() {
        // Given: trunk t0 ← t1 ← t2。stack-a は t0 から、stack-b は t1 から切った
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("t2", &["t1"]),
            ("a1", &["t0"]),
            ("b1", &["t1"]),
        ]);
        let anchors = anchors("t2", &["t2", "a1", "b1"], Some("t2"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: それぞれの分岐点 t0, t1 が表示される
        assert_eq!(result.unmerged, ids(&["a1", "b1"]));
        assert_eq!(result.fork_points, ids(&["t0", "t1"]));
        assert_eq!(result.trunk, ids(&["t0", "t1", "t2"]));
    }

    #[test]
    fn stacked_branches_share_one_fork_point() {
        // Given: stack-a（a1 ← a2）の上に stack-b（b1）を積んだ
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
            ("b1", &["a2"]),
        ]);
        let anchors = anchors("t1", &["t1", "a2", "b1"], Some("b1"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: a と b の両方が未マージで、分岐点は t0 だけ（a2 は未マージなので分岐点にならない）
        assert_eq!(result.unmerged, ids(&["a1", "a2", "b1"]));
        assert_eq!(result.fork_points, ids(&["t0"]));
    }

    #[test]
    fn detached_head_on_unmerged_commit_is_shown() {
        // Given: branch の付いていないコミット x1 に HEAD がある（detached）
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("x1", &["t0"])]);
        let anchors = anchors("t1", &["t1"], Some("x1"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: x1 は未マージとして表示され、分岐点 t0 も表示される
        assert_eq!(result.unmerged, ids(&["x1"]));
        assert_eq!(result.fork_points, ids(&["t0"]));
    }

    #[test]
    fn head_on_old_trunk_commit_is_shown_as_trunk() {
        // Given: trunk t0 ← t1 ← t2 で、HEAD は古い t0 にある
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("t2", &["t1"])]);
        let anchors = anchors("t2", &["t2"], Some("t0"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: t0 は trunk 側の起点として表示され、間の t1 は表示されない
        assert_eq!(result.trunk, ids(&["t0", "t2"]));
        assert!(result.unmerged.is_empty());
    }

    #[test]
    fn merged_branch_is_shown_as_trunk_not_unmerged() {
        // Given: stack-a（a1）は trunk のマージコミット t1 に取り込み済み
        let dag = dag(&[("t0", &[]), ("a1", &["t0"]), ("t1", &["t0", "a1"])]);
        let anchors = anchors("t1", &["t1", "a1"], Some("t1"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: a1 は未マージにならず、trunk 側の起点として表示される
        assert!(result.unmerged.is_empty());
        assert_eq!(result.trunk, ids(&["a1", "t1"]));
    }

    #[test]
    fn merging_trunk_into_branch_keeps_both_fork_points() {
        // Given: stack-a は t0 から切り、その後 t1 を取り込んだ（a2 は a1 と t1 のマージ）
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("t2", &["t1"]),
            ("a1", &["t0"]),
            ("a2", &["a1", "t1"]),
        ]);
        let anchors = anchors("t2", &["t2", "a2"], Some("a2"));

        // When: 既定のモードで選ぶ
        let result = select(&dag, &anchors, Mode::Default);

        // Then: 最初に切った t0 と、取り込んだ t1 の両方が分岐点になる
        assert_eq!(result.unmerged, ids(&["a1", "a2"]));
        assert_eq!(result.fork_points, ids(&["t0", "t1"]));
        assert_eq!(result.trunk, ids(&["t0", "t1", "t2"]));
    }

    #[test]
    fn all_mode_shows_every_trunk_ancestor() {
        // Given: trunk t0 ← t1 ← t2、t0 から切った stack-a（a1）
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("t2", &["t1"]),
            ("a1", &["t0"]),
        ]);
        let anchors = anchors("t2", &["t2", "a1"], Some("t2"));

        // When: All モードで選ぶ
        let result = select(&dag, &anchors, Mode::All);

        // Then: trunk の祖先がすべて表示され、未マージと分岐点は既定と同じ
        assert_eq!(result.trunk, ids(&["t0", "t1", "t2"]));
        assert_eq!(result.unmerged, ids(&["a1"]));
        assert_eq!(result.fork_points, ids(&["t0"]));
    }

    #[test]
    fn remote_branches_and_tags_are_not_anchors() {
        // Given: ローカルには main（t1）だけ。remote の origin/feature-r（r1）とタグ v1（v1）がある
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("r1", &["t0"]),
            ("v1", &["t0"]),
        ]);
        let refs = Refs {
            head: Some(Head::Branch {
                name: "main".into(),
                commit: id("t1"),
            }),
            local_branches: vec![("main".into(), id("t1"))],
            remote_branches: vec![("origin/feature-r".into(), id("r1"))],
            tags: vec![("v1".into(), id("v1"))],
            worktrees: vec![],
        };

        // When: refs から起点を作り、既定のモードで選ぶ
        let result = select(&dag, &Anchors::from_refs(&refs, id("t1")), Mode::Default);

        // Then: r1 と v1 は表示対象にならない
        assert!(!result.contains(&id("r1")));
        assert!(!result.contains(&id("v1")));
        assert_eq!(result.trunk, ids(&["t1"]));
    }

    #[test]
    fn linked_worktree_heads_are_anchors() {
        // Given: worktree "review" の HEAD が w1 にあり、worktree "fresh" は unborn
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("w1", &["t0"])]);
        let refs = Refs {
            head: Some(Head::Branch {
                name: "main".into(),
                commit: id("t1"),
            }),
            local_branches: vec![("main".into(), id("t1"))],
            worktrees: vec![
                Worktree {
                    name: "review".into(),
                    head: Some(id("w1")),
                },
                Worktree {
                    name: "fresh".into(),
                    head: None,
                },
            ],
            ..Refs::default()
        };

        // When: refs から起点を作り、既定のモードで選ぶ
        let result = select(&dag, &Anchors::from_refs(&refs, id("t1")), Mode::Default);

        // Then: w1 は未マージとして表示され、unborn の worktree は何も足さない
        assert_eq!(result.unmerged, ids(&["w1"]));
        assert_eq!(result.fork_points, ids(&["t0"]));
    }
}
