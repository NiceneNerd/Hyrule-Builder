#![feature(option_result_contains)]
use std::path::PathBuf;
use structopt::StructOpt;

mod builder;
mod unbuilder;
mod unzip_some;
mod util;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Hyrule Builder",
    about = "Mod building tool for The Legend of Zelda: Breath of the Wild",
    version = env!("CARGO_PKG_VERSION"),
    rename_all = "kebab-case",
    setting = structopt::clap::AppSettings::ColoredHelp
)]
struct Opt {
    #[structopt(long, short, help = "Show detailed output")]
    verbose: bool,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub(crate) enum Command {
    /// Builds a mod from a source-like structure into binary game files
    /// {n}Note: Flags can be set using a config.yml file. See readme for details.
    #[structopt(setting = structopt::clap::AppSettings::ColoredHelp)]
    Build {
        #[structopt(long, short, help = "Use big endian/Wii U mode")]
        be: bool,
        #[structopt(long, short, help = "Suppress warnings, show only errors")]
        ignore_warnings: bool,
        #[structopt(long, short, help = "Treat warnings as errors and abort")]
        hard_warnings: bool,
        #[structopt(
            long,
            short,
            use_delimiter = true,
            help = "Comma separated list of custom actors to add to TitleBG.pack, e.g.\n`--title-actors=Weapon_Bow_001,Enemy_Golem_Senior`"
        )]
        title_actors: Vec<String>,
        #[structopt(long, short, help = "Output folder for built mod")]
        output: Option<PathBuf>,
        #[structopt(long, short, help = "Source mod folder to build")]
        source: Option<PathBuf>,
    },
    /// Creates a new source-like mod project
    #[structopt(
        setting = structopt::clap::AppSettings::ColoredHelp,
        alias = "unbuild"
    )]
    Init {
        #[structopt(long, short, help = "Use big endian/Wii U mode")]
        be: bool,
        #[structopt(help = "Target folder to create project in [default: .]")]
        directory: Option<PathBuf>,
        #[structopt(long, short, help = "Source mod folder to unbuild")]
        source: Option<PathBuf>,
        #[structopt(long, short, help = "Create default config.yml")]
        config: bool,
    },
}

fn main() -> Result<(), anyhow::Error> {
    let opt = Opt::from_args();
    if cfg!(debug) {
        println!("{:?}", opt);
    }
    match opt.command {
        Command::Init {
            be,
            directory,
            source,
            config,
        } => unbuilder::unbuild(be, directory, source, config),
        Command::Build {
            be,
            hard_warnings,
            ignore_warnings,
            output,
            title_actors,
            source,
        } => builder::build(
            source.unwrap_or_else(|| PathBuf::from("./")),
            be,
            hard_warnings,
            ignore_warnings,
            opt.verbose,
            output,
            title_actors,
        ),
    }
}
