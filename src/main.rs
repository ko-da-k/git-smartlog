use std::io::IsTerminal;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Parser, ValueEnum};
use git_smartlog::render::Color;
use git_smartlog::repo::GixRepo;
use git_smartlog::select::Mode;

/// A jj-like commit graph for plain git repositories.
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Also show every ancestor of trunk.
    #[arg(long, short = 'a')]
    all: bool,

    /// Number of trunk commits to show, newest first. Unmerged commits, the trunk
    /// commits they were cut from and HEAD are always shown. Ignored with `--all`.
    #[arg(long, short = 'n', default_value_t = 10)]
    limit: usize,

    /// When to use colors.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,
}

impl Args {
    fn mode(&self) -> Mode {
        if self.all {
            Mode::All
        } else {
            Mode::Recent { limit: self.limit }
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

impl ColorArg {
    /// `auto` colors only when stdout is a terminal and `NO_COLOR` is unset.
    fn resolve(self) -> Color {
        match self {
            ColorArg::Always => Color::Always,
            ColorArg::Never => Color::Never,
            ColorArg::Auto
                if std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() =>
            {
                Color::Always
            }
            ColorArg::Auto => Color::Never,
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let repo = GixRepo::discover(&std::env::current_dir()?)?;
    let mode = args.mode();
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    print!(
        "{}",
        git_smartlog::smartlog(&repo, mode, args.color.resolve(), now)?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_from_arguments() {
        let cases: &[(&str, &[&str], Mode)] = &[
            // Given: 引数なし / When: mode を求める / Then: trunk の直近 10 件
            ("default limit", &[], Mode::Recent { limit: 10 }),
            // Given: --limit 3 / When: mode を求める / Then: trunk の直近 3 件
            (
                "explicit limit",
                &["--limit", "3"],
                Mode::Recent { limit: 3 },
            ),
            // Given: --all / When: mode を求める / Then: limit は使わず trunk の祖先すべて
            ("all ignores limit", &["--all", "--limit", "3"], Mode::All),
            // Given: 短縮形 -a / When: mode を求める / Then: --all と同じく trunk の祖先すべて
            ("short all", &["-a"], Mode::All),
        ];

        for (name, argv, expected) in cases {
            let args =
                Args::try_parse_from(std::iter::once("git-smartlog").chain(argv.iter().copied()))
                    .unwrap_or_else(|e| panic!("case: {name}: {e}"));

            let result = args.mode();

            assert_eq!(result, *expected, "case: {name}");
        }
    }
}
