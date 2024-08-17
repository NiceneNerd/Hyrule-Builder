use crate::{
    builder::BuildConfig,
    settings::{ConfigCommand, Settings},
};
use add::AddCommand;
use anyhow::{anyhow, Result};
use botw_utils::hashes::{Platform, StockHashTable};
use builder::WarnLevel;
use colored::*;
use fs_err as fs;
use parking_lot::RwLock;
use roead::yaz0::decompress;
use rstb::ResourceSizeTable;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use structopt::{clap::AppSettings::ColoredHelp, StructOpt};

mod add;
mod builder;
mod settings;
mod unbuilder;
mod unzip_some;
mod util;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "Hyrule Builder",
    about = "Mod building tool for The Legend of Zelda: Breath of the Wild",
    version = env!("CARGO_PKG_VERSION"),
    rename_all = "kebab-case",
    setting = ColoredHelp
)]
struct Opt {
    #[structopt(long, short, help = "Show detailed output")]
    verbose: bool,
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum Command {
    /// Build a mod from a source-like structure into binary game files
    /// {n}Note: Flags can be set using a config.yml file. See readme for details.
    #[structopt(setting = ColoredHelp)]
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
        #[structopt(help = "Source mod folder to build")]
        source: Option<PathBuf>,
        #[structopt(long, short, help = "Output folder for built mod")]
        output: Option<PathBuf>,
    },
    /// Create a new source-like mod project
    #[structopt(setting = ColoredHelp, alias = "unbuild")]
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
    /// Get or set Hyrule Builder configuration parameters
    #[structopt(setting = ColoredHelp, alias = "conf")]
    Config(ConfigCommand),
    /// Add new content to the active mod project
    #[structopt(setting = ColoredHelp)]
    Add {
        #[structopt(long, short, default_value = ".", help = "Project folder to add to")]
        project: PathBuf,
        #[structopt(subcommand)]
        command: AddCommand,
    },
}

