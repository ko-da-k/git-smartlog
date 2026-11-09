//! End-to-end tests against real repositories created with the `git` command.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use git_smartlog::render::Color;
use git_smartlog::repo::GixRepo;
use git_smartlog::select::Mode;
use git_smartlog::smartlog;

/// Fixed commit time for every commit (2026-01-01T00:00:00Z), so hashes are stable.
const COMMIT_TIME: i64 = 1_767_225_600;

/// A throwaway repository under the system temp dir, isolated from user config.
struct TestRepo {
    dir: PathBuf,
    /// Seconds added to COMMIT_TIME for the next commit, so later commits are newer.
    clock: std::cell::Cell<i64>,
}

impl TestRepo {
    /// An empty, unique directory (no repository yet).
    fn empty() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "git-smartlog-e2e-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self {
            dir,
            clock: std::cell::Cell::new(0),
        }
    }

    fn init() -> Self {
        let repo = Self::empty();
        repo.git(&["init", "--quiet", "--initial-branch=main"]);
        repo
    }

    /// `git clone --depth <depth>` of this repository into a new directory.
    fn shallow_clone(&self, depth: usize) -> Self {
        let clone = Self::empty();
        let url = format!("file://{}", self.dir.display());
        clone.git(&["clone", "--quiet", "--depth", &depth.to_string(), &url, "."]);
        clone
    }

    fn path(&self) -> &Path {
        &self.dir
    }

    /// Runs git with user/system config disabled and fixed identity and dates.
    fn git(&self, args: &[&str]) -> String {
        let date = format!("{} +0000", COMMIT_TIME + self.clock.get());
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Sample")
            .env("GIT_AUTHOR_EMAIL", "sample@example.com")
            .env("GIT_COMMITTER_NAME", "Sample")
            .env("GIT_COMMITTER_EMAIL", "sample@example.com")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("utf-8")
            .trim()
            .to_string()
    }

    /// Creates an empty commit "Sample commit <name>".
    fn commit(&self, name: &str) {
        self.clock.set(self.clock.get() + 1);
        self.git(&[
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            &format!("Sample commit {name}"),
        ]);
    }

    fn open(&self) -> GixRepo {
        GixRepo::discover(self.path()).expect("open repo")
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn unborn_trunk_is_reported_with_a_fix() {
    // Given: git init しただけで、コミットがまだない repo
    let repo = TestRepo::init();

    // When: smartlog を実行する
    let result = smartlog(
        &repo.open(),
        Mode::Recent { limit: 1 },
        Color::Never,
        COMMIT_TIME,
    );

    // Then: trunk の main が見つからないことと、設定方法を伝えるエラーになる
    let message = result.expect_err("should fail").to_string();
    assert!(
        message.contains("trunk branch `main` not found"),
        "{message}"
    );
    assert!(message.contains("smartlog.mainBranch"), "{message}");
}

/// Replaces each abbreviated hash in `output` with its commit's name ("Sample commit <name>").
fn with_names(repo: &TestRepo, output: &str) -> String {
    let mut result = output.to_string();
    for line in repo.git(&["log", "--all", "--format=%H %s"]).lines() {
        let (hash, subject) = line.split_once(' ').expect("hash and subject");
        let name = subject
            .strip_prefix("Sample commit ")
            .expect("sample subject");
        result = result.replace(&hash[..7], name);
    }
    result
}

#[test]
fn draws_adr_example_from_a_real_repository() {
    // Given: ADR の2つ目の例と同じ構造の repo。stack-a ← stack-b ← stack-c を t0 から積み、
    //        stack-b は c2 を切った後に b3 へ進んだ。main は t3 まで進み、HEAD は feature-x
    let repo = TestRepo::init();
    repo.git(&["config", "smartlog.mainBranch", "main"]);
    repo.commit("t0");
    repo.git(&["switch", "--quiet", "-c", "stack-a"]);
    for name in ["a1", "a2", "a3"] {
        repo.commit(name);
    }
    repo.git(&["switch", "--quiet", "-c", "stack-b"]);
    repo.commit("b1");
    repo.commit("b2");
    repo.git(&["switch", "--quiet", "-c", "stack-c"]);
    repo.commit("c2");
    repo.git(&["switch", "--quiet", "stack-b"]);
    repo.commit("b3");
    repo.git(&["switch", "--quiet", "main"]);
    for name in ["t1", "t2", "t3"] {
        repo.commit(name);
    }
    repo.git(&["switch", "--quiet", "-c", "feature-x"]);
    repo.commit("x");
    let now = COMMIT_TIME + 3 * 86_400 + 100;

    // When: --all で、色なしで描く
    let output = smartlog(&repo.open(), Mode::All, Color::Never, now).expect("smartlog");

    // Then: ハッシュを名前に置き換えると、ADR の図（render のテスト 31 と同じ期待値）と一致する
    let expected = "  @  x 3d (ᐅ feature-x) Sample commit x
╭─╯
◆  t3 3d (main) Sample commit t3
◆  t2 3d Sample commit t2
◆  t1 3d Sample commit t1
│     ○  b3 3d (stack-b) Sample commit b3
│   ╭─╯
│   │ ○  c2 3d (stack-c) Sample commit c2
│   ├─╯
│   ○  b2 3d Sample commit b2
│   ○  b1 3d Sample commit b1
│ ╭─╯
│ ○  a3 3d (stack-a) Sample commit a3
│ ○  a2 3d Sample commit a2
│ ○  a1 3d Sample commit a1
├─╯
◆  t0 3d Sample commit t0
";
    let actual: String = with_names(&repo, &output)
        .lines()
        .map(|l| format!("{}\n", l.trim_end()))
        .collect();
    assert_eq!(actual, expected, "\n{actual}");
}

#[test]
fn git_smartlog_runs_binary_on_path_before_alias() {
    // Given: branchless と同じく alias.smartlog が設定された repo と、PATH 上の git-smartlog
    let repo = TestRepo::init();
    repo.commit("t0");
    repo.git(&["config", "alias.smartlog", "!echo from-alias"]);
    let binary = Path::new(env!("CARGO_BIN_EXE_git-smartlog"));
    let path = format!(
        "{}:{}",
        binary.parent().expect("bin dir").display(),
        std::env::var("PATH").unwrap_or_default()
    );

    // When: `git smartlog` を実行する
    let output = Command::new("git")
        .args(["smartlog", "--color", "never"])
        .current_dir(repo.path())
        .env("PATH", path)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("run git smartlog");

    // Then: alias ではなく git-smartlog が動き、グラフを出力する
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!stdout.contains("from-alias"), "{stdout}");
    assert!(stdout.contains("Sample commit t0"), "{stdout}");
}

