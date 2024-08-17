use super::{Builder, Result};
use crate::util::get_ext;
use anyhow::Context;
use join_str::jstr;
use path_slash::PathBufExt;
use roead::{
    byml::{Byml, Map},
    sarc::SarcWriter,
    yaz0::compress,
};
use std::{
    collections::HashSet,
    ffi::OsStr,
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

pub static NESTED_EVENTS: &[&str] = &["SignalFlowchart"];
pub static TITLE_EVENTS: &[&str] = &[
    "AocResident",
    "Aoc2Resident",
    "Demo000_0",
    "Demo000_2",
    "Demo001_0",
    "Demo002_0",
    "Demo005_0",
    "Demo006_0",
    "Demo007_1",
    "Demo008_1",
    "Demo008_3",
    "Demo010_0",
    "Demo010_1",
    "Demo011_0",
    "Demo017_0",
    "Demo025_0",
    "Demo042_0",
    "Demo042_1",
    "Demo048_0",
    "Demo048_1",
    "Demo103_0",
    "GetDemo",
    "OperationGuide",
    "SDemo_D-6",
];

pub struct Event<'a> {
    builder: &'a Builder,
    pub name: String,
    files: HashSet<PathBuf>,
}

impl<'a> Debug for Event<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("name", &self.name)
            .field("files", &self.files)
            .finish()
    }
}

impl<'a> Event<'a> {
    pub fn new(
        builder: &'a Builder,
        file: &Path,
    ) -> Result<(
        std::collections::hash_map::IntoIter<smartstring::alias::String, roead::byml::Byml>,
        Option<Self>,
    )> {
        let name = file
            .file_stem()
            .context("Missing filename")?
            .to_str()
            .unwrap()
            .trim_end_matches(".info");
        let event_info: Map = Byml::from_text(
            fs::read_to_string(file)
                .with_context(|| format!("Failed to read event file at {}", file.display()))?,
        )
        .with_context(|| format!("Failed to parse YAML at {}", file.display()))?
        .into_map()?
        .into_iter()
        .map(|(mut k, v)| {
            k.insert_str(0, name);
            k.insert(name.len(), '<');
            k.push('>');
            (k, v)
        })
        .collect();
        if builder.title_events.contains(name) {
            return Ok((event_info.into_iter(), None));
        }
        let root = builder.source_content();
        let event_flow_root = root.join("EventFlow");
        let as_root = root.join("Actor/AS").join(name);
        let camera_root = root.join("Camera").join(name);
        let main_exts = [Some(OsStr::new("bfevfl")), Some(OsStr::new("bfevtm"))];
        let files: HashSet<PathBuf> = find_subfiles(&event_info)?
            .map(|file| event_flow_root.join(file))
            .chain(
                find_as_files(&event_info)?
                    .map(|file| as_root.join(file).with_extension("bas.yml")),
            )
            .chain(find_camera_files(&event_info)?.map(|file| camera_root.join(file)))
            .chain(find_single_files(&event_info, name)?.map(|file| root.join(file)))
            .collect();
        if !files.is_empty()
            && files
                .iter()
                .chain(&[file.to_owned()])
                .any(|f| builder.modified_files.contains(f))
            && !files
                .iter()
                .filter(|f| {
                    main_exts.contains(&f.extension())
                        || f.file_stem().unwrap().to_str().unwrap().ends_with(".bdemo")
                })
                .filter(|f| {
                    !builder
                        .title_events
                        .any(|e| f.file_name().unwrap().to_str().unwrap().contains(e))
                })
                .any(|f| !f.exists())
        {
            builder.vprint(&jstr!("Event {&name} modified"));
            Ok((
                event_info.into_iter(),
                Some(Self {
                    builder,
                    files,
                    name: name.to_owned(),
                }),
            ))
        } else {
            Ok((event_info.into_iter(), None))
        }
    }

