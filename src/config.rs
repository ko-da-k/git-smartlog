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
    ["smartlog.mainBranch", "init.defaultBranch"]
        .into_iter()
        .filter_map(|key| config.get_string(key))
        .find(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "main".to_string())
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
    refs.local_branches
        .iter()
        .find(|(branch, _)| branch == name)
        .map(|(_, commit)| commit.clone())
        .ok_or_else(|| TrunkError::NotFound { name: name.to_string() })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::repo::Head;

    /// Config backed by a map, for tests.
    struct FakeConfig(HashMap<&'static str, &'static str>);

    impl ConfigSource for FakeConfig {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).map(|v| v.to_string())
        }
    }

    #[test]
    fn resolve_trunk_name_follows_priority() {
        let cases: &[(&str, &[(&str, &str)], &str)] = &[
            (
                // Given: 両方の設定がある / When: 解決する / Then: smartlog.mainBranch が優先される
                "smartlog.mainBranch wins",
                &[("smartlog.mainBranch", "develop"), ("init.defaultBranch", "trunk")],
                "develop",
            ),
            (
                // Given: init.defaultBranch だけがある / When: 解決する / Then: それが使われる
                "falls back to init.defaultBranch",
                &[("init.defaultBranch", "trunk")],
                "trunk",
            ),
            (
                // Given: どちらの設定もない / When: 解決する / Then: main になる
                "defaults to main",
                &[],
                "main",
            ),
            (
                // Given: smartlog.mainBranch が空文字列 / When: 解決する / Then: 無視して次の候補に進む
                "ignores empty value",
                &[("smartlog.mainBranch", ""), ("init.defaultBranch", "trunk")],
                "trunk",
            ),
        ];

        for (name, entries, expected) in cases {
            let config = FakeConfig(entries.iter().copied().collect());

            let result = resolve_trunk_name(&config);

            assert_eq!(result, *expected, "case: {name}");
        }
    }

    #[test]
    fn find_trunk_returns_commit_or_not_found() {
        let main_commit = CommitId::new("t0");
        let cases: &[(&str, Refs, Result<CommitId, TrunkError>)] = &[
            (
                // Given: main branch が t0 を指す / When: main を探す / Then: t0 が返る
                "found",
                Refs {
                    head: Some(Head::Branch { name: "main".into(), commit: main_commit.clone() }),
                    local_branches: vec![("main".into(), main_commit.clone())],
                    ..Refs::default()
                },
                Ok(main_commit.clone()),
            ),
            (
                // Given: 別の branch しかない / When: main を探す / Then: NotFound
                "missing",
                Refs {
                    local_branches: vec![("feature-x".into(), CommitId::new("x1"))],
                    ..Refs::default()
                },
                Err(TrunkError::NotFound { name: "main".into() }),
            ),
            (
                // Given: HEAD が main を指すがコミットがない（unborn） / When: main を探す / Then: NotFound
                "unborn",
                Refs {
                    head: Some(Head::Unborn { name: "main".into() }),
                    ..Refs::default()
                },
                Err(TrunkError::NotFound { name: "main".into() }),
            ),
            (
                // Given: remote にだけ main がある / When: main を探す / Then: ローカルではないので NotFound
                "remote only",
                Refs {
                    remote_branches: vec![("origin/main".into(), main_commit.clone())],
                    ..Refs::default()
                },
                Err(TrunkError::NotFound { name: "main".into() }),
            ),
        ];

        for (name, refs, expected) in cases {
            let result = find_trunk(refs, "main");

            assert_eq!(&result, expected, "case: {name}");
        }
    }
}
