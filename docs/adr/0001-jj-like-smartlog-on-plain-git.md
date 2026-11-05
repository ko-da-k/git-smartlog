# ADR-0001: git-smartlog — a jj-like commit graph from plain git data

## Status

Accepted

## Context

- Look like `jj log`: newest first, no connector lines, `◆` for trunk, `○` for work in progress, `@` for HEAD.
- By default, show only what relates to trunk and our own branches.
- Depend on neither git-branchless nor jj; use only git refs and the DAG.

## Decision

### Build a new Rust CLI, `git-smartlog`

- Dependencies: `gix` (repository access), `sapling-renderdag` (graph rendering).
  - sapling-renderdag is the renderer behind `jj log`, so lane shapes match jj.
    Row order, glyphs, elided edges and compact rows (`with_min_row_height(0)`) are ours to set.
- Read-only; no rebase / undo.
- Uses only git refs and the DAG.
- Does not read branchless's event log or jj metadata (`.jj/`, `refs/jj/*`).
- Trunk is a local branch, resolved in order:
  `smartlog.mainBranch` → `init.defaultBranch` → `main`.

### Displayed set

| Term | Definition | Glyph |
|---|---|---|
| trunk | Reachable from trunk | `◆` |
| unmerged | Reachable from an anchor, not from trunk | `○` |
| HEAD | Checked-out commit | `@` |
| anchor | Local branches (`refs/heads/*`), HEAD, each worktree's HEAD | — |
| fork point | Parent of an unmerged commit that is on trunk | Same as the commit itself (`◆`, since it is on trunk) |

- "Fork point" is a reason for inclusion, not a commit kind; the glyph follows whether the commit is on trunk or unmerged.
- When a branch is cut from another unmerged branch (stacked PRs), its base is an unmerged commit (`○`). It is already displayed as unmerged, so no special handling is needed.

```
unmerged    = ancestors(anchors) − ancestors(trunk)
fork points = parents(unmerged) ∩ ancestors(trunk)
displayed   = unmerged ∪ fork points ∪ trunk tip ∪ anchors on trunk   (default)
            + all ancestors of trunk                                  (--all)
```

- Remote branches and tags are labels only; they do not affect the displayed set.
- A worktree's HEAD is an anchor, like jj's `<workspace>@`, labeled `<wt>@`.

### Split lanes at branch boundaries

jj does not treat bookmarks as boundaries, so a branch cut from the tip of `main` (e.g. `feature-x`) is stacked in the same column.
Git workflows are branch-based, so we split lanes at branch boundaries; each lane's top shows its branch name.

- Boundary: the parent of an unmerged commit is any of:
  1. on trunk
  2. has a local branch
  3. has two or more displayed children
- At a boundary, no child sits directly above the parent; all children are side lanes merging into it.
  Branches sharing a parent therefore look the same regardless of the parent's kind, and none looks like the parent's continuation. Ordering only changes which lane is on top.
- Stacked PRs follow the same rule; each level shifts one column right.
- Implementation: use sapling-renderdag's `reserve` to claim the parent's column first.
  Before drawing a lane's top, walk first parents to collect the boundaries below it and reserve them, nearest to trunk first.
  (Reserving right before each child is too late: the child already holds a column, so the parent lands to the right and the merge is drawn awkwardly.)

Example (hashes, branch names and messages are made-up samples; `stack-a` → `stack-b` → `stack-c` are stacked PRs):

```
  ○  xxxxxxx (feature-x) Sample commit X
╭─╯
◆  ttttt03 (main) Sample trunk commit 3
◆  ttttt02 Sample trunk commit 2
◆  ttttt01 Sample trunk commit 1
│     ○  ccccc02 (stack-c) Sample commit C2
│   ╭─╯
│   ○  bbbbb02 (stack-b) Sample commit B2
│   ○  bbbbb01 Sample commit B1
│ ╭─╯
│ ○  aaaaa03 (stack-a) Sample commit A3
│ ○  aaaaa02 Sample commit A2
│ ○  aaaaa01 Sample commit A1
├─╯
◆  ttttt00 Sample trunk commit 0
```

After `bbbbb03` is added to stack-b, `bbbbb02` loses its branch name, but it has two displayed children (`bbbbb03` and `ccccc02`), so it is a boundary. Both are side lanes merging into `bbbbb02`, showing `ccccc02` was cut from `bbbbb02`:

```
  ○  xxxxxxx (feature-x) Sample commit X
╭─╯
◆  ttttt03 (main) Sample trunk commit 3
◆  ttttt02 Sample trunk commit 2
◆  ttttt01 Sample trunk commit 1
│     ○  bbbbb03 (stack-b) Sample commit B3
│   ╭─╯
│   │ ○  ccccc02 (stack-c) Sample commit C2
│   ├─╯
│   ○  bbbbb02 Sample commit B2
│   ○  bbbbb01 Sample commit B1
│ ╭─╯
│ ○  aaaaa03 (stack-a) Sample commit A3
│ ○  aaaaa02 Sample commit A2
│ ○  aaaaa01 Sample commit A1
├─╯
◆  ttttt00 Sample trunk commit 0
```

- Without rule 3, the child drawn first stays above `bbbbb02`, so its branch looks like it owns `bbbbb01` and `bbbbb02`.
  Which child that is depends on commit time, so the shape flips whenever you switch the branch you work on.

(Both outputs are from a prototype with sapling-renderdag 0.1.0.)

### Show merged commits as references

When a branch merges trunk in (e.g. "Merge branch 'main' into stack-a"), a line to the merged trunk commit would cross other lanes.
Instead, show which commit was merged under the merge row.

- Unmerged merge commits: draw only the first-parent edge. List each other parent under the row as `↑ merge: <hash> <summary>`, whether or not it is displayed.
- Merge commits on trunk (e.g. a PR merge into `main`): no reference rows; draw edges to displayed parents as usual.
- The arrow is always `↑`: changes flow up from the merged commit into the merge (older rows are lower). It does not point to where the commit is drawn, which may be hidden.

Example (made-up samples; `aaaaa03` merges `ttttt01` into stack-a):

```
◆  ttttt02 (main) Sample trunk commit 2
│ ○  aaaaa03 (stack-a) Merge branch 'main' into stack-a
│ │  ↑ merge: ttttt01 Sample trunk commit 1
│ ○  aaaaa02 Sample commit A2
│ ○  aaaaa01 Sample commit A1
◆ │  ttttt01 Sample trunk commit 1
├─╯
◆  ttttt00 Sample trunk commit 0
```

The reference is the second line of the row's message, so renderdag draws it beside the lane.

### Structure

```
src/
  main.rs     CLI (clap): --all, --color auto|always|never
  config.rs   resolve trunk
  repo.rs     trait RepoView (thin layer over gix) + GixRepo
  dag.rs      oid → parents map, ancestor helpers
  select.rs   displayed set → Selection { trunk, unmerged, fork_points }
  edges.rs    edges between displayed nodes (Parent / Ancestor / Anonymous)
  order.rs    newest first, stacks kept contiguous
  render.rs   render with sapling-renderdag (BoxDrawing, min_row_height = 0), colorize
```

- gix stays behind a trait; select / edges / order are pure functions, testable with named fake DAGs.

