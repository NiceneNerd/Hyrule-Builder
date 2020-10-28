use botw_utils::hashes::{Platform, StockHashTable};
use chrono::prelude::*;
use glob::glob;
use path_macro::path;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pyfunction;
use std::collections::{HashMap, HashSet};
use std::fs::{metadata, read_to_string, write};
use std::path::{Path, PathBuf};

#[pymodule]
pub fn hyrule_builder(_: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(build_mod, m)?)?;
    Ok(())
}

#[derive(FromPyObject, Debug)]
pub struct BuildArgs {
    #[pyo3(attribute("directory"))]
    input: String,
    #[pyo3(attribute("output"))]
    output: Option<String>,
    #[pyo3(attribute("be"))]
    be: bool,
    #[pyo3(attribute("no_rstb"))]
    no_rstb: bool,
    #[pyo3(attribute("no_guess"))]
    no_guess: bool,
    #[pyo3(attribute("no_warn"))]
    no_warn: bool,
    #[pyo3(attribute("hard_warn"))]
    strict: bool,
    #[pyo3(attribute("verbose"))]
    verbose: bool,
    #[pyo3(attribute("single"))]
    single: bool,
    #[pyo3(attribute("title_actors"))]
    title_actors: String,
}

struct Actor<'a> {
    name: &'a str,
    name_jpn: &'a str,
    priority: &'a str,
    aiprogram: String,
    aischedule: String,
}

#[pyfunction]
pub fn build_mod(args: BuildArgs, meta: &PyDict) -> PyResult<()> {
    let input = PathBuf::from(&args.input);
    let output = if let Some(out) = args.output {
        PathBuf::from(out)
    } else {
        path!(input / "build")
    };
    let content = String::from(if args.be {
        "content"
    } else {
        "01007EF00011E000/romfs"
    });
    let aoc = String::from(if args.be {
        "aoc"
    } else {
        "01007EF00011F001/romfs"
    });

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

    let mut builder = ModBuilder {
        input,
        output,
        meta: meta.extract::<HashMap<String, String>>().unwrap(),
        be: args.be,
        guess: !args.no_guess,
        verbose: args.verbose,
        warn: !args.no_warn,
        strict: args.strict,
        single: args.single,
        no_rstb: args.no_rstb,
        content,
        aoc,
        titles: args
            .title_actors
            .split(',')
            .map(|x| x.to_string())
            .collect::<HashSet<String>>(),
        table: StockHashTable::new(
            &(if args.be {
                Platform::WiiU
            } else {
                Platform::Switch
            }),
        ),
        file_times,
    };
    builder.build()?;
    Ok(())
}

#[derive(Debug)]
pub struct ModBuilder {
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

impl ModBuilder {
    fn save_times(&self) -> PyResult<()> {
        write(
            path!(self.input / ".done"),
            &self
                .file_times
                .iter()
                .map(|(f, t)| format!("{},{}\n", f.to_string_lossy(), t))
                .collect::<String>(),
        )
        .unwrap();
        Ok(())
    }

    fn build(&mut self) -> PyResult<()> {
        let files: Vec<PathBuf> = glob(&path!(self.input / "**" / "*").to_string_lossy())
            .unwrap()
            .filter_map(|x| {
                if let Ok(path) = x {
                    if path.is_file() {
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
                }
                None
            })
            .collect();
        let mut updated_files: Vec<PathBuf> = vec![];
        let mut yml_files: Vec<PathBuf> = vec![];
        let mut other_files: Vec<PathBuf> = vec![];
        for file in &files {
            let modified = metadata(path!(self.input / file))
                .unwrap()
                .modified()
                .unwrap();
            if !self.file_times.contains_key(file)
                || modified
                    .duration_since(
                        std::time::UNIX_EPOCH
                            .checked_add(std::time::Duration::from_secs(
                                *self.file_times.get(file).unwrap(),
                            ))
                            .unwrap(),
                    )
                    .is_ok()
            {
                updated_files.push(file.to_owned());
            }
            if let Some(ext) = file.extension() {
                if ext.to_string_lossy() == "yml" {
                    yml_files.push(file.to_owned());
                    continue;
                }
            }
            other_files.push(file.to_owned());
        }
        println!("{:?}", updated_files);

        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for file in &updated_files {
            self.file_times.insert(file.to_owned(), time);
        }
        self.save_times()?;
        println!("Mod built successfully!");
        Ok(())
    }
}
