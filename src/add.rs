use crate::{settings::Settings, unbuilder::Unbuilder};
use anyhow::{anyhow, Context, Result};
use join_str::jstr;
use roead::{
    aamp::{hash_name, Parameter, ParameterIO, ParameterListing},
    byml::Byml,
    sarc::Sarc,
    yaz0::decompress,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

pub static AOC_PACKS: &[&str] = &[
    "AocMainField",
    "Dungeon120",
    "Dungeon121",
    "Dungeon122",
    "Dungeon123",
    "Dungeon124",
    "Dungeon125",
    "Dungeon126",
    "Dungeon127",
    "Dungeon128",
    "Dungeon129",
    "Dungeon130",
    "Dungeon131",
    "Dungeon132",
    "Dungeon133",
    "Dungeon134",
    "Dungeon135",
    "Dungeon136",
    "FinalTrial",
    "RemainsElectric",
    "RemainsFire",
    "RemainsWater",
    "RemainsWind",
];

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum AddCommand {
    /// Add an actor to the current project, either modifying a vanilla actor or duplicating it as a new one
    Actor {
        #[structopt(help = "Base actor name")]
        base_actor: String,
        #[structopt(help = "New actor name")]
        new_actor: Option<String>,
        #[structopt(
            long,
            short,
            help = "Add only actor link/info, don't duplicate other files"
        )]
        minimal: bool,
    },
    /// Adds unbuilt actor info to the current project
    Actorinfo,
    /// Add a map unit to the current project
    Map {
        #[structopt(help = "The section of the map unit to add (e.g. `B-2`)")]
        unit: String,
        #[structopt(help = "The map unit type (static or dynamic)")]
        map_type: String,
        #[structopt(short, long, help = "Pull the AOC field (Trial of the Sword) map unit")]
        aocfield: bool,
    },
    /// Add an event to the current project, either modifying a vanilla event or duplicating it as a new one
    Event {
        #[structopt(help = "Base event name")]
        base_event: String,
        #[structopt(help = "New event name")]
        new_event: Option<String>,
        #[structopt(long, short, help = "Add only event info, don't duplicate other files")]
        minimal: bool,
    },
    /// Add a root game pack to the mod (e.g. `Bootup.pack`, `AocMainField.pack`, etc.)
    Pack {
        #[structopt(help = "Pack to add (`.pack` can be omitted)")]
        pack: String,
    },
}