fn check_project(project: &Path) -> Result<bool> {
    if !project.join(".db").exists() {
        Err(anyhow!(
            "The specified folder is not a Hyrule Builder project"
        ))
    } else if project.join("content").exists() || project.join("aoc").exists() {
        Ok(true)
    } else if project.join("01007EF00011E000").exists() || project.join("01007EF00011F001").exists()
    {
        Ok(false)
    } else {
        Err(anyhow!(
            "The specified folder is not a valid Hyrule Builder project"
        ))
    }
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    match opt.command {
        Command::Config(conf) => {
            let mut config = Settings::get_settings()?;
            match conf {
                ConfigCommand::List { values } => {
                    config.list(values);
                }
                ConfigCommand::Get { setting } => {
                    config.get(&setting)?;
                }
                ConfigCommand::Set { setting, value } => {
                    config.set(&setting, &value)?;
                }
                ConfigCommand::Import {
                    from_bcml,
                    from_cemu,
                    cemu_dir,
                } => {
                    if from_cemu && from_bcml {
                        return Err(anyhow!("Cannot combine `--from-cemu` and `--from-bcml`"));
                    }
                    if from_cemu {
                        config.set_from_cemu(&cemu_dir.unwrap())?;
                    } else if from_bcml {
                        config.set_from_bcml()?;
                    }
                }
            }
            Ok(())
        }
        Command::Init {
            be,
            directory,
            source,
            config,
        } => unbuilder::unbuild(be, source, directory, config),
        Command::Build {
            be,
            hard_warnings,
            ignore_warnings,
            output,
            title_actors,
            source,
        } => {
            let source = source.map_or(
                std::env::current_dir()
                    .expect("There's no current working directory"),
                |path| {
                    dunce::canonicalize(path).expect("Provided path was invalid")
            });
            let config: Option<BuildConfig> = if source.join("config.yml").exists() {
                Some(serde_yml::from_reader(&std::fs::File::open(
                    source.join("config.yml"),
                )?)?)
            } else {
                None
            };
            let be = config
                .as_ref()
                .map(|c| c.flags.contains(&"be".to_string()) || be)
                .unwrap_or(be);
            let hard_warnings = config
                .as_ref()
                .map(|c| c.flags.contains(&"hard_warnings".to_owned()) || hard_warnings)
                .unwrap_or(hard_warnings);
            let ignore_warnings = config
                .as_ref()
                .map(|c| c.flags.contains(&"ignore_warnings".to_owned()) || ignore_warnings)
                .unwrap_or(ignore_warnings);
            let verbose = config
                .as_ref()
                .map(|c| c.flags.contains(&"verbose".to_owned()) || opt.verbose)
                .unwrap_or(opt.verbose);
            let output = config
                .as_ref()
                .and_then(|c| c.options.get("output"))
                .map(PathBuf::from)
                .or(output)
                .unwrap_or_else(|| source.join("build"));
            let meta = config.as_ref().map(|c| c.meta.clone()).unwrap_or_default();
            let title_actors = config
                .as_ref()
                .and_then(|c| c.options.get("title_actors"))
                .map(|t| t.split(',').map(|s| s.to_owned()).collect())
                .unwrap_or(title_actors);
            let content = PathBuf::from(if be {
                "content"
            } else {
                "01007EF00011E000/romfs"
            });
            builder::Builder {
                be,
                file_times: HashMap::default(),
                meta,
                modified_files: HashSet::new(),
                actorinfo: None,
                hash_table: StockHashTable::new(&if be {
                    Platform::WiiU
                } else {
                    Platform::Switch
                }),
                size_table: Arc::new(Mutex::new({
                    let try_table = output
                        .join(&content)
                        .join("System/Resource/ResourceSizeTable.product.srsizetable");
                    if try_table.exists() {
                        if verbose {
                            println!("{}", "Loading last built RSTB".bright_black());
                        }
                        ResourceSizeTable::from_binary(decompress(fs::read(try_table)?)?)?
                    } else {
                        let try_table = source
                            .join(&content)
                            .join("System/Resource/ResourceSizeTable.product.json");
                        if try_table.exists() {
                            if verbose {
                                println!("{}", "Loading JSON RSTB".bright_black());
                            }
                            ResourceSizeTable::from_text(fs::read_to_string(try_table)?)?
                        } else {
                            if verbose {
                                println!("{}", "Loading fresh RSTB".bright_black());
                            }
                            ResourceSizeTable::new_from_stock(if be {
                                rstb::Endian::Big
                            } else {
                                rstb::Endian::Little
                            })
                        }
                    }
                })),
                content,
                aoc: PathBuf::from(if be {
                    "aoc/0010"
                } else {
                    "01007EF00011F001/romfs"
                }),
                output,
                source,
                title_actors: title_actors
                    .into_iter()
                    .chain(builder::actor::TITLE_ACTORS.iter().map(|t| t.to_string()))
                    .collect(),
                title_events: builder::event::TITLE_EVENTS
                    .iter()
                    .chain(builder::event::NESTED_EVENTS.iter())
                    .map(|t| t.to_string())
                    .collect(),
                compiled: Arc::new(RwLock::new(HashMap::default())),
                verbose,
                warn: if hard_warnings {
                    WarnLevel::Error
                } else if config
                    .as_ref()
                    .map(|c| c.flags.contains(&"ignore_warnings".to_owned()))
                    .unwrap_or(ignore_warnings)
                {
                    WarnLevel::None
                } else {
                    WarnLevel::Warn
                },
            }
            .build()
        }
        Command::Add { project, command } => {
            let be = check_project(&project)?;
            let config = Settings::get_settings()?;
            match command {
                AddCommand::Actor { .. } => command.add_actor(project, config, be)?,
                AddCommand::Actorinfo => command.add_actorinfo(project, config, be)?,
                AddCommand::Map { .. } => command.add_map(project, config, be)?,
                AddCommand::Event { .. } => command.add_event(project, config, be)?,
                AddCommand::Pack { .. } => command.add_pack(project, config, be)?,
            };
            Ok(())
        }
    }
}
