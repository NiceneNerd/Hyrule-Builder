use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use glob::glob;
use rayon::prelude::*;

use crate::{builder::ModBuilder, pre::*};

struct EventsBuilder<'a, 'b> {
    _parent: &'a ModBuilder,
    _events: Vec<EventBuilder<'b>>,
}

impl<'a, 'b> EventsBuilder<'a, 'b> {
    fn new(builder: &'a ModBuilder) -> Result<EventsBuilder> {
        let events = glob::glob(
            builder
                .content_path()
                .join("Event")
                .join("**/*.info.yml")
                .to_str()
                .unwrap(),
        )
        .unwrap()
        .filter_map(|f| f.ok())
        .map(|f| EventBuilder::new(builder, f))
        .collect::<Result<Vec<EventBuilder>>>()?;
        Ok(EventsBuilder {
            _parent: builder,
            _events: events,
        })
    }

    fn build(&self) -> Result<()> {
        let resident_events = if self._parent.content_path().join(""){} else {};
    }
}

struct EventBuilder<'a> {
    _parent: &'a ModBuilder,
    _name: String,
    _info: Byml,
    _files: HashSet<String>,
}

impl<'a> EventBuilder<'a> {
    fn new<P: AsRef<Path>>(builder: &'a ModBuilder, info_path: P) -> Result<Self> {
        let path = info_path.as_ref();
        let name = path
            .with_extension("")
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let info =
            Byml::from_text(&fs::read_to_string(path)?).map_err(|e| format_err!("{:?}", e))?;
        Ok(EventBuilder {
            _parent: builder,
            _files: {
                let mut files = HashSet::new();
                for entry in info.as_hash()?.values() {
                    let entry = entry.as_hash()?;
                    if let Some(sub_list) = entry.get("subfile") {
                        for file_entry in sub_list.as_array()?.iter() {
                            if let Some(file_name) = file_entry.as_hash()?.get("file") {
                                let store_path =
                                    ["EventFlow", file_name.as_string()?.as_str()].join("/");
                                files.insert(store_path);
                            }
                        }
                    }
                    if let Some(as_list) = entry.get("as") {
                        for file_entry in as_list.as_array()?.iter() {
                            if let Some(file_name) = file_entry.as_hash()?.get("file") {
                                let store_path =
                                    ["AS/", &name, "/", file_name.as_string()?.as_str(), ".bas"]
                                        .join("");
                                files.insert(store_path);
                            }
                        }
                    }
                    if let Some(camera) = entry.get("camera") {
                        for file_entry in camera.as_array()?.iter() {
                            if let Some(file_name) = file_entry.as_hash()?.get("file") {
                                let store_path =
                                    ["Camera/", &name, "/", file_name.as_string()?.as_str()]
                                        .join("");
                                files.insert(store_path);
                            }
                        }
                    }
                    if let Some(demo) = entry.get("demo_event") {
                        if demo.as_bool()? {
                            let store_path = ["Demo/", &name, ".bdemo"].join("");
                            files.insert(store_path);
                        }
                    }
                    if let Some(elink) = entry.get("elink_user") {
                        let store_path = ["Effect/", &elink.as_string()?, ".sesetlist"].join("");
                        files.insert(store_path);
                    }
                    if let Some(model) = entry.get("exist_extra_model") {
                        if model.as_bool()? {
                            let store_path = ["Model/", &name, ".sbfres"].join("");
                            files.insert(store_path);
                        }
                    }
                    if let Some(slink) = entry.get("slink_user") {
                        let store_path = ["Sound/Resource/", &slink.as_string()?, ".bars"].join("");
                        files.insert(store_path);
                    }
                    if entry.get("is_timeline").unwrap().as_bool()? {
                        let store_path = ["EventFlow/", &name, ".bfevtm"].join("");
                        files.insert(store_path);
                        if let Some(sub_timelines) = entry.get("sub_timelines") {
                            for file_entry in sub_timelines.as_array()?.iter() {
                                if let Some(file_name) = file_entry.as_hash()?.get("file") {
                                    let store_path =
                                        ["EventFlow", file_name.as_string()?.as_str()].join("/");
                                    files.insert(store_path);
                                }
                            }
                        }
                    }
                }
                files
            },
            _name: name,
            _info: info,
        })
    }

    fn apply_info(&mut self, info: &mut Byml) {
        let info = info.as_mut_hash().unwrap();
        for (k, mut v) in self._info.as_mut_hash().unwrap().iter_mut() {
            info.insert([&self._name, "<", k, ">"].join(""), std::mem::take(&mut v));
        }
    }

    fn build_pack(&self) -> Result<Option<Vec<u8>>> {
        let mut event_pack = SarcWriter::new(self._parent.endian.into());
        for (file, source) in self
            ._files
            .iter()
            .map(|f| {
                (
                    f,
                    self._parent
                        .content_path
                        .to_path(&self._parent.input_dir)
                        .join(&f),
                )
            })
            .filter(|(_, source)| self._parent.modified_files.contains(source))
        {
            event_pack.files.insert(file.clone(), fs::read(&source)?);
        }
        if event_pack.files.len() > 0 {
            Ok(Some(compress(&event_pack.write_to_bytes()?)?))
        } else {
            Ok(None)
        }
    }
}
