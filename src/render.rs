//! Drawing the graph with sapling-renderdag.

use std::collections::HashMap;

use renderdag::{Ancestor, GraphRowRenderer, Renderer};

use crate::CommitId;
use crate::dag::Dag;
use crate::edges::Edge;
use crate::repo::{Head, Refs};

/// What kind of node a row is; decides the glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// `@`
    Head,
    /// `○`
    Unmerged,
    /// `◆`
    Trunk,
}

/// A name shown next to a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Label {
    /// Local branch; `current` is true when HEAD is on it (shown with `ᐅ`).
    Branch { name: String, current: bool },
    /// Linked worktree's HEAD, shown as `<name>@`.
    Worktree(String),
    Tag(String),
    Remote(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    Always,
    Never,
}

/// Everything needed to draw, already computed by the earlier stages.
pub struct RenderInput<'a> {
    pub dag: &'a Dag,
    pub order: &'a [CommitId],
    pub edges: &'a HashMap<CommitId, Vec<Edge>>,
    pub reservations: &'a HashMap<CommitId, Vec<CommitId>>,
    /// Non-first parents of unmerged merges, written as `↑ merge:` lines under the row.
    pub merge_refs: &'a HashMap<CommitId, Vec<CommitId>>,
    pub kinds: &'a HashMap<CommitId, NodeKind>,
    pub labels: &'a HashMap<CommitId, Vec<Label>>,
    /// "Now" for relative dates, in seconds since the Unix epoch (fixed in tests).
    pub now: i64,
}

pub fn render(input: &RenderInput<'_>, color: Color) -> String {
    let mut renderer = GraphRowRenderer::new()
        .output()
        .with_min_row_height(0)
        .build_box_drawing();
    let paint = Painter(color);
    let mut out = String::new();

    for id in input.order {
        for boundary in input.reservations.get(id).into_iter().flatten() {
            renderer.reserve(boundary.clone());
        }
        let parents = input.edges[id].iter().map(to_ancestor).collect();
        let kind = input.kinds[id];
        let glyph = paint.glyph(kind);
        let message = row_message(input, id, kind, &paint);
        out.push_str(&renderer.next_row(id.clone(), parents, glyph, message));
    }
    out
}

/// Labels per commit from repository refs, in display order:
/// local branches, worktrees, tags, remote branches.
pub fn labels(refs: &Refs) -> HashMap<CommitId, Vec<Label>> {
    let current = match &refs.head {
        Some(Head::Branch { name, .. }) => Some(name.as_str()),
        _ => None,
    };
    let mut map: HashMap<CommitId, Vec<Label>> = HashMap::new();
    let mut add = |commit: &CommitId, label| map.entry(commit.clone()).or_default().push(label);

    for (name, commit) in &refs.local_branches {
        let current = current == Some(name.as_str());
        add(commit, Label::Branch { name: name.clone(), current });
    }
    for worktree in &refs.worktrees {
        if let Some(commit) = &worktree.head {
            add(commit, Label::Worktree(worktree.name.clone()));
        }
    }
    for (name, commit) in &refs.tags {
        add(commit, Label::Tag(name.clone()));
    }
    for (name, commit) in &refs.remote_branches {
        add(commit, Label::Remote(name.clone()));
    }
    map
}

fn to_ancestor(edge: &Edge) -> Ancestor<CommitId> {
    match edge {
        Edge::Parent(id) => Ancestor::Parent(id.clone()),
        Edge::Ancestor(id) => Ancestor::Ancestor(id.clone()),
        Edge::Anonymous => Ancestor::Anonymous,
    }
}

/// `<hash> <age> (<labels>) <summary>`, then one `↑ merge:` line per merged parent.
fn row_message(input: &RenderInput<'_>, id: &CommitId, kind: NodeKind, paint: &Painter) -> String {
    let meta = input.dag.meta(id);
    let trunk = kind == NodeKind::Trunk;

    let mut parts = vec![paint.hash(id.short(), trunk)];
    if let Some(meta) = meta {
        parts.push(paint.paint(GREEN, &relative_age(input.now - meta.time)));
    }
    if let Some(labels) = input.labels.get(id).filter(|l| !l.is_empty()) {
        let text: Vec<String> = labels.iter().map(Label::text).collect();
        parts.push(paint.paint(BRIGHT_GREEN, &format!("({})", text.join(", "))));
    }
    if let Some(meta) = meta {
        parts.push(if trunk { paint.paint(DIM, &meta.summary) } else { meta.summary.clone() });
    }

    let mut message = parts.join(" ");
    for merged in input.merge_refs.get(id).into_iter().flatten() {
        let summary = input.dag.meta(merged).map_or("", |m| m.summary.as_str());
        let line = format!("↑ merge: {} {summary}", merged.short());
        message.push('\n');
        message.push_str(&paint.paint(DIM, line.trim_end()));
    }
    message
}

