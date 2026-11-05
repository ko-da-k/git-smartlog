//! Repository access. Only this module depends on gix.

use std::path::Path;

use crate::CommitId;
use crate::config::ConfigSource;
use crate::dag::Dag;

/// Where HEAD points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// On a local branch that has commits.
    Branch { name: String, commit: CommitId },
    /// Detached at a commit.
    Detached(CommitId),
    /// On a branch with no commits yet (fresh repository).
    Unborn { name: String },
}

impl Head {
    pub fn commit(&self) -> Option<&CommitId> {
        match self {
            Head::Branch { commit, .. } | Head::Detached(commit) => Some(commit),
            Head::Unborn { .. } => None,
        }
    }
}

/// A linked worktree (`git worktree add`). The main worktree is represented by [`Head`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worktree {
    pub name: String,
    /// `None` when its HEAD is unborn.
    pub head: Option<CommitId>,
}

/// All refs needed for selection and labels.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refs {
    pub head: Option<Head>,
    /// `refs/heads/*`: (short name, commit).
    pub local_branches: Vec<(String, CommitId)>,
    /// `refs/remotes/*`: (short name such as `origin/main`, commit). Labels only.
    pub remote_branches: Vec<(String, CommitId)>,
    /// `refs/tags/*`, peeled to commits. Labels only.
    pub tags: Vec<(String, CommitId)>,
    pub worktrees: Vec<Worktree>,
}

/// What the rest of the pipeline needs from a repository.
pub trait RepoView: ConfigSource {
    fn refs(&self) -> anyhow::Result<Refs>;

    /// Loads every commit reachable from `heads` (with metadata) into a [`Dag`].
    fn load_dag(&self, heads: &[CommitId]) -> anyhow::Result<Dag>;
}

/// [`RepoView`] backed by gix.
pub struct GixRepo {
    repo: gix::Repository,
}

impl GixRepo {
    /// Opens the repository containing `path` (like `git rev-parse --show-toplevel`).
    pub fn discover(path: &Path) -> anyhow::Result<Self> {
        todo!()
    }
}

impl ConfigSource for GixRepo {
    fn get_string(&self, key: &str) -> Option<String> {
        todo!()
    }
}

impl RepoView for GixRepo {
    fn refs(&self) -> anyhow::Result<Refs> {
        todo!()
    }

    fn load_dag(&self, heads: &[CommitId]) -> anyhow::Result<Dag> {
        todo!()
    }
}