#[test]
fn recent_mode_shows_trunk_history_including_merged_commits() {
    // Given: main に initial commit t0 があり、feature（a1 ← a2）を --no-ff でマージした（m）。
    //        未マージの branch はなく、HEAD は main
    let repo = TestRepo::init();
    repo.commit("t0");
    repo.git(&["switch", "--quiet", "-c", "feature"]);
    repo.commit("a1");
    repo.commit("a2");
    repo.git(&["switch", "--quiet", "main"]);
    repo.clock.set(repo.clock.get() + 1);
    repo.git(&[
        "merge",
        "--quiet",
        "--no-ff",
        "-m",
        "Sample commit m",
        "feature",
    ]);
    repo.git(&["branch", "--quiet", "-d", "feature"]);
    let now = COMMIT_TIME + 100;

    // When: 既定の limit（10 件）で、色なしで描く
    let output =
        smartlog(&repo.open(), Mode::Recent { limit: 10 }, Color::Never, now).expect("smartlog");

    // Then: マージ m、取り込まれた a2, a1、initial commit t0 がすべて trunk として表示される
    let actual: String = with_names(&repo, &output)
        .lines()
        .map(|l| format!("{}\n", l.trim_end()))
        .collect();
    let expected = "@    m 1m (ᐅ main) Sample commit m
├─╮
│ ◆  a2 1m Sample commit a2
│ ◆  a1 1m Sample commit a1
├─╯
◆  t0 1m Sample commit t0
";
    assert_eq!(actual, expected, "\n{actual}");
}

#[test]
fn shallow_clone_treats_boundary_commits_as_roots() {
    // Given: main に t0 ← t1 ← t2 がある repo を --depth 2 で clone した（t1 が shallow の境界で、t0 は手元にない）
    let origin = TestRepo::init();
    for name in ["t0", "t1", "t2"] {
        origin.commit(name);
    }
    let repo = origin.shallow_clone(2);
    let now = COMMIT_TIME + 100;

    // When: 既定の limit（10 件）で、色なしで描く
    let output =
        smartlog(&repo.open(), Mode::Recent { limit: 10 }, Color::Never, now).expect("smartlog");

    // Then: 失敗せず、境界の t1 が親のない最後の行として描かれ、手元にない t0 は現れない
    let actual: String = with_names(&repo, &output)
        .lines()
        .map(|l| format!("{}\n", l.trim_end()))
        .collect();
    let expected = "@  t2 1m (ᐅ main, origin/main) Sample commit t2
◆  t1 1m Sample commit t1
";
    assert_eq!(actual, expected, "\n{actual}");
}