/// Compact age such as `5m`, `3h`, `12d`, `2y`.
fn relative_age(seconds: i64) -> String {
    let seconds = seconds.max(0);
    match seconds {
        s if s < 3_600 => format!("{}m", s / 60),
        s if s < 86_400 => format!("{}h", s / 3_600),
        s if s < 365 * 86_400 => format!("{}d", s / 86_400),
        s => format!("{}y", s / (365 * 86_400)),
    }
}

impl Label {
    fn text(&self) -> String {
        match self {
            Label::Branch { name, current: true } => format!("ᐅ {name}"),
            Label::Branch { name, current: false } => name.clone(),
            Label::Worktree(name) => format!("{name}@"),
            Label::Tag(name) | Label::Remote(name) => name.clone(),
        }
    }
}

const DIM: &str = "2";
const GREEN: &str = "32";
const BRIGHT_GREEN: &str = "92";

/// Applies ANSI styles only when colors are enabled.
struct Painter(Color);

impl Painter {
    fn paint(&self, style: &str, text: &str) -> String {
        match self.0 {
            Color::Always => format!("\x1b[{style}m{text}\x1b[0m"),
            Color::Never => text.to_string(),
        }
    }

    fn glyph(&self, kind: NodeKind) -> String {
        match kind {
            NodeKind::Head => self.paint("1;32", "@"),
            NodeKind::Unmerged => self.paint("1;34", "○"),
            NodeKind::Trunk => self.paint("36", "◆"),
        }
    }

    fn hash(&self, short: &str, trunk: bool) -> String {
        self.paint(if trunk { "2;33" } else { "1;35" }, short)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::testing::{dag, id};
    use crate::draw;
    use crate::repo::{Head, Worktree};
    use crate::select::Mode;

    /// 3 days after the fake DAG's commits (whose times are their indexes), so every age is "3d".
    const NOW: i64 = 3 * 86_400 + 100;

    /// Refs with local `branches` as (name, commit) and HEAD on branch `head`.
    fn refs(branches: &[(&str, &str)], head: &str) -> Refs {
        let local_branches: Vec<(String, CommitId)> =
            branches.iter().map(|(n, c)| (n.to_string(), id(c))).collect();
        let commit = local_branches.iter().find(|(n, _)| n == head).expect("head branch").1.clone();
        Refs {
            head: Some(Head::Branch { name: head.into(), commit }),
            local_branches,
            ..Refs::default()
        }
    }

    /// The glyph drawn on the row whose hash is `name`.
    fn glyph_of(output: &str, name: &str) -> char {
        let marker = format!("  {name} ");
        let line = output.lines().find(|l| l.contains(&marker)).unwrap_or_else(|| panic!("{name}: {output}"));
        let before = &line[..line.find(&marker).unwrap()];
        before.chars().last().expect("glyph")
    }

    #[test]
    fn glyphs_follow_commit_kind() {
        // Given: trunk t0 ← t1（main）、t0 から切った stack-a（a1 ← a2）。HEAD は stack-a
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("a1", &["t0"]), ("a2", &["a1"])]);
        let refs = refs(&[("main", "t1"), ("stack-a", "a2")], "stack-a");

        // When: 色なしで描く
        let output = draw(&dag, &refs, &id("t1"), Mode::Default, Color::Never, NOW);