impl AddCommand {
    pub fn add_actor(&self, project: PathBuf, config: Settings, be: bool) -> Result<()> {
        if let AddCommand::Actor {
            base_actor,
            minimal,
            new_actor,
        } = self
        {
            let project = project.join(if be {
                "content"
            } else {
                "01007EF00011E000/romfs"
            });
            let base_pack = if be {
                config.update_dir
            } else {
                config.game_dir_nx
            }
            .context("Game directory not set")?
            .join(jstr!("Actor/Pack/{&base_actor}.sbactorpack"));
            println!("Loading base actor pack...");
            let sarc = Sarc::new(
                fs::read(&base_pack)
                    .with_context(|| format!("Base pack not found at {}", base_pack.display()))?,
            )?;
            let actorlink = Some(jstr!("Actor/ActorLink/{&base_actor}.bxml"));
            println!("Cloning actor files...");
            for file in sarc.files() {
                let (is_yml, out_data) = match &file.data[..4] {
                    b"AAMP" => (true, {
                        let mut pio = ParameterIO::from_binary(file.data)?;
                        if let Some(new_actor) = new_actor {
                            if file.name == actorlink.as_deref() {
                                for link in super::builder::actor::ACTOR_LINKS.keys() {
                                    if let Some(target) = pio
                                        .object_mut(hash_name("LinkTarget"))
                                        .context("Actor link missing LinkTarget")?
                                        .get_mut(*link)
                                    {
                                        if target.as_str()? != "Dummy" {
                                            *target =
                                                Parameter::StringRef(new_actor.as_str().into());
                                        }
                                    }
                                }
                            }
                        }
                        pio.to_text().as_bytes().to_vec()
                    }),
                    b"BY\x00\x02" | b"YB\x02\x00" => (
                        true,
                        Byml::from_binary(file.data)?.to_text().as_bytes().to_vec(),
                    ),
                    _ => (false, file.data.to_vec()),
                };
                let out = if !minimal || file.name == actorlink.as_deref() {
                    let path = Path::new(file.name.unwrap());
                    let ext = path.extension().context("No extension")?.to_str().unwrap();
                    if is_yml {
                        project.join(path.with_file_name(jstr!(
                            "{new_actor.as_ref().unwrap_or(base_actor)}.{ext}.yml"
                        )))
                    } else {
                        project.join(path.with_file_name(jstr!(
                            "{new_actor.as_ref().unwrap_or(base_actor)}.{ext}"
                        )))
                    }
                } else {
                    project.join(file.name.context("No file name")?)
                };
                if !out.exists() {
                    fs::create_dir_all(out.parent().unwrap())?;
                    fs::write(&out, &out_data)?;
                }
            }
            if let Some(new_actor) = new_actor {
                println!("Cloning base actor info...");
                let actorinfo_root = project.join("Actor/ActorInfo");
                if !actorinfo_root.exists() {
                    return Err(anyhow!("Cannot clone actor without actor info in mod"));
                } else {
                    let mut info = Byml::from_text(
                        fs::read_to_string(actorinfo_root.join(jstr!("{&base_actor}.info.yml")))
                            .context("Base actor info not found")?,
                    )?;
                    info["name"] = Byml::String(new_actor.into());
                    fs::write(
                        actorinfo_root.join(jstr!("{&new_actor}.info.yml")),
                        info.to_text(),
                    )?;
                    println!("Successfully cloned {} as {}", base_actor, new_actor);
                }
            } else {
                println!("Successfully added {}", base_actor);
            }
            Ok(())
        } else {
            unreachable!()
        }
    }

    pub fn add_actorinfo(&self, project: PathBuf, config: Settings, be: bool) -> Result<()> {
        let base_path = if be {
            config.update_dir
        } else {
            config.game_dir_nx
        }
        .context("Game directory not set")?
        .join("Actor/ActorInfo.product.sbyml");
        let unbuilder = Unbuilder {
            be,
            output: &project,
            source: PathBuf::new(),
        };
        unbuilder.unbuild_actorinfo(&base_path)?;
        println!("Actor info added to project");
        Ok(())
    }

    pub fn add_map(&self, project: PathBuf, config: Settings, be: bool) -> Result<()> {
        if let Self::Map {
            unit,
            map_type,
            aocfield,
        } = self
        {
            let map_type = match map_type.to_lowercase().as_str() {
                "static" => "Static".to_owned(),
                "dynamic" => "Dynamic".to_owned(),
                _ => return Err(anyhow!("Invalid map unit type")),
            };
            println!("Loading map...");
            let map_path = Path::new("Map")
                .join(if *aocfield { "AocField" } else { "MainField" })
                .join(jstr!("{&unit}/{&unit}_{&map_type}.smubin"));
            let source = if be {
                config.dlc_dir.or(config.update_dir)
            } else {
                config.dlc_dir_nx.or(config.game_dir_nx)
            }
            .context("Game directories not set")?
            .join(&map_path);
            let mubin = Byml::from_binary(decompress(fs::read(source)?)?)?;
            let out = project
                .join(if be {
                    "aoc/0010"
                } else {
                    "01007EF00011F001/romfs"
                })
                .join(&map_path)
                .with_extension("smubin.yml");
            fs::create_dir_all(out.parent().unwrap())?;
            fs::write(out, mubin.to_text())?;
            println!("Map {} {} added", &unit, map_type);
        };
        Ok(())
    }

