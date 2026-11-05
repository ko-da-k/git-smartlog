//! git-smartlog: a jj-like commit graph for plain git repositories.
//!
//! Pipeline (see `docs/adr/0001-jj-like-smartlog-on-plain-git.md`):
//!
//! ```text
//! repo (gix) → dag → select → edges → lanes → order → render
//! ```
//!
//! Everything except `repo` is a pure function over [`dag::Dag`], so it can be
//! tested with small fake DAGs whose commit ids are names like `"a1"`.

pub mod config;
pub mod dag;
pub mod edges;
pub mod lanes;
pub mod order;
pub mod render;
pub mod repo;
pub mod select;

use std::collections::{HashMap, HashSet};
use std::fmt;

use dag::Dag;
use render::{Color, NodeKind, RenderInput};
use repo::{RepoView, Refs};
use select::{Anchors, Mode};

/// Reads refs and commits from `repo` and draws the graph.
pub fn smartlog(repo: &impl RepoView, mode: Mode, color: Color, now: i64) -> anyhow::Result<String> {
    let refs = repo.refs()?;
    let trunk_name = config::resolve_trunk_name(repo);
    let trunk = config::find_trunk(&refs, &trunk_name)?;
    let anchors = Anchors::from_refs(&refs, trunk.clone());
    let heads: Vec<CommitId> = std::iter::once(trunk.clone())
        .chain(anchors.branches.iter().cloned())
        .chain(anchors.head.iter().cloned())
        .chain(anchors.worktree_heads.iter().cloned())
        .collect();
    let dag = repo.load_dag(&heads)?;
    Ok(draw(&dag, &refs, &trunk, mode, color, now))
}

/// The whole pipeline after loading: select → edges → order → lanes → render.
pub fn draw(dag: &Dag, refs: &Refs, trunk: &CommitId, mode: Mode, color: Color, now: i64) -> String {
    let anchors = Anchors::from_refs(refs, trunk.clone());
    let selection = select::select(dag, &anchors, mode);
    let edges = edges::build_edges(dag, &selection);
    let merge_refs = edges::merge_refs(dag, &selection);
    let order = order::row_order(dag, &edges);

    let branch_heads: HashSet<CommitId> =
        refs.local_branches.iter().map(|(_, c)| c.clone()).collect();
    let boundaries = lanes::boundaries(&selection, &edges, &branch_heads);
    let reservations = lanes::reservations(&order, &edges, &selection, &boundaries);

    let mut kinds: HashMap<CommitId, NodeKind> = HashMap::new();
    kinds.extend(selection.trunk.iter().map(|c| (c.clone(), NodeKind::Trunk)));
    kinds.extend(selection.unmerged.iter().map(|c| (c.clone(), NodeKind::Unmerged)));
    if let Some(head) = &anchors.head {
        kinds.insert(head.clone(), NodeKind::Head);
    }
    let labels = render::labels(refs);

    render::render(
        &RenderInput {
            dag,
            order: &order,
            edges: &edges,
            reservations: &reservations,
            merge_refs: &merge_refs,
            kinds: &kinds,
            labels: &labels,
            now,
        },
        color,
    )
}

/// A commit id. Hex object id for real repositories; any name in tests.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CommitId(pub String);

impl CommitId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Abbreviated form used in the graph (first 7 characters).
    pub fn short(&self) -> &str {
        let end = self.0.char_indices().nth(7).map_or(self.0.len(), |(i, _)| i);
        &self.0[..end]
    }
}

impl fmt::Display for CommitId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