        // Then: HEAD の a2 は @、未マージの a1 は ○、trunk の t1 と分岐点 t0 は ◆
        assert_eq!(glyph_of(&output, "a2"), '@', "{output}");
        assert_eq!(glyph_of(&output, "a1"), '○', "{output}");
        assert_eq!(glyph_of(&output, "t1"), '◆', "{output}");
        assert_eq!(glyph_of(&output, "t0"), '◆', "{output}");
    }

    #[test]
    fn no_connector_only_lines_between_linear_commits() {
        // Given: trunk t0 から切った stack-a（a1 ← a2 ← a3）
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("a1", &["t0"]), ("a2", &["a1"]), ("a3", &["a2"])]);
        let refs = refs(&[("main", "t1"), ("stack-a", "a3")], "main");

        // When: 色なしで描く
        let output = draw(&dag, &refs, &id("t1"), Mode::Default, Color::Never, NOW);

        // Then: 縦線と空白だけの行（連結行）が1つもない
        assert!(
            output.lines().all(|l| !l.chars().all(|c| c == '│' || c == ' ')),
            "{output}"
        );
    }

    #[test]
    fn boundaries_split_lanes_as_in_adr_example() {
        // Given: ADR の2つ目の例。stack-a ← stack-b ← stack-c を t0 から積み、stack-b は b3 に進んだ。HEAD は feature-x
        let dag = dag(&[
            ("t0", &[]),
            ("a1", &["t0"]),
            ("a2", &["a1"]),
            ("a3", &["a2"]),
            ("b1", &["a3"]),
            ("b2", &["b1"]),
            ("c2", &["b2"]),
            ("b3", &["b2"]),
            ("t1", &["t0"]),
            ("t2", &["t1"]),
            ("t3", &["t2"]),
            ("x", &["t3"]),
        ]);
        let refs = refs(
            &[("main", "t3"), ("feature-x", "x"), ("stack-a", "a3"), ("stack-b", "b3"), ("stack-c", "c2")],
            "feature-x",
        );

        // When: --all で、色なしで描く
        let output = draw(&dag, &refs, &id("t3"), Mode::All, Color::Never, NOW);

        // Then: ADR の図と同じ形になる（境目では、どの子も親の真上に来ず横から合流する）
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
        let actual: String = output.lines().map(|l| format!("{}\n", l.trim_end())).collect();
        assert_eq!(actual, expected, "\n{actual}");
    }

    #[test]
    fn labels_show_branches_worktrees_tags_and_remotes() {
        // Given: t1 に main（HEAD）、worktree review、タグ v1、remote の origin/main がある
        let dag = dag(&[("t0", &[]), ("t1", &["t0"])]);
        let mut refs = refs(&[("main", "t1")], "main");
        refs.worktrees = vec![Worktree { name: "review".into(), head: Some(id("t1")) }];
        refs.tags = vec![("v1".into(), id("t1"))];
        refs.remote_branches = vec![("origin/main".into(), id("t1"))];

        // When: 色なしで描く
        let output = draw(&dag, &refs, &id("t1"), Mode::Default, Color::Never, NOW);

        // Then: ローカル branch（HEAD は ᐅ 付き）、worktree、タグ、remote の順に並ぶ
        assert!(output.contains("(ᐅ main, review@, v1, origin/main)"), "{output}");
    }

    #[test]
    fn ansi_codes_only_with_color_always() {
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("a1", &["t0"])]);
        let refs = refs(&[("main", "t1"), ("stack-a", "a1")], "stack-a");
        let cases = [
            // Given: 同じ DAG / When: Never で描く / Then: ANSI の色コードを含まない
            (Color::Never, false),
            // Given: 同じ DAG / When: Always で描く / Then: ANSI の色コードを含む
            (Color::Always, true),
        ];

        for (color, has_ansi) in cases {
            let output = draw(&dag, &refs, &id("t1"), Mode::Default, color, NOW);

            assert_eq!(output.contains('\x1b'), has_ansi, "{color:?}: {output}");
        }
    }

    #[test]
    fn merged_commit_is_written_under_unmerged_merge() {
        // Given: stack-a の a2 は、a1 と trunk の t1 のマージ
        let dag = dag(&[("t0", &[]), ("t1", &["t0"]), ("t2", &["t1"]), ("a1", &["t0"]), ("a2", &["a1", "t1"])]);
        let refs = refs(&[("main", "t2"), ("stack-a", "a2")], "main");

        // When: 色なしで描く
        let output = draw(&dag, &refs, &id("t2"), Mode::Default, Color::Never, NOW);

        // Then: a2 の行のすぐ下に、取り込んだ t1 が ↑ merge: の行で書かれる
        let lines: Vec<&str> = output.lines().collect();
        let row = lines.iter().position(|l| l.contains("  a2 ")).expect("a2 row");
        assert!(lines[row + 1].trim_end().ends_with("↑ merge: t1 Sample commit t1"), "{output}");
    }
}
