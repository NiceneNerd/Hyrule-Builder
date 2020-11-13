use rayon::prelude::*;
use sarc::SarcEntry;
use std::ops::{Deref, DerefMut};

// pub type SarcFile = sarc::SarcFile;
#[derive(Debug)]
pub struct SarcFile(pub sarc::SarcFile);

impl Deref for SarcFile {
    type Target = sarc::SarcFile;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SarcFile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for SarcFile {
    fn default() -> SarcFile {
        SarcFile(sarc::SarcFile {
            byte_order: sarc::Endian::Big,
            files: vec![],
        })
    }
}

impl From<sarc::SarcFile> for SarcFile {
    fn from(sarc: sarc::SarcFile) -> Self {
        SarcFile(sarc)
    }
}

pub trait SarcFileExt {
    fn new(byte_order: sarc::Endian) -> Self;
    fn get_file(&self, path: &str) -> Option<&SarcEntry>;
    fn add_file(&mut self, path: &str, data: &[u8]);
    fn add_entries(&mut self, entries: &[SarcEntry]);
    fn read_from_file<P: AsRef<std::path::Path>>(file: P) -> Result<SarcFile, sarc::parser::Error>;
}

impl SarcFileExt for SarcFile {
    fn new(byte_order: sarc::Endian) -> Self {
        Self(sarc::SarcFile {
            byte_order,
            files: vec![],
        })
    }

    fn read_from_file<P: AsRef<std::path::Path>>(file: P) -> Result<Self, sarc::parser::Error> {
        Ok(sarc::SarcFile::read_from_file(file)?.into())
    }

    #[inline]
    fn get_file(&self, path: &str) -> Option<&SarcEntry> {
        self.files
            .par_iter()
            .find_first(|x| x.name.is_some() && x.name.as_ref().unwrap() == path)
    }

    fn add_file(&mut self, path: &str, data: &[u8]) {
        if self.get_file(path).is_some() {
            self.files
                .retain(|x| x.name.is_none() || x.name.as_ref().unwrap() != path);
        };
        self.files.push(SarcEntry {
            name: Some(path.to_owned()),
            data: data.to_owned(),
        })
    }

    fn add_entries(&mut self, entries: &[SarcEntry]) {
        for entry in entries {
            self.add_file(entry.name.as_ref().unwrap(), entry.data.as_ref());
        }
    }
}