    pub fn build(self) -> Result<Vec<u8>> {
        self.builder
            .vprint(&jstr!("Building event pack {&self.name}"));
        let mut pack = SarcWriter::new(self.builder.endian());
        let root = self.builder.source.join(&self.builder.content);
        self.files.into_iter().try_for_each(|f| -> Result<()> {
            let mut filename = f
                .strip_prefix(&root)
                .with_context(|| f.to_slash_lossy().to_string())?
                .to_owned();
            if !f.exists() {
                if !self
                    .builder
                    .title_events
                    .any(|e| f.file_name().unwrap().to_str().unwrap().contains(e))
                    && !root.join("Pack/TitleBG.pack").join(&filename).exists()
                {
                    self.builder.warn(&jstr!(
                        "Event {&self.name} missing file {&f.to_slash_lossy()}"
                    ))?;
                }
                return Ok(());
            };
            if get_ext(&filename)? == "yml" {
                filename = filename.with_extension("");
            }
            let data = self.builder.get_resource_data(&f)?;
            pack.add_file(filename.to_slash_lossy(), data);
            Ok(())
        })?;
        let data = pack.to_binary();
        self.builder
            .set_resource_size(&jstr!("Event/{&self.name}.beventpack"), &data);
        self.builder.vprint(&jstr!("Built event {&self.name}"));
        Ok(compress(data))
    }
}

fn find_subfiles(event_info: &Map) -> Result<impl Iterator<Item = &str>> {
    Ok(event_info
        .values()
        .filter_map(|info| {
            info.as_map()
                .ok()
                .and_then(|hash| hash.get("subfile"))
                .and_then(|subfiles| subfiles.as_array().ok())
                .map(|subfiles| {
                    subfiles
                        .iter()
                        .filter_map(|file| {
                            file.as_map()
                                .ok()
                                .and_then(|file_hash| file_hash.get("file"))
                                .and_then(|file_val| file_val.as_string().ok().map(|s| s.as_str()))
                        })
                        .collect::<Vec<_>>()
                })
        })
        .flatten())
}

fn find_as_files(event_info: &Map) -> Result<impl Iterator<Item = &str>> {
    Ok(event_info
        .values()
        .filter_map(|info| {
            info.as_map()
                .ok()
                .and_then(|hash| hash.get("as"))
                .and_then(|subfiles| subfiles.as_array().ok())
                .map(|subfiles| {
                    subfiles
                        .iter()
                        .filter_map(|file| {
                            file.as_map()
                                .ok()
                                .and_then(|file_hash| file_hash.get("file"))
                                .and_then(|file_val| file_val.as_string().ok().map(|s| s.as_str()))
                        })
                        .collect::<Vec<_>>()
                })
        })
        .flatten())
}

fn find_camera_files(event_info: &Map) -> Result<impl Iterator<Item = &str>> {
    Ok(event_info
        .values()
        .filter_map(|info| {
            info.as_map()
                .ok()
                .and_then(|hash| hash.get("camera"))
                .and_then(|subfiles| subfiles.as_array().ok())
                .map(|subfiles| {
                    subfiles
                        .iter()
                        .filter_map(|file| {
                            file.as_map()
                                .ok()
                                .and_then(|file_hash| file_hash.get("file"))
                                .and_then(|file_val| file_val.as_string().ok().map(|s| s.as_str()))
                        })
                        .collect::<Vec<_>>()
                })
        })
        .flatten())
}

fn find_single_files(event_info: &Map, name: &str) -> Result<impl Iterator<Item = PathBuf>> {
    let mut files: HashSet<PathBuf> = HashSet::with_capacity(1);
    if name == "Demo614_2" {
        // Hack because I have no idea why `Demo614_2.sbeventpack` has a timeline
        files.insert("EventFlow/Demo614_2.bfevtm".into());
    }
    for subevent in event_info.values() {
        let subevent = subevent.as_map()?;
        if let Some(Byml::Bool(has_demo)) = subevent.get("demo_event") {
            if *has_demo {
                files.insert(jstr!("Demo/{name}.bdemo.yml").into());
            };
        };
        if let Some(Byml::Bool(has_timeline)) = subevent.get("is_timeline") {
            if *has_timeline {
                files.insert(jstr!("EventFlow/{name}.bfevtm").into());
                files.insert(jstr!("EventFlow/{name}_effect.bfevtm").into());
            } else {
                files.insert(jstr!("EventFlow/{name}.bfevfl").into());
            };
        } else {
            files.insert(jstr!("EventFlow/{name}.bfevfl").into());
        };
        if let Some(Byml::String(elink)) = subevent.get("elink_user") {
            files.insert(jstr!("Effect/{elink}.sesetlist").into());
        };
        if let Some(Byml::String(slink)) = subevent.get("slink_user") {
            files.insert(jstr!("Sound/Resource/{slink}.bars").into());
        };
        if let Some(Byml::Bool(has_model)) = subevent.get("exist_extra_model") {
            if *has_model {
                files.insert(jstr!("Model/{name}.sbfres").into());
            }
        };
    }
    Ok(files.into_iter())
}