    pub fn add_event(&self, project: PathBuf, config: Settings, be: bool) -> Result<()> {
        if let Self::Event {
            base_event,
            new_event,
            minimal,
        } = self
        {
            let project = project.join(if be {
                "content"
            } else {
                "01007EF00011E000/romfs"
            });
            let base_pack = if be {
                config.update_dir
            } else {
                config.game_dir_nx
            }
            .context("Game directory not set")?
            .join(jstr!("Event/{&base_event}.sbeventpack"));
            println!("Loading base event pack...");
            let sarc = Sarc::new(decompress(fs::read(&base_pack).with_context(|| {
                format!("Base pack not found at {}", base_pack.display())
            })?)?)?;
            for (file, data) in sarc
                .files()
                .filter_map(|file| file.name.map(|n| (n, file.data)))
            {
                let (is_yml, out_data) = match &data[..4] {
                    b"AAMP" => (true, {
                        ParameterIO::from_binary(data)?
                            .to_text()
                            .as_bytes()
                            .to_vec()
                    }),
                    b"BY\x00\x02" | b"YB\x02\x00" => {
                        (true, Byml::from_binary(data)?.to_text().as_bytes().to_vec())
                    }
                    _ => (false, data.to_vec()),
                };
                let file: PathBuf = if new_event.is_some() && !minimal && file.contains(base_event)
                {
                    file.replace(base_event, new_event.as_ref().unwrap()).into()
                } else {
                    file.into()
                };
                let ext = file.extension().unwrap().to_str().unwrap();
                let out = project.join(if is_yml {
                    file.with_extension(jstr!("{ext}.yml"))
                } else {
                    file
                });
                if !out.exists() {
                    fs::create_dir_all(out.parent().unwrap())?;
                    fs::write(&out, &out_data)?;
                }
            }
            if let Some(new_event) = new_event {
                println!("Cloning base event info...");
                let eventinfo_root = project.join("Event/EventInfo");
                if !eventinfo_root.exists() {
                    return Err(anyhow!("Cannot clone event without event info in mod"));
                } else {
                    let mut info = Byml::from_text(
                        fs::read_to_string(eventinfo_root.join(jstr!("{&base_event}.info.yml")))
                            .context("Base event info not found")?,
                    )?;
                    if !minimal {
                        fn update_names(node: &mut Byml, base_event: &str, new_event: &str) {
                            match node {
                                Byml::Array(array) => {
                                    for child in array.iter_mut() {
                                        update_names(child, base_event, new_event);
                                    }
                                }
                                Byml::Map(hash) => {
                                    for child in hash.values_mut() {
                                        update_names(child, base_event, new_event);
                                    }
                                }
                                Byml::String(old_value) => {
                                    if old_value.contains(base_event) {
                                        *node = new_event.into();
                                    }
                                }
                                _ => (),
                            };
                        }
                        update_names(&mut info, base_event, new_event);
                    }
                    fs::write(
                        eventinfo_root.join(jstr!("{&new_event}.info.yml")),
                        info.to_text(),
                    )?;
                    println!("Successfully cloned {} as {}", base_event, new_event);
                }
            } else {
                println!("Successfully added {}", base_event);
            }
        };
        Ok(())
    }

    pub fn add_pack(&self, project: PathBuf, config: Settings, be: bool) -> Result<()> {
        if let Self::Pack { pack } = self {
            let pack = pack.trim_end_matches(".pack");
            let rel_path = jstr!("Pack/{pack}.pack");
            let aoc_pack = AOC_PACKS.contains(&pack);
            let base_path = match (aoc_pack, be) {
                (true, true) => config.dlc_dir,
                (true, false) => config.dlc_dir_nx,
                (false, true) => config.update_dir,
                (false, false) => config.game_dir_nx,
            }
            .context("Game directory not set")?
            .join(&rel_path);
            let sarc = Sarc::new(fs::read(base_path)?)?;
            let unbuilder = Unbuilder {
                be,
                output: &project,
                source: PathBuf::new(),
            };
            unbuilder.unbuild_sarc(
                sarc,
                Some(
                    &project
                        .join(match (aoc_pack, be) {
                            (true, true) => "aoc/0010",
                            (true, false) => "01007EF00011F001/romfs",
                            (false, true) => "content",
                            (false, false) => "01007EF00011E000/romfs",
                        })
                        .join(&rel_path),
                ),
            )?;
            println!("{}.pack added to project", &pack);
        };
        Ok(())
    }
}
