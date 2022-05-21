pub mod actor;
pub mod config;
pub mod event;

use super::util::*;
use crate::{
    builder::{actor::Actor, event::Event},
    unzip_some::unzip_some,
};
use anyhow::{anyhow, format_err, Context, Result};
use botw_utils::{get_canon_name, get_canon_name_without_root, hashes::StockHashTable};
use colored::*;
use join_str::jstr;
use path_slash::{PathBufExt, PathExt};
use rayon::prelude::*;
use roead::{
    aamp::ParameterIO,
    byml::Byml,
    sarc::{Sarc, SarcWriter},
    yaz0::compress,
    Endian,
};
use rstb::ResourceSizeTable;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

pub type Hash = BTreeMap<String, Byml>;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BuildConfig {
    pub meta: HashMap<String, String>,
    pub flags: Vec<String>,
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WarnLevel {
    None,
    Warn,
    Error,
}

#[derive(Debug)]
pub struct Builder {
    pub be: bool,
    pub source: PathBuf,
    pub output: PathBuf,
    pub content: PathBuf,
    pub aoc: PathBuf,
    pub file_times: HashMap<PathBuf, u64>,
    pub modified_files: HashSet<PathBuf>,
    pub hash_table: StockHashTable,
    pub compiled: Arc<Mutex<HashMap<PathBuf, Vec<u8>>>>,
    pub size_table: Arc<Mutex<ResourceSizeTable>>,
    pub title_actors: HashSet<String>,
    pub title_events: HashSet<String>,
    pub actorinfo: Option<Hash>,
    pub meta: HashMap<String, String>,
    pub warn: WarnLevel,
    pub verbose: bool,
}

impl Builder {
    #[inline]
    fn endian(&self) -> Endian {
        if self.be {
            Endian::Big
        } else {
            Endian::Little
        }
    }

    #[inline]
    fn vprint(&self, message: &str) {
        if self.verbose {
            println!("{}", message.bright_black());
        }
    }

    fn warn(&self, message: &str) -> Result<()> {
        match self.warn {
            WarnLevel::None => Ok(()),
            WarnLevel::Warn => {
                println!("{}", message.yellow());
                Ok(())
            }
            WarnLevel::Error => Err(format_err!("{}", message.red())),
        }
    }

    #[inline(always)]
    fn source_content(&self) -> PathBuf {
        self.source.join(&self.content)
    }

    #[inline(always)]
    fn out_content(&self) -> PathBuf {
        self.output.join(&self.content)
    }

    fn get_canon_name(&self, file: &Path) -> Option<String> {
        if let Some(sarc_root) = file
            .ancestors()
            .skip(1)
            .find(|p| SARC_EXTS.contains(&p.extension()))
        {
            Some(get_canon_name_without_root(
                file.strip_prefix(sarc_root).unwrap(),
            ))
        } else if let Ok(source_rel) = file.strip_prefix(&self.source) {
            get_canon_name(source_rel)
        } else if let Ok(out_rel) = file.strip_prefix(&self.output) {
            get_canon_name(out_rel)
        } else {
            unreachable!()
        }
    }

    fn get_resource_data(&self, file: &Path) -> Result<Vec<u8>> {
        let mut compiled = self.compiled.lock().unwrap();
        if let Some(data) = compiled.get(file) {
            Ok(data.clone())
        } else {
            let bytes = std::fs::read(&file)
                .with_context(|| jstr!("Failed to read {file.to_str().unwrap()}"))?;
            let mut ext = get_ext(file)?;
            let data = if ext == "yml" {
                let text = std::str::from_utf8(&bytes)?;
                if text.len() >= 3 && &text[0..3] == "!io" {
                    ParameterIO::from_text(text)
                        .with_context(|| {
                            jstr!("Failed to parse AAMP file {file.to_str().unwrap()}")
                        })?
                        .to_binary()
                } else {
                    Byml::from_text(text)
                        .with_context(|| {
                            jstr!("Failed to parse BYML file {file.to_str().unwrap()}")
                        })?
                        .to_binary(self.endian())
                }
            } else {
                bytes
            };
            if ext == "yml" {
                ext = file
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .context("Uh oh")?
                    .rsplit('.')
                    .next()
                    .context("Uh oh")?;
            }
            if !EXCLUDE_RSTB.contains(&ext) {
                let mut size_table = self.size_table.lock().unwrap();
                if let Some(canon) = self.get_canon_name(Path::new(
                    file.to_str()
                        .context("Funky filename")?
                        .trim_end_matches(".yml"),
                )) {
                    if self.hash_table.is_file_modded(&canon, &data, true) {
                        if let Some(size) = rstb::calc::estimate_from_slice_and_name(
                            &data,
                            &canon,
                            if self.be {
                                rstb::Endian::Big
                            } else {
                                rstb::Endian::Little
                            },
                        ) {
                            if size > size_table.get(canon.as_str()).unwrap_or(0) {
                                size_table.set(canon.as_str(), size);
                            };
                        } else {
                            size_table.remove(canon.as_str());
                        }
                    }
                }
            };
            let data = if &data[0..4] != b"Yaz0" && ext.starts_with('s') && ext != "sarc" {
                compress(data)
            } else {
                data
            };
            compiled.insert(file.to_owned(), data.to_vec());
            Ok(data)
        }
    }

    fn set_resource_size(&self, entry: &str, data: &[u8]) {
        if !self.hash_table.is_file_modded(entry, data, true) {
            return;
        }
        let mut size_table = self.size_table.lock().unwrap();
        if let Some(size) = rstb::calc::estimate_from_slice_and_name(
            data,
            entry,
            if self.be {
                rstb::Endian::Big
            } else {
                rstb::Endian::Little
            },
        ) {
            if size > size_table.get(entry).unwrap_or(0) {
                size_table.set(entry, size);
            }
        } else {
            size_table.remove(entry);
        }
    }

    fn load_modified_files(&mut self) -> Result<()> {
        println!("Scanning project files");
        let db = self.source.join(".db");
        if db.exists() && fs::metadata(&db)?.len() > 1 {
            self.file_times.extend(
                fs::read_to_string(db)?
                    .lines()
                    .filter(|x| !x.is_empty())
                    .map(|l| -> Result<(PathBuf, u64)> {
                        let mut data = l.split(',');
                        Ok((
                            self.source.join(data.next().context("Invalid DB")?),
                            str::parse::<u64>(data.next().context("Invalid DB")?)?,
                        ))
                    })
                    .collect::<Result<Vec<(PathBuf, u64)>>>()?,
            );
        }
        self.modified_files = glob::glob(self.source.join("**/*").to_str().context("Bad glob")?)?
            .filter_map(Result::ok)
            .filter(|f| f.is_file() && f.file_name() != Some(OsStr::new(".db")))
            .filter(|file| {
                !self.file_times.contains_key(file) || {
                    fs::metadata(file)
                        .unwrap()
                        .modified()
                        .unwrap()
                        .duration_since(
                            std::time::UNIX_EPOCH
                                .checked_add(std::time::Duration::from_secs(
                                    *self.file_times.get(file).unwrap(),
                                ))
                                .unwrap(),
                        )
                        .is_ok()
                }
            })
            .collect();
        Ok(())
    }

    fn load_actorinfo(&mut self) -> Result<()> {
        println!("Loading actor info");
        self.actorinfo = Some(
            glob::glob(
                self.source_content()
                    .join("Actor/ActorInfo/**/*.info.yml")
                    .as_os_str()
                    .to_str()
                    .unwrap(),
            )?
            .filter_map(Result::ok)
            .try_fold(BTreeMap::new(), |mut actorinfo, file| -> Result<Hash> {
                actorinfo.insert(
                    file.file_stem()
                        .context("Whoa, no filename")?
                        .to_string_lossy()
                        .trim_end_matches(".info")
                        .into(),
                    Byml::from_text(fs::read_to_string(&file)?)?,
                );
                Ok(actorinfo)
            })?,
        );
        Ok(())
    }

    // TODO: Make use of these
    // fn update_actorinfo(&mut self, info: Hash) -> Result<()> {
    //     if let Some(actorinfo) = self.actorinfo.as_mut() {
    //         actorinfo.extend(info);
    //     } else {
    //         let actorinfo_path = self.source_content().join("Actor/ActorInfo");
    //         if actorinfo_path.exists() {
    //             self.load_actorinfo()?;
    //             self.actorinfo.as_mut().unwrap().extend(info);
    //         } else {
    //             self.warn("Actor info should be updated, but Actor/ActorInfo is missing")?;
    //         }
    //     }
    //     Ok(())
    // }

    fn build_actors(&mut self) -> Result<()> {
        let actor_root = self.source_content().join("Actor");
        if self
            .modified_files
            .par_iter()
            .any(|p| p.starts_with(&actor_root))
        {
            println!("Checking actor packs");
            let modded_actors: Vec<Actor> =
                glob::glob(actor_root.join("ActorLink/*.bxml.yml").to_str().unwrap())?
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>()
                    .into_par_iter()
                    .map(|f| Actor::new(self, &f))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect();
            if !modded_actors.is_empty() {
                let output_pack_dir = self.out_content().join("Actor/Pack");
                std::fs::create_dir_all(&output_pack_dir)?;
                println!("Building {} actor packs", modded_actors.len());
                let (title_actors, free_actors): (Vec<_>, Vec<_>) = modded_actors
                    .into_par_iter()
                    .partition(|a| self.title_actors.contains(&a.name));
                self.vprint(&format!("  {} normal actor packs", free_actors.len()));
                free_actors
                    .into_par_iter()
                    .try_for_each(|a| -> Result<()> {
                        std::fs::write(
                            output_pack_dir.join(&jstr!("{&a.name}.sbactorpack")),
                            a.build()?,
                        )?;
                        Ok(())
                    })?;
                self.vprint(&format!("  {} TitleBG actor packs", title_actors.len()));
                let built_title_actors = title_actors
                    .into_par_iter()
                    .map(|a| -> Result<(PathBuf, Vec<u8>)> {
                        Ok((
                            jstr!("TitleBG.pack/Actor/Pack/{&a.name}.sbactorpack").into(),
                            a.build()?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?;
                self.compiled.lock().unwrap().par_extend(built_title_actors);
            }
        }
        Ok(())
    }

    fn build_actorinfo(&mut self) -> Result<()> {
        if let Some(actorinfo) = self.actorinfo.take() {
            println!("Building actor info");
            let mut info = Hash::new();
            info.insert(
                "Hashes".to_owned(),
                Byml::Array({
                    let mut hashes: Vec<_> = actorinfo.keys().map(|n| hash_name(n)).collect();
                    hashes.sort_unstable();
                    hashes
                        .into_par_iter()
                        .map(|hash| {
                            if hash < 2147483648 {
                                Byml::Int(hash as i32)
                            } else {
                                Byml::UInt(hash)
                            }
                        })
                        .collect()
                }),
            );
            info.insert(
                "Actors".to_owned(),
                Byml::Array({
                    let mut actors: Vec<_> = actorinfo.into_par_iter().map(|(_, a)| a).collect();
                    actors.sort_unstable_by_key(|a| {
                        hash_name(a.as_hash().unwrap()["name"].as_string().unwrap())
                    });
                    actors
                }),
            );
            fs::write(
                self.out_content().join("Actor/ActorInfo.product.sbyml"),
                compress(Byml::Hash(info).to_binary(self.endian())),
            )?;
        }
        Ok(())
    }

    fn build_events(&mut self) -> Result<()> {
        let event_root = self.source_content().join("Event");
        let event_info_root = event_root.join("EventInfo");
        if event_root.exists() {
            println!("Checking events");
            let title_event_path = self.source_content().join("Pack/TitleBG.pack/EventFlow");
            if title_event_path.exists() {
                self.title_events.extend(
                    glob::glob(title_event_path.join("*.bfevfl").to_str().unwrap())?
                        .flat_map(Result::ok)
                        .map(|f| f.file_stem().unwrap().to_string_lossy().into()),
                )
            };
            let (event_info, event_packs): (Hash, Vec<Event>) = unzip_some(
                glob::glob(event_info_root.join("*.info.yml").to_str().unwrap())?
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>()
                    .into_par_iter()
                    .map(
                        |file| -> Result<(
                            std::collections::btree_map::IntoIter<String, Byml>,
                            Option<Event>,
                        )> { Event::new(self, &file) },
                    )
                    .collect::<Result<Vec<_>>>()?
                    .into_par_iter()
                    .map(|(i, e)| (Some(i), e)),
            );
            if self
                .modified_files
                .par_iter()
                .any(|p| p.starts_with(&event_info_root))
            {
                println!("Building event info");
                let data = Byml::Hash(event_info).to_binary(self.endian());
                self.set_resource_size("Event/EventInfo.product.byml", &data);
                self.compiled
                    .lock()
                    .unwrap()
                    .insert("Event/EventInfo.product.sbyml".into(), compress(data));
            }
            if !event_packs.is_empty() {
                let output_pack_dir = self.out_content().join("Event");
                std::fs::create_dir_all(&output_pack_dir)?;
                println!("Building {} event packs", event_packs.len());
                event_packs
                    .into_par_iter()
                    .try_for_each(|e| -> Result<()> {
                        std::fs::write(
                            output_pack_dir.join(&jstr!("{&e.name}.sbeventpack")),
                            e.build()?,
                        )?;
                        Ok(())
                    })?;
            }
        }
        Ok(())
    }

    fn build_texts(&self) -> Result<()> {
        let message_root = self.source_content().join("Message");
        if self
            .modified_files
            .par_iter()
            .any(|f| f.starts_with(&message_root))
        {
            let pack_out = self.out_content().join("Pack");
            fs::create_dir_all(&pack_out)?;
            for dir in fs::read_dir(&message_root)?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|e| e.is_dir())
            {
                let lang = dir
                    .file_name()
                    .context("Weird")?
                    .to_str()
                    .context("Weird")?;
                println!("Building {} texts", lang);
                let message_sarc = Arc::new(Mutex::new(SarcWriter::new(self.endian())));
                let endian = if self.be {
                    msyt::Endianness::Big
                } else {
                    msyt::Endianness::Little
                };
                glob::glob(dir.join("**/*.msyt").to_str().unwrap())?
                    .filter_map(Result::ok)
                    .collect::<Vec<_>>()
                    .into_par_iter()
                    .try_for_each(|f| -> Result<()> {
                        let text = fs::read_to_string(&f)?;
                        let msyt: msyt::Msyt = serde_yaml::from_str(&text)?;
                        message_sarc.lock().unwrap().add_file(
                            &f.strip_prefix(&dir)?
                                .with_extension("msbt")
                                .to_slash_lossy(),
                            msyt.into_msbt_bytes(endian)
                                .map_err(|e| anyhow::anyhow!(e))?,
                        );
                        Ok(())
                    })?;
                let message_bytes = Arc::try_unwrap(message_sarc)
                    .unwrap()
                    .into_inner()?
                    .to_binary();
                let message_path = jstr!("Message/Msg_{lang}.product.ssarc");
                self.set_resource_size(&message_path.replace(".ss", ".s"), &message_bytes);
                let mut bootup_sarc = SarcWriter::new(self.endian());
                bootup_sarc.add_file(&message_path, compress(message_bytes));
                fs::write(
                    pack_out.join(&jstr!("Bootup_{lang}.pack")),
                    bootup_sarc.to_binary(),
                )?;
            }
        }
        Ok(())
    }

    fn build_sarc(&self, sarc_path: &Path, sarc: &mut SarcWriter) -> Result<Vec<u8>> {
        let prefix = if sarc_path.join(".slash").exists() {
            "/"
        } else {
            ""
        };
        let align_path = sarc_path.join(".align");
        if align_path.exists() {
            sarc.set_alignment(fs::read_to_string(align_path)?.parse::<u8>()?);
        }
        if sarc_path.file_name() == Some(std::ffi::OsStr::new("TitleBG.pack")) {
            for (path, data) in self
                .compiled
                .lock()
                .unwrap()
                .iter()
                .filter_map(|(path, data)| {
                    path.strip_prefix("TitleBG.pack").ok().map(|p| (p, data))
                })
            {
                sarc.add_file(path.to_str().unwrap(), data.clone());
            }
        } else if sarc_path.file_name() == Some(std::ffi::OsStr::new("Bootup.pack")) {
            if let Ok(data) = self.get_resource_data(Path::new("Event/EventInfo.product.sbyml")) {
                sarc.add_file("Event/EventInfo.product.sbyml", data);
            } else if !sarc.contains("Event/EventInfo.product.sbyml") {
                anyhow::bail!("No event info???")
            }
        };
        self.modified_files
            .iter()
            .filter_map(|f| f.strip_prefix(&sarc_path).ok())
            .filter(|f| {
                f.ancestors()
                    .skip(1)
                    .all(|s| !SARC_EXTS.contains(&s.extension()))
                    && !f
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with('.'))
                        .unwrap_or(true)
            })
            .map(|f| sarc_path.join(f))
            .chain(
                glob::glob(&sarc_path.join("**/*.*").to_string_lossy())?
                    .filter_map(Result::ok)
                    .filter(|f| f.is_dir()),
            )
            .try_for_each(|f| -> Result<()> {
                let add_path = jstr!(r#"{prefix}{&f.strip_prefix(&sarc_path)?.to_slash_lossy().trim_end_matches(".yml")}"#);
                let data = if f.is_dir() && SARC_EXTS.contains(&f.extension()) {
                    let mut sarc_writer = if sarc.contains(&*add_path) {
                        SarcWriter::from(Sarc::read(
                            sarc.get_file_data(&add_path).context("Uh??")?,
                        )?)
                    } else {
                        SarcWriter::new(self.endian())
                    };
                    self.build_sarc(&f, &mut sarc_writer)?
                } else if f.is_file() {
                    self.get_resource_data(&f)?
                } else {
                    return Ok(());
                };
                self.set_resource_size(&botw_utils::get_canon_name_without_root(&add_path), &data);
                let ext = add_path.rfind('.').map(|i| &add_path[i + 1..]);
                sarc.add_file(
                    &add_path,
                    if &data[0..4] != b"Yaz0"
                        && ext.map(|e| e.starts_with('s')).unwrap_or(false)
                        && ext != Some("sarc")
                    {
                        compress(data)
                    } else {
                        data
                    },
                );
                Ok(())
            })?;
        if !sarc.is_empty() {
            Ok(sarc.to_binary())
        } else {
            Ok(vec![])
        }
    }

    fn build_packs(&self) -> Result<()> {
        for root in [&self.aoc, &self.content] {
            let source_root = self.source.join(root);
            let packs = glob::glob(&source_root.join("Pack/*.pack").to_string_lossy())?
                .filter_map(Result::ok)
                .filter(|f| {
                    SARC_EXTS.contains(&f.extension())
                        && self.modified_files.par_iter().any(|mf| mf.starts_with(f))
                })
                .collect::<Vec<_>>();
            println!("Building {} packs", packs.len());
            packs.into_par_iter().try_for_each(|pack| -> Result<()> {
                self.vprint(&format!(
                    "Building {}",
                    pack.file_name()
                        .and_then(|n| n.to_str())
                        .context("No pack name")?
                ));
                let out = self
                    .output
                    .join(&root)
                    .join(pack.strip_prefix(&source_root)?);
                let mut sarc = if out.exists() {
                    SarcWriter::from(Sarc::read(fs::read(&out)?)?)
                } else {
                    SarcWriter::new(self.endian())
                };
                fs::create_dir_all(out.parent().context("No parent???")?)?;
                fs::write(out, self.build_sarc(&pack, &mut sarc)?)?;
                Ok(())
            })?;
        }
        Ok(())
    }

    fn build_maps(&self) -> Result<()> {
        for root in [&self.aoc, &self.content] {
            let map_dir = self.source.join(root).join("Map");
            if self
                .modified_files
                .par_iter()
                .any(|f| f.starts_with(&map_dir))
            {
                println!(
                    "Building {} maps",
                    if root == &self.aoc { "DLC" } else { "base" }
                );
                let yml_ext = Some(OsStr::new("yml"));
                self.modified_files
                    .par_iter()
                    .filter(|f| f.starts_with(&map_dir))
                    .try_for_each(|f| -> Result<()> {
                        let out = self
                            .output
                            .join(root)
                            .join("Map")
                            .join(f.strip_prefix(&map_dir)?);
                        fs::create_dir_all(&out.parent().context("No parent??")?)?;
                        fs::write(
                            if out.extension() == yml_ext {
                                out.with_extension("")
                            } else {
                                out
                            },
                            self.get_resource_data(f)?,
                        )?;
                        Ok(())
                    })?;
            }
        }
        Ok(())
    }

    fn build_misc(&self) -> Result<()> {
        let phys_root = self.source_content().join("Physics");
        let (phys_hksc, phys_tmrb) = (
            phys_root.join("StaticCompound"),
            phys_root.join("TeraMeshRigidBody"),
        );
        let misc_files: Vec<&PathBuf> = [&self.aoc, &self.content]
            .into_iter()
            .map(|r| {
                UNPROCESSED_DIRS
                    .iter()
                    .map(|d| self.source.join(r).join(d))
                    .collect::<Vec<PathBuf>>()
            })
            .flatten()
            .map(|s| {
                self.modified_files
                    .par_iter()
                    .filter(|f| f.starts_with(&s))
                    .collect::<Vec<&PathBuf>>()
            })
            .flatten()
            .filter(|f| {
                !(f.starts_with(&phys_root)
                    || (f.starts_with(&phys_hksc) || f.starts_with(&phys_tmrb)))
            })
            .collect();
        if !misc_files.is_empty() {
            println!("Building {} miscellaneous files", misc_files.len());
            misc_files.into_par_iter().try_for_each(|f| -> Result<()> {
                let out = self.output.join(f.strip_prefix(&self.source)?);
                fs::create_dir_all(out.parent().context("No parent???")?)?;
                if let Some(canon) = self.get_canon_name(f) {
                    let data = fs::read(&f)?;
                    self.set_resource_size(&canon, &data);
                    fs::write(&out, data)?;
                } else {
                    fs::copy(&f, &out)?;
                }
                Ok(())
            })?;
        }
        Ok(())
    }

    fn build_rstb(&self) -> Result<()> {
        println!("Building RSTB");
        let res_dir = self.output.join(&self.content).join("System/Resource");
        fs::create_dir_all(&res_dir)?;
        fs::write(
            res_dir.join("ResourceSizeTable.product.srsizetable"),
            compress(self.size_table.lock().unwrap().to_binary(if self.be {
                rstb::Endian::Big
            } else {
                rstb::Endian::Little
            })),
        )?;
        Ok(())
    }

    fn update_db(&mut self) -> Result<()> {
        println!("Saving state");
        let moment = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        self.file_times
            .extend(self.modified_files.iter().map(|f| (f.clone(), moment)));
        let mut db = std::fs::File::create(self.source.join(".db"))?;
        for (f, t) in &self.file_times {
            writeln!(
                db,
                "{},{}",
                f.strip_prefix(&self.source)?.to_slash_lossy(),
                t
            )?;
        }
        Ok(())
    }

    fn build_meta(&self) -> Result<()> {
        if self.be {
            let mut file = fs::File::create(self.output.join("rules.txt"))?;
            writeln!(file, "[Definition]")?;
            writeln!(
                file,
                "titleIds = 00050000101C9300,00050000101C9400,00050000101C9500"
            )?;
            if !self.meta.contains_key("path") && self.meta.contains_key("name") {
                writeln!(
                    file,
                    "path = The Legend of Zelda: Breath of the Wild/Mods/{}",
                    self.meta["name"]
                )?;
            }
            for (k, v) in self.meta.iter() {
                writeln!(file, "{} = {}", k, v)?;
            }
            writeln!(file, "version = 7")?;
        }
        Ok(())
    }

    pub fn build(&mut self) -> Result<()> {
        if !validate_source(&self.source) {
            return Err(anyhow!("Source folder is not a Hyrule Builder project"));
        }
        self.load_modified_files()?;
        if self.modified_files.is_empty() {
            println!("Nope, nothing to do");
            return Ok(());
        }
        if self.source_content().join("Actor/ActorInfo").exists() {
            self.load_actorinfo()?;
        }
        self.build_actors()?;
        self.build_actorinfo()?;
        self.build_events()?;
        self.build_texts()?;
        self.build_packs()?;
        self.build_maps()?;
        self.build_misc()?;
        self.build_rstb()?;
        self.build_meta()?;
        self.update_db()?;
        Ok(())
    }
}

const CRC32: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

fn hash_name(name: &str) -> u32 {
    CRC32.checksum(name.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::{Builder, WarnLevel};
    use botw_utils::hashes::{Platform, StockHashTable};
    use rstb::ResourceSizeTable;
    use std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    #[test]
    fn test_hash() {
        assert_eq!(super::hash_name("EnemyFortressMgrTag"), 31119);
        // for (i, alg) in [
        //     crc::CRC_32_AIXM,
        //     crc::CRC_32_AUTOSAR,
        //     crc::CRC_32_BASE91_D,
        //     crc::CRC_32_BZIP2,
        //     crc::CRC_32_CD_ROM_EDC,
        //     crc::CRC_32_CKSUM,
        //     crc::CRC_32_ISCSI,
        //     crc::CRC_32_ISO_HDLC,
        //     crc::CRC_32_JAMCRC,
        //     crc::CRC_32_MPEG_2,
        //     crc::CRC_32_XFER,
        // ]
        // .iter()
        // .enumerate()
        // {
        //     let crc: crc::Crc<u32> = crc::Crc::<u32>::new(alg);
        //     if crc.checksum(b"EnemyFortressMgrTag") == 31119 {
        //         println!("{}", i);
        //         return;
        //     }
        // }
        // panic!("None worked!!!");
    }

    #[test]
    fn build_u() {
        std::fs::remove_dir_all("test/project/build").unwrap_or(());
        std::fs::remove_file("test/project/.db").unwrap_or(());
        Builder {
            be: true,
            file_times: HashMap::new(),
            meta: HashMap::new(),
            modified_files: HashSet::new(),
            actorinfo: None,
            hash_table: StockHashTable::new(&Platform::WiiU),
            size_table: Arc::new(Mutex::new(ResourceSizeTable::new_from_stock(
                rstb::Endian::Big,
            ))),
            content: PathBuf::from("content"),
            aoc: PathBuf::from("aoc/0010"),
            output: "test/project/build".into(),
            source: "test/project".into(),
            title_actors: super::actor::TITLE_ACTORS
                .iter()
                .map(|t| t.to_string())
                .collect(),
            title_events: super::event::TITLE_EVENTS
                .iter()
                .chain(super::event::NESTED_EVENTS.iter())
                .map(|t| t.to_string())
                .collect(),
            compiled: Arc::new(Mutex::new(HashMap::new())),
            verbose: false,
            warn: WarnLevel::Warn,
        }
        .build()
        .unwrap()
    }

    #[test]
    fn glob_test() {
        dbg!(glob::glob("test/project/content/Pack/**/*.*")
            .unwrap()
            .filter_map(Result::ok)
            .collect::<Vec<_>>());
    }
}
