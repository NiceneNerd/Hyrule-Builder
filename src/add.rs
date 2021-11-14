use crate::{settings::Settings, unbuilder::Unbuilder};
use anyhow::{anyhow, Context, Result};
use jstr::jstr;
use roead::{
    aamp::{hash_name, Parameter, ParameterIO},
    byml::Byml,
    sarc::Sarc,
    yaz0::decompress,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub enum AddCommand {
    /// Add an actor to the project, either modifying a vanilla actor or duplicating it as a new one
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
    /// Adds unbuilt actor info to this project
    Actorinfo,
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
            let sarc = Sarc::read(decompress(fs::read(&base_pack).with_context(|| {
                format!("Base pack not found at {}", base_pack.display())
            })?)?)?;
            let actorlink = Some(jstr!("Actor/ActorLink/{&base_actor}.bxml"));
            println!("Cloning actor files...");
            for (file, data) in sarc.into_files().into_iter() {
                let (is_yml, out_data) = match &data[..4] {
                    b"AAMP" => (true, {
                        let mut pio = ParameterIO::from_binary(&data)?;
                        if let Some(new_actor) = new_actor {
                            if file == actorlink {
                                for link in super::builder::actor::ACTOR_LINKS.keys() {
                                    if let Some(target) = pio
                                        .objects
                                        .get_mut(hash_name("LinkTarget"))
                                        .context("Actor link missing LinkTarget")?
                                        .0
                                        .get_mut(link)
                                    {
                                        if target.as_string()? != "Dummy" {
                                            *target = Parameter::StringRef(new_actor.clone());
                                        }
                                    }
                                }
                            }
                        }
                        pio.to_text().as_bytes().to_vec()
                    }),
                    b"BY\x00\x02" | b"YB\x02\x00" => (
                        true,
                        Byml::from_binary(&data)?.to_text().as_bytes().to_vec(),
                    ),
                    _ => (false, data),
                };
                let out = if !minimal || file == actorlink {
                    let path = Path::new(file.as_ref().unwrap());
                    let ext = path.extension().context("No extension")?.to_str().unwrap();
                    if is_yml {
                        project.join(path.with_file_name(&jstr!(
                            "{new_actor.as_ref().unwrap_or(base_actor)}.{ext}.yml"
                        )))
                    } else {
                        project.join(path.with_file_name(&jstr!(
                            "{new_actor.as_ref().unwrap_or(base_actor)}.{ext}"
                        )))
                    }
                } else {
                    project.join(file.context("No file name")?)
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
                        &fs::read_to_string(actorinfo_root.join(&jstr!("{&base_actor}.info.yml")))
                            .context("Base actor info not found")?,
                    )?;
                    info["name"] = Byml::String(new_actor.clone());
                    fs::write(
                        actorinfo_root.join(&jstr!("{&new_actor}.info.yml")),
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
}
