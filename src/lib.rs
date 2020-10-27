use botw_utils::hashes::{Platform, StockHashTable};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::{HashMap, HashSet};
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
    out: PathBuf,
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
    ) -> Self {
        ModBuilderR {
            input: PathBuf::from(input),
            out: PathBuf::from(output),
            meta: meta.extract::<HashMap<String, String>>().unwrap(),
            be,
            guess,
            verbose,
            warn,
            strict,
            single,
            no_rstb,
            content: String::from(if be {
                "content"
            } else {
                "01007EF00011E000/romfs"
            }),
            aoc: String::from(if be { "aoc" } else { "01007EF00011F001/romfs" }),
            titles: titles
                .split(",")
                .map(|x| x.to_string())
                .collect::<HashSet<String>>(),
            table: StockHashTable::new(&(if be { Platform::WiiU } else { Platform::Switch })),
        }
    }

    pub fn build(&self) {
        println!("Building from Rust!");
    }
}