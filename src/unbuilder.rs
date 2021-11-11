use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use super::util::*;
use anyhow::{anyhow, format_err, Context, Result};
use jstr::jstr;
use rayon::prelude::*;
use roead::{sarc::Sarc, *};

static BLANK_META: &[u8] = b"\
Meta:
    name: New Mod
    description: A new mod project
Flags: []
Options: {}
";

static HANDLED: &[&str] = &[
    "ResourceSizeTable.product.srsizetable",
    "ActorInfo.product.sbyml",
    "EventInfo.product.sbyml",
    "QuestProduct.sbquestpack",
    "rules.txt",
    "info.json",
];

static ROOT_PACKS: &[&str] = &["sbactorpack", "sbeventpack"];

static EXCLUDE_UNPACK: &[&str] = &[
    "tera_resource.Cafe_Cafe_GX2.release.ssarc",
    "tera_resource.Nin_NX_NVN.release.ssarc",
];

static EXCLUDE_UNPACK_EXTS: &[&str] = &["baatarc", "sgenvb", "ssarc", "sblarc", "sfarc"];

#[derive(Debug)]
struct Unbuilder<'a> {
    be: bool,
    output: &'a Path,
    source: PathBuf,
}

#[inline]
fn unbuild_aamp(data: &[u8], out: &Path) -> Result<()> {
    fs::write(
        out,
        aamp::ParameterIO::from_binary(&yaz0::decompress_if(data)?)?.to_text(),
    )?;
    Ok(())
}

#[inline]
fn unbuild_byml(data: &[u8], out: &Path) -> Result<()> {
    fs::write(
        out,
        byml::Byml::from_binary(&yaz0::decompress_if(data)?)?.to_text(),
    )?;
    Ok(())
}

