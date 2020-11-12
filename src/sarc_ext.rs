use rayon::prelude::*;
use sarc::SarcEntry;

pub type SarcFile = sarc::SarcFile;

pub trait SarcFileExt {
    fn get_file(&self, path: &str) -> Option<&SarcEntry>;
    fn add_file(&mut self, path: &str, data: &[u8]) -> ();
    fn add_entries(&mut self, entries: &Vec<SarcEntry>) -> ();
}

impl SarcFileExt for SarcFile {
    fn get_file(&self, path: &str) -> Option<&SarcEntry> {
        self.files
            .par_iter()
            .find_first(|x| x.name == Some(path.to_owned()))
    }

    fn add_file(&mut self, path: &str, data: &[u8]) -> () {
        if let Some(file) = self.get_file(path) {
            let pos = self
                .files
                .par_iter()
                .position_first(|x| x.name == file.name)
                .unwrap();
            self.files.remove(pos);
        };
        self.files.push(SarcEntry {
            name: Some(path.to_owned()),
            data: data.to_vec(),
        })
    }

    fn add_entries(&mut self, entries: &Vec<SarcEntry>) -> () {
        for entry in entries {
            self.add_file(entry.name.as_ref().unwrap(), entry.data.as_ref());
        }
    }
}
