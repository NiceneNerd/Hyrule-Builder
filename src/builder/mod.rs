#![allow(unused_imports, dead_code)]

mod event;

use botw_utils::{extensions::*, get_canon_name, get_canon_name_without_root};
use crate::pre::*;
use rayon::prelude::*;
use relative_path::RelativePathBuf;
use std::fs;
use std::io::{BufWriter, Cursor};
use std::path::{Path, PathBuf, StripPrefixError};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

#[inline]
fn should_compress<P: AsRef<Path>>(path: P) -> bool {
    let ext = path.as_ref().extension().unwrap().to_str().unwrap();
    ext.starts_with("s") && ext != "sarc"
}

#[inline]
fn get_sarc_parent<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    path.as_ref()
        .ancestors()
        .find(|f| {
            f.extension().is_some() && SARC_EXTS.contains(&f.extension().unwrap().to_str().unwrap())
        })
        .map(|f| f.to_owned())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WarnLevel {
    None,
    Warn,
    Error,
}

#[derive(Debug, PartialEq)]
pub enum BuiltYaml {
    Byml(Byml),
    Aamp(ParameterIO),
    Msbt(Msyt),
}

impl BuiltYaml {
    fn read_from_path<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let path = path.as_ref();
        let ext = path.extension().unwrap().to_str().unwrap();
        let contents = fs::read_to_string(path)?;
        if ext == "msyt" {
            Ok(Some(Self::Msbt(serde_yaml::from_str(&contents)?)))
        } else {
            if let Some(ext) = path.with_extension("").extension() {
                let ext = ext.to_str().unwrap();
                if BYML_EXTS.contains(&ext) {
                    Ok(Some(Self::Byml(
                        Byml::from_text(&contents).map_err(|e| format_err!("{:?}", e))?,
                    )))
                } else if AAMP_EXTS.contains(&ext) {
                    Ok(Some(Self::Aamp(ParameterIO::from_text(&contents)?)))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
    }

    fn into_binary(self, endian: Endian) -> Result<Vec<u8>> {
        Ok(match self {
            Self::Byml(byml) => byml.to_binary(endian.into(), 2)?,
            Self::Aamp(pio) => pio.to_binary()?,
            Self::Msbt(msyt) => msyt
                .into_msbt_bytes(endian.into())
                .map_err(|e| format_err!("{:?}", e))?,
        })
    }

    fn into_compressed_binary(self, endian: Endian) -> Result<Vec<u8>> {
        let uncompressed = self.into_binary(endian)?;
        let mut data: Vec<u8> = Vec::with_capacity(uncompressed.len());
        Yaz0Writer::new(&mut BufWriter::new(&mut data))
            .compress_and_write(&uncompressed, COMPRESS)?;
        Ok(data)
    }

    fn into_maybe_compressed_binary<P: AsRef<Path>>(
        self,
        endian: Endian,
        path: P,
    ) -> Result<Vec<u8>> {
        if should_compress(path) {
            self.into_compressed_binary(endian)
        } else {
            self.into_binary(endian)
        }
    }
}

#[derive(Debug)]
pub struct ModBuilder {
    endian: Endian,
    meta: HashMap<String, String>,
    input_dir: PathBuf,
    output_dir: PathBuf,
    content_path: RelativePathBuf,
    dlc_path: RelativePathBuf,
    warn_level: WarnLevel,
    verbose: bool,
    title_actors: HashSet<String>,
    file_times: HashMap<PathBuf, u64>,
    modified_files: HashSet<PathBuf>,
    compiled_yaml: HashMap<String, BuiltYaml>,
    canonical_paths: HashMap<PathBuf, String>,
}

impl ModBuilder {
    #[inline]
    fn content_path(&self) -> PathBuf {
        self.content_path.to_path(&self.input_dir)
    }

    fn load_modified_files(&mut self) -> Result<()> {
        let input_dir = self.input_dir.clone();
        if self.input_dir.join(".db").exists() {
            self.file_times.extend(
                fs::read_to_string(self.input_dir.join(".db"))
                    .unwrap()
                    .split('\n')
                    .filter(|x| x != &"")
                    .map(|l| {
                        let data: Vec<&str> = l.split(',').collect();
                        (input_dir.join(data[0]), str::parse::<u64>(data[1]).unwrap())
                    }),
            );
        }
        self.modified_files = glob::glob(self.input_dir.join("**").join("*.*").to_str().unwrap())
            .unwrap()
            .filter_map(|f| {
                if let Ok(file) = f {
                    if file.is_file()
                        && (!self.file_times.contains_key(&file) || {
                            let modified = fs::metadata(&file).unwrap().modified().unwrap();
                            modified
                                .duration_since(
                                    std::time::UNIX_EPOCH
                                        .checked_add(std::time::Duration::from_secs(
                                            *self.file_times.get(&file).unwrap(),
                                        ))
                                        .unwrap(),
                                )
                                .is_ok()
                        })
                    {
                        Some(file)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        Ok(())
    }

    fn register_paths(&mut self) -> Result<()> {
        let input = self.input_dir.clone();
        self.canonical_paths = self
            .modified_files
            .par_iter()
            .map(|f| -> Result<(PathBuf, String)> {
                Ok((
                    f.clone(),
                    if let Some(parent) = get_sarc_parent(f) {
                        get_canon_name_without_root(&f.strip_prefix(parent)?)
                    } else {
                        get_canon_name(f.strip_prefix(&input)?).unwrap()
                    },
                ))
            })
            .collect::<Result<HashMap<PathBuf, String>>>()?;
        Ok(())
    }

    fn compile_yaml(&mut self) -> Result<()> {
        let canon_map = std::sync::Arc::new(&self.canonical_paths);
        self.compiled_yaml = self
            .modified_files
            .par_iter()
            .filter(|f| {
                f.extension().is_some()
                    && ["yml", "msyt"].contains(&f.extension().unwrap().to_str().unwrap())
            })
            .filter_map(|f| -> Option<Result<(String, BuiltYaml)>> {
                let canon = canon_map.get(f).unwrap().to_owned();
                if let Some(built) = BuiltYaml::read_from_path(f).transpose() {
                    Some(if let Ok(built) = built {
                        Ok((canon, built))
                    } else {
                        Err(built.unwrap_err())
                    })
                } else {
                    None
                }
            })
            .collect::<Result<HashMap<String, BuiltYaml>>>()?;
        Ok(())
    }

    fn build(&mut self) -> Result<()> {
        self.load_modified_files()?;
        self.register_paths()?;
        self.compile_yaml()?;
        Ok(())
    }
}
