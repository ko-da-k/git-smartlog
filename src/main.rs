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
    #[arg(long)]
    all: bool,

    /// When to use colors.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,
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
    let mode = if args.all { Mode::All } else { Mode::Default };
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    print!(
        "{}",
        git_smartlog::smartlog(&repo, mode, args.color.resolve(), now)?
    );
    Ok(())
}
