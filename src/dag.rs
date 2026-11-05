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
        self.parents.insert(id.clone(), parents);
        self.meta.insert(id, meta);
    }

    pub fn contains(&self, id: &CommitId) -> bool {
        self.parents.contains_key(id)
    }

    /// Parents in order. Empty for root commits and unknown ids.
    pub fn parents(&self, id: &CommitId) -> &[CommitId] {
        self.parents.get(id).map_or(&[], Vec::as_slice)
    }

    pub fn meta(&self, id: &CommitId) -> Option<&CommitMeta> {
        self.meta.get(id)
    }

    /// All ancestors of `heads`, including the heads themselves.
    ///
    /// Iterative, so deep histories do not overflow the stack.
    pub fn ancestors<'a>(&self, heads: impl IntoIterator<Item = &'a CommitId>) -> HashSet<CommitId> {
        let mut seen = HashSet::new();
        let mut stack: Vec<&CommitId> = heads.into_iter().collect();
        while let Some(id) = stack.pop() {
            if seen.insert(id.clone()) {
                stack.extend(self.parents(id));
            }
        }
        seen
    }
}

/// Helpers for building fake DAGs in tests.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    pub fn id(name: &str) -> CommitId {
        CommitId::new(name)
    }

    pub fn ids(names: &[&str]) -> HashSet<CommitId> {
        names.iter().map(|n| id(n)).collect()
    }

    /// Builds a DAG from `(commit, parents)` pairs. Commit time is the pair's index,
    /// so later entries are newer.
    pub fn dag(edges: &[(&str, &[&str])]) -> Dag {
        let mut dag = Dag::new();
        for (time, (commit, parents)) in edges.iter().enumerate() {
            dag.insert(
                id(commit),
                parents.iter().map(|p| id(p)).collect(),
                CommitMeta {
                    summary: format!("Sample commit {commit}"),
                    time: time as i64,
                },
            );
        }
        dag
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{dag, id, ids};

    #[test]
    fn ancestors_include_heads_and_both_parents_of_merges() {
        // Given: t0 ← t1 ← m（m は t1 と a1 のマージ）、a1 ← t0、無関係な x0
        let dag = dag(&[
            ("t0", &[]),
            ("t1", &["t0"]),
            ("a1", &["t0"]),
            ("m", &["t1", "a1"]),
            ("x0", &[]),
        ]);

        // When: m の祖先を求める
        let result = dag.ancestors([&id("m")]);

        // Then: m 自身と、両方の親を辿った祖先が含まれ、無関係な x0 は含まれない
        assert_eq!(result, ids(&["m", "t1", "a1", "t0"]));
    }

    #[test]
    fn parents_of_unknown_id_is_empty() {
        // Given: t0 だけを持つ DAG
        let dag = dag(&[("t0", &[])]);

        // When: 登録されていない id の親を求める
        let result = dag.parents(&id("unknown"));

        // Then: 空になり、パニックしない
        assert!(result.is_empty());
    }
}
