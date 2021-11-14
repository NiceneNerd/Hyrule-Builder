use anyhow::{format_err, Context, Result};
pub use botw_utils::extensions::{AAMP_EXTS, BYML_EXTS};
use jstr::jstr;
use lazy_static::lazy_static;
use path_slash::PathExt;
use roead::aamp::ParameterIO;
use std::path::Path;

pub static PROCESSED_DIRS: &[&str] = &["Actor", "Event", "Map", "Pack"];

pub static UNPROCESSED_DIRS: &[&str] = &[
    "Effect",
    "Font",
    "Game",
    "Layout",
    "Local",
    "Model",
    "Movie",
    "NavMesh",
    "Physics",
    "Sound",
    "Terrain",
    "StockItem",
    "System",
    "Voice",
    "UI",
];

pub static EXCLUDE_RSTB: &[&str] = &[
    "pack", "bgdata", "txt", "bgsvdata", "yml", "json", "ps1", "bak", "bat", "ini", "png", "bfstm",
    "py", "sh", "old", "stera",
];

lazy_static! {
    pub static ref SARC_EXTS: Vec<Option<&'static std::ffi::OsStr>> =
        botw_utils::extensions::SARC_EXTS
            .iter()
            .map(|ext| Some(std::ffi::OsStr::new(ext)))
            .collect();
}

pub fn get_ext(file: &Path) -> Result<&str> {
    file.extension()
        .ok_or_else(|| format_err!("No extension on {:?}", &file))?
        .to_str()
        .context("Invalid UTF 8 in file extension")
}

pub fn parse_aamp(file: &Path) -> Result<ParameterIO> {
    if file.extension() == Some(std::ffi::OsStr::new("yml")) {
        Ok(ParameterIO::from_text(std::fs::read_to_string(file)?)
            .with_context(|| jstr!("Failed to parse AAMP file {&file.to_slash_lossy()}"))?)
    } else {
        Ok(ParameterIO::from_binary(std::fs::read(file)?)
            .with_context(|| jstr!("Failed to parse AAMP file {&file.to_slash_lossy()}"))?)
    }
}

#[inline]
pub fn validate_source(source: &Path) -> bool {
    source.join("content").exists()
        || source.join("aoc").exists()
        || source.join("01007EF00011E000/romfs").exists()
        || source.join("01007EF00011F001/romfs").exists()
}
