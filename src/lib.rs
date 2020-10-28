use botw_utils::hashes::{Platform, StockHashTable};
use glob::glob;
use path_macro::path;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::{HashMap, HashSet};
use std::fs::{metadata, read_to_string, write};
use std::path::{Path, PathBuf};

#[pymodule]
pub fn hyrule_builder(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<ModBuilderR>()?;
    Ok(())
}

#[pyclass]
#[derive(Debug)]
pub struct ModBuilderR {
    input: PathBuf,
    output: PathBuf,
    meta: HashMap<String, String>,
    content: String,
    aoc: String,
    be: bool,
    guess: bool,
    verbose: bool,
    titles: HashSet<String>,
    table: StockHashTable,
    warn: bool,
    strict: bool,
    single: bool,
    no_rstb: bool,
    file_times: HashMap<PathBuf, u64>,
}

#[pymethods]
impl ModBuilderR {
    #[new]
    pub fn new(
        input: &str,
        output: &str,
        meta: &PyDict,
        be: bool,
        guess: bool,
        verbose: bool,
        titles: &str,
        warn: bool,
        strict: bool,
        single: bool,
        no_rstb: bool,
    ) -> PyResult<Self> {
        let input = PathBuf::from(input);
        let output = PathBuf::from(output);
        let content = String::from(if be {
            "content"
        } else {
            "01007EF00011E000/romfs"
        });
        let aoc = String::from(if be { "aoc" } else { "01007EF00011F001/romfs" });

        if !path!(input / content).exists() && !path!(input / aoc).exists() {
            return Err(pyo3::exceptions::PyValueError::new_err("Invalid folders"));
        }
        let mut file_times: HashMap<PathBuf, u64> = HashMap::new();
        if path!(input / ".done").exists() {
            for line in read_to_string(path!(input / ".done"))?
                .split('\n')
                .filter(|x| *x != "")
            {
                let data: Vec<&str> = line.split(',').collect();
                file_times.insert(PathBuf::from(data[0]), str::parse::<u64>(data[1])?);
            }
        }

        Ok(ModBuilderR {
            input,
            output,
            meta: meta.extract::<HashMap<String, String>>().unwrap(),
            be,
            guess,
            verbose,
            warn,
            strict,
            single,
            no_rstb,
            content,
            aoc,
            titles: titles
                .split(',')
                .map(|x| x.to_string())
                .collect::<HashSet<String>>(),
            table: StockHashTable::new(&(if be { Platform::WiiU } else { Platform::Switch })),
            file_times,
        })
    }

    pub fn build(&mut self) -> PyResult<()> {
        let files: Vec<PathBuf> = glob(&path!(self.input / "**" / "*").to_string_lossy())
            .unwrap()
            .filter_map(|x| {
                if let Ok(path) = x {
                    if let Ok(path) = path.strip_prefix(&self.input) {
                        if !path
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy())
                            .any(|c| c == "build" || c.starts_with('.'))
                        {
                            return Some(path.to_owned());
                        }
                    }
                }
                None
            })
            .collect();
        let fresh_files: Vec<PathBuf> = files
            .into_iter()
            .filter(|x| {
                let mod_time = metadata(path!(self.input / x)).unwrap().modified().unwrap();
                !self.file_times.contains_key(x)
                    || mod_time
                        .duration_since(
                            std::time::UNIX_EPOCH
                                .checked_add(std::time::Duration::from_secs(
                                    *self.file_times.get(x).unwrap(),
                                ))
                                .unwrap(),
                        )
                        .is_ok()
            })
            .collect();
        println!("{:?}", fresh_files);
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for file in &fresh_files {
            self.file_times.insert(file.to_owned(), time);
        }
        write(
            path!(self.input / ".done"),
            &self
                .file_times
                .iter()
                .map(|(f, t)| format!("{},{}\n", f.to_string_lossy(), t))
                .collect::<String>(),
        )
        .unwrap();

        println!("Mod built successfully!");
        Ok(())
    }
}
