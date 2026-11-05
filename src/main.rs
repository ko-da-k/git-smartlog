use clap::{Parser, ValueEnum};

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

fn main() -> anyhow::Result<()> {
    let _args = Args::parse();
    todo!()
}
