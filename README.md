# git-smartlog

jj-like commit graph for plain git repositories.

```
  ○  xxxxxxx (feature-x) Sample commit X
╭─╯
◆  ttttt03 (main) Sample trunk commit 3
│     ○  bbbbb03 (stack-b) Sample commit B3
│   ╭─╯
│   │ ○  ccccc02 (stack-c) Sample commit C2
│   ├─╯
│   ○  bbbbb02 Sample commit B2
│ ╭─╯
│ ○  aaaaa03 (stack-a) Sample commit A3
├─╯
◆  ttttt00 Sample trunk commit 0
```

- Newest first, no connector-only lines, like `jj log`.
- Shows your own branches and recent trunk history: unmerged commits (`○`), the trunk commits they were cut from (`◆`), the newest trunk commits (`◆`, 10 by default), and HEAD (`@`).
- Splits lanes at branch boundaries, so each lane's top shows its branch name.
- Uses only git refs and the commit graph. No dependency on jj or git-branchless.

## Install

Requires Rust 1.88 or later. Install a release from this repository (not published to crates.io):

```sh
cargo install --locked --git https://github.com/ko-da-k/git-smartlog --tag v0.1.0
```

Omit `--tag` to install the latest `main`.

## Usage

```sh
git smartlog            # your branches and the newest 10 trunk commits
git smartlog --limit 3  # only the newest 3 trunk commits (-n 3)
git smartlog --all      # every ancestor of trunk (-a)
git smartlog --color always | less -R
```

`git-smartlog` on `PATH` is run as `git smartlog`. To use `git sl`:

```sh
git config --global alias.sl smartlog
```

In a repository set up with `git branchless init`, `alias.sl` runs git-branchless.
Override it per repository with `git config --local alias.sl smartlog`.

## Configuration

| Key | Default | Meaning |
|---|---|---|
| `smartlog.mainBranch` | `init.defaultBranch`, then `main` | Trunk branch |

## License

MIT
