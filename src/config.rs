//! Resolving which branch is trunk.

use crate::CommitId;
use crate::repo::Refs;

/// Read access to git config. Implemented by the gix repository and by test fakes.
pub trait ConfigSource {
    fn get_string(&self, key: &str) -> Option<String>;
}

/// Name of the trunk branch:
/// `smartlog.mainBranch` → `init.defaultBranch` → `"main"`.
pub fn resolve_trunk_name(config: &impl ConfigSource) -> String {
    todo!()
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum TrunkError {
    /// The resolved trunk branch does not exist (or is unborn).
    #[error(
        "trunk branch `{name}` not found; create it or set `git config smartlog.mainBranch <branch>`"
    )]
    NotFound { name: String },
}

/// Commit the trunk branch points to.
pub fn find_trunk(refs: &Refs, name: &str) -> Result<CommitId, TrunkError> {
    todo!()
}
