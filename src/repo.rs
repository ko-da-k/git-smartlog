//! Repository access. Only this module depends on gix.

use std::path::Path;

use crate::CommitId;
use crate::config::ConfigSource;
use crate::dag::{CommitMeta, Dag};

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
        Ok(Self {
            repo: gix::discover(path)?,
        })
    }

    fn head(&self) -> anyhow::Result<Head> {
        let head = self.repo.head()?;
        let branch = head.referent_name().map(|n| n.shorten().to_string());
        Ok(match branch {
            Some(name) if head.is_unborn() => Head::Unborn { name },
            Some(name) => Head::Branch {
                name,
                commit: commit_id(self.repo.head_id()?),
            },
            None => Head::Detached(commit_id(self.repo.head_id()?)),
        })
    }

    /// `(short name, commit)` for each reference, skipping `*/HEAD` aliases and
    /// references that do not point to a commit (e.g. tags on trees).
    fn named_commits<'r>(
        refs: impl Iterator<Item = Result<gix::Reference<'r>, gix::Error>>,
    ) -> anyhow::Result<Vec<(String, CommitId)>> {
        let mut result = Vec::new();
        for reference in refs {
            let mut reference = reference?;
            let name = reference.name().shorten().to_string();
            if name.ends_with("/HEAD") {
                continue;
            }
            if let Ok(commit) = reference.peel_to_commit() {
                result.push((name, commit_id(commit.id())));
            }
        }
        Ok(result)
    }

    fn worktrees(&self) -> anyhow::Result<Vec<Worktree>> {
        let mut result = Vec::new();
        for proxy in self.repo.worktrees()? {
            let name = proxy.id()?.to_string();
            let head = proxy
                .into_repo_with_possibly_inaccessible_worktree()
                .ok()
                .and_then(|repo| repo.head_id().ok().map(commit_id));
            result.push(Worktree { name, head });
        }
        Ok(result)
    }
}

fn commit_id(id: impl Into<gix::ObjectId>) -> CommitId {
    CommitId(id.into().to_string())
}

impl ConfigSource for GixRepo {
    fn get_string(&self, key: &str) -> Option<String> {
        self.repo.config_snapshot().string(key).map(|v| v.to_string())
    }
}

impl RepoView for GixRepo {
    fn refs(&self) -> anyhow::Result<Refs> {
        let platform = self.repo.references()?;
        Ok(Refs {
            head: Some(self.head()?),
            local_branches: Self::named_commits(platform.local_branches()?)?,
            remote_branches: Self::named_commits(platform.remote_branches()?)?,
            tags: Self::named_commits(platform.tags()?)?,
            worktrees: self.worktrees()?,
        })
    }

    fn load_dag(&self, heads: &[CommitId]) -> anyhow::Result<Dag> {
        let mut dag = Dag::new();
        let mut stack: Vec<CommitId> = heads.to_vec();
        while let Some(id) = stack.pop() {
            if dag.contains(&id) {
                continue;
            }
            let commit = self.repo.find_commit(gix::ObjectId::from_hex(id.0.as_bytes())?)?;
            let parents: Vec<CommitId> = commit.parent_ids().map(commit_id).collect();
            let meta = CommitMeta {
                summary: commit.message()?.summary().to_string(),
                time: commit.time()?.seconds,
            };
            stack.extend(parents.iter().filter(|p| !dag.contains(p)).cloned());
            dag.insert(id, parents, meta);
        }
        Ok(dag)
    }
}
