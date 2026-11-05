# git-smartlog

A jj-like commit graph for plain git repositories.

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
- Shows trunk and your own branches: unmerged commits (`○`), the trunk commits they were cut from (`◆`), and HEAD (`@`).
- Splits lanes at branch boundaries, so each lane's top shows its branch name.
- Uses only git refs and the commit graph. No dependency on jj or git-branchless.

## Usage

```sh
git smartlog            # trunk and your branches
git smartlog --all      # also every ancestor of trunk
git smartlog --color always | less -R
```

`git-smartlog` on `PATH` is run as `git smartlog`. To use `git sl`:

```sh
git config --global alias.sl smartlog
```

## Configuration

| Key | Default | Meaning |
|---|---|---|
| `smartlog.mainBranch` | `init.defaultBranch`, then `main` | Trunk branch |

## Design

See [ADR-0001](docs/adr/0001-jj-like-smartlog-on-plain-git.md).

## License

MIT
