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
    fn init() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "git-smartlog-e2e-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let repo = Self {
            dir,
            clock: std::cell::Cell::new(0),
        };
        repo.git(&["init", "--quiet", "--initial-branch=main"]);
        repo
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
    let result = smartlog(&repo.open(), Mode::Default, Color::Never, COMMIT_TIME);

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