impl Unbuilder<'_> {
    #[inline]
    fn content(&self) -> &str {
        if self.be {
            "content"
        } else {
            "01007EF00011E000/romfs"
        }
    }

    #[inline]
    fn aoc(&self) -> &str {
        if self.be {
            "aoc/0010"
        } else {
            "01007EF00011F001/romfs"
        }
    }

    #[inline]
    fn out_content(&self) -> PathBuf {
        self.output.join(self.content())
    }

    fn unbuild(self) -> Result<()> {
        if !validate_source(&self.source) {
            return Err(anyhow!("Source folder is not in a supported mod format"));
        }
        // let files: Vec<PathBuf> = glob::glob(self.source.join("**/*").to_str().unwrap())
        //     .unwrap()
        //     .filter_map(|f| f.ok())
        //     .filter(|f| f.is_file())
        //     .collect();
        println!("Unbuilding processed files...");
        for dir in PROCESSED_DIRS {
            glob::glob(
                &self
                    .source
                    .join(jstr!("{self.content()}/{dir}/**/*.*"))
                    .to_string_lossy(),
            )?
            .chain(glob::glob(
                &self
                    .source
                    .join(jstr!("{self.aoc()}/{dir}/**/*.*"))
                    .to_string_lossy(),
            )?)
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .into_par_iter()
            .try_for_each(|f| -> Result<()> {
                self.unbuild_file(&f)?;
                Ok(())
            })?;
        }
        println!("Unbuilding general files...");
        for dir in UNPROCESSED_DIRS {
            glob::glob(
                &self
                    .source
                    .join(jstr!("{self.content()}/{dir}/**/*.*"))
                    .to_string_lossy(),
            )?
            .chain(glob::glob(
                &self
                    .source
                    .join(jstr!("{self.aoc()}/{dir}/**/*.*"))
                    .to_string_lossy(),
            )?)
            .filter_map(Result::ok)
            .collect::<Vec<_>>()
            .into_par_iter()
            .try_for_each(|f| -> Result<()> {
                let out = self.output.join(f.strip_prefix(&self.source)?);
                fs::create_dir_all(out.parent().context("No parent???")?)?;
                fs::copy(&f, &out)?;
                Ok(())
            })?;
        }
        let actorinfo = self
            .source
            .join(if self.be {
                "content"
            } else {
                "01007EF00011E000/romfs"
            })
            .join("Actor/ActorInfo.product.sbyml");
        if actorinfo.exists() {
            self.unbuild_actorinfo(&actorinfo)?;
        }
        let actor_pack_dir = self.out_content().join("Actor/Pack");
        if actor_pack_dir.exists() {
            fs::remove_dir_all(actor_pack_dir)?;
        }
        Ok(())
    }

    fn unbuild_file(&self, file: &Path) -> Result<()> {
        let file_name = file.file_name().unwrap().to_str().unwrap();
        let rel = file.strip_prefix(&self.source)?.to_owned();
        if HANDLED.contains(&file_name) {
            return Ok(());
        }
        let out = self.output.join(&rel);
        if !out.parent().unwrap().exists() {
            fs::create_dir_all(out.parent().unwrap())?;
        }
        let ext = match get_ext(&rel) {
            Ok(e) => e,
            Err(_) => {
                fs::copy(&file, &out)?;
                return Ok(());
            }
        };
        let data = fs::read(file)?;
        if AAMP_EXTS.contains(&ext) {
            unbuild_aamp(&data, &out.with_extension(jstr!("{ext}.yml")))?;
        } else if BYML_EXTS.contains(&ext) {
            unbuild_byml(&data, &out.with_extension(jstr!("{ext}.yml")))?;
        } else if botw_utils::extensions::SARC_EXTS.contains(&ext) && !data.is_empty() {
            let sarc = Sarc::read(&data)?;
            if file_name.starts_with("Bootup_") && file_name.len() == 16 {
                // if self.no_msyt {
                //     drop(sarc);
                //     fs::write(out, data)?;
                // } else {
                self.unbuild_text(sarc)?;
                // }
            } else {
                self.unbuild_sarc(
                    sarc,
                    if ROOT_PACKS.contains(&ext) {
                        None
                    } else {
                        Some(&out)
                    },
                )?;
            }
        } else {
            fs::write(out, data)?;
        }
        Ok(())
    }

    fn unbuild_actorinfo(&self, file: &Path) -> Result<()> {
        println!("Unbuilding actor info...");
        let actorinfo = byml::Byml::from_binary(&yaz0::decompress_if(fs::read(file)?)?)?;
        fs::create_dir_all(self.out_content().join("Actor/ActorInfo"))?;
        actorinfo
            .as_hash()?
            .get("Actors")
            .context("Invalid actor info file")?
            .as_array()?
            .into_par_iter()
            .try_for_each(|a| -> Result<()> {
                let actor = a.as_hash()?;
                fs::write(
                    self.out_content()
                        .join("Actor/ActorInfo")
                        .join(jstr!(r#"{actor["name"].as_string()?}.info.yml"#)),
                    byml::Byml::Hash(actor.clone()).to_text(),
                )?;
                Ok(())
            })?;
        Ok(())
    }

    fn unbuild_eventinfo(&self, data: &[u8]) -> Result<()> {
        println!("Unbuilding event info...");
        let eventinfo = byml::Byml::from_binary(&yaz0::decompress(data)?)?;
        let eventinfo = eventinfo.as_hash()?;
        fs::create_dir_all(self.out_content().join("Event/EventInfo"))?;
        let mut events: BTreeMap<String, BTreeMap<String, byml::Byml>> = BTreeMap::new();
        for (name, event) in eventinfo {
            let base_event = name.split('<').next().unwrap();
            let sub_event = &name[name.find('<').unwrap() + 1..name.find('>').unwrap()];
            if !events.contains_key(base_event) {
                events.insert(base_event.to_owned(), BTreeMap::new());
            }
            events
                .get_mut(base_event)
                .unwrap()
                .insert(sub_event.to_owned(), event.clone());
        }
        events
            .into_par_iter()
            .try_for_each(|(name, data)| -> Result<()> {
                fs::write(
                    self.out_content()
                        .join("Event/EventInfo")
                        .join(name)
                        .with_extension("info.yml"),
                    byml::Byml::Hash(data).to_text(),
                )?;
                Ok(())
            })?;
        Ok(())
    }

    // fn unbuild_questpack(&self, data: &[u8]) -> Result<()> {
    //     println!("Unbuilding quest info...");
    //     let mut questpack = byml::Byml::from_binary(&yaz0::decompress(data)?)?;
    //     fs::create_dir_all(self.content().join("Quest"))?;
    //     questpack
    //         .as_mut_array()?
    //         .into_par_iter()
    //         .try_for_each(|q| -> Result<()> {
    //             let quest = q.as_mut_hash()?;
    //             let name = jstr!(r#"{quest.remove("Name").unwrap().as_string()?}.info.yml"#);
    //             fs::write(self.content().join("Quest").join(name), q.to_text())?;
    //             Ok(())
    //         })?;
    //     Ok(())
    // }

    fn unbuild_sarc(&self, sarc: Sarc, output: Option<&Path>) -> Result<()> {
        let output = output
            .map(|o| o.to_owned())
            .unwrap_or_else(|| self.out_content());
        if !output.exists() {
            fs::create_dir_all(&output)?;
        }
        if sarc
            .files()
            .any(|f| f.name().unwrap_or("").starts_with('/'))
        {
            fs::write(output.join(".slash"), b"")?;
        }
        if sarc.guess_min_alignment() != 4 {
            fs::write(
                output.join(".align"),
                format!("{}", sarc.guess_min_alignment()),
            )?;
        }
        for file in sarc.files().filter(|f| f.name().is_some()) {
            let name = file.name().unwrap().trim_start_matches('/');
            let out = output.join(name);
            fs::create_dir_all(out.parent().unwrap())?;
            if let Some(ext) = name.split('.').last() {
                if &file.data()[0..4] == b"AAMP" {
                    let out = out.with_extension(jstr!("{ext}.yml"));
                    if !out.exists() {
                        unbuild_aamp(file.data(), &out)?;
                    }
                } else if BYML_EXTS.contains(&ext) {
                    if name.ends_with("EventInfo.product.sbyml") {
                        self.unbuild_eventinfo(file.data())?;
                    // } else if name.ends_with("questpack") {
                    //     self.unbuild_questpack(file.data())?;
                    } else {
                        let out = out.with_extension(jstr!("{ext}.yml"));
                        unbuild_byml(file.data(), &out)?;
                    }
                } else if file.data().len() > 0x15
                    && (&file.data()[0..4] == b"SARC" || &file.data()[0x11..0x15] == b"SARC")
                    && !EXCLUDE_UNPACK.contains(&name)
                    && !EXCLUDE_UNPACK_EXTS.contains(&ext)
                {
                    let subsarc = Sarc::read(file.data())?;
                    self.unbuild_sarc(
                        subsarc,
                        if output.file_name().unwrap().to_str().unwrap() == "TitleBG.pack"
                            && ext == "sbactorpack"
                        {
                            None
                        } else {
                            Some(&out)
                        },
                    )?;
                } else {
                    fs::write(out, file.data())?;
                }
            } else {
                fs::write(out, file.data())?;
            }
        }
        Ok(())
    }

    fn unbuild_text(&self, sarc: Sarc) -> Result<()> {
        let msg_pack = sarc
            .files()
            .find(|f| f.name().is_some() && f.name().unwrap().contains(".ssarc"))
            .context("{} is missing a message SARC")?;
        let lang = &msg_pack.name().unwrap()[0xC..0x10];
        println!("Unbuilding {} texts...", lang);
        let msg_sarc = Sarc::read(msg_pack.data())?;
        (0..msg_sarc.len())
            .into_par_iter()
            .try_for_each(|i| -> Result<()> {
                let file = match msg_sarc.get_file_by_index(i) {
                    Some(f) => f,
                    None => return Ok(()),
                };
                let out = self
                    .out_content()
                    .join(&jstr!("Message/{lang}/{file.name().unwrap()}"))
                    .with_extension("msyt");
                if !out.parent().unwrap().exists() {
                    fs::create_dir_all(out.parent().unwrap())?;
                }
                fs::write(
                    out,
                    serde_yaml::to_string(
                        &msyt::Msyt::from_msbt_bytes(file.data())
                            .map_err(|e| format_err!("{}", e))?,
                    )?,
                )?;
                Ok(())
            })?;
        Ok(())
    }
}

pub(crate) fn unbuild(
    be: bool,
    source: Option<PathBuf>,
    directory: Option<PathBuf>,
    config: bool,
) -> Result<()> {
    println!("Initializing mod project...");
    let output = directory.unwrap_or_else(|| PathBuf::from("."));
    if output.exists() {
        fs::remove_dir_all(&output)?;
    }
    fs::create_dir_all(&output)?;
    if config {
        fs::write(output.join("config.yml"), BLANK_META)?;
    }
    if let Some(source) = source {
        println!("Unbuilding source files...");
        Unbuilder {
            be,
            output: &output,
            source,
        }
        .unbuild()?;
    } else {
        fs::create_dir_all(output.join(if be {
            "content"
        } else {
            "01007EF00011E000/romfs"
        }))?;
    }
    fs::write(output.join(".db"), b"")?;
    println!("Done");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn unbuild_u() {
        super::unbuild(
            true,
            Some("test/source".into()),
            Some("test/project".into()),
            true,
        )
        .unwrap();
    }

    #[test]
    fn unbuild_nx() {
        super::unbuild(
            false,
            Some("test/source_nx".into()),
            Some("test/project_nx".into()),
            true,
        )
        .unwrap();
    }
}
