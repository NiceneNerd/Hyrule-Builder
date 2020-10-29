use botw_utils::hashes::{Platform, StockHashTable};
// use chrono::prelude::*;
use glob::glob;
use path_macro::path;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pyfunction;
use sarc::{Endian, SarcEntry, SarcFile};
use std::collections::{HashMap, HashSet};
use std::fs::{metadata, read, read_to_string, write, File};
use std::path::PathBuf;

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

#[derive(Debug)]
struct Actor {
    name: String,
    pack: SarcFile,
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
        actors: vec![],
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
    actors: Vec<Actor>,
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

    fn parse_pio(&self, file: &PathBuf) -> PyResult<aamp::ParameterIO> {
        let mut file = File::open(file)?;
        if let Ok(pio) = aamp::ParameterIO::from_binary(&mut file) {
            Ok(pio)
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(
                "AAMP file {:?} could not be parsed",
            ))
        }
    }

    fn parse_actor(&mut self, link: &PathBuf) -> PyResult<()> {
        let yml = read_to_string(link)?;
        let pio: aamp::ParameterIO = aamp::ParameterIO::from_text(&yml).unwrap();
        let actor_dir = path!(self.input / self.content / "Actor");

        let mut files: Vec<SarcEntry> = vec![];
        let add_param_file = |path: &str, v: &str| -> PyResult<SarcEntry> {
            let pio_path = path.replace("{}", v);
            Ok(SarcEntry {
                data: self
                    .parse_pio(&path!(actor_dir / &pio_path))?
                    .to_binary()
                    .unwrap(),
                name: Some(format!("Actor/{}", pio_path)),
            })
        };

        for (k, v) in pio.object("LinkTarget").unwrap().params().iter() {
            if let aamp::Parameter::StringRef(v) = v {
                if v == "Dummy" {
                    continue;
                }
                match k {
                    3293308145 => files.push(add_param_file("AIProgram/{}.baiprog", v)?),
                    1241489578 => files.push(add_param_file("AnimationInfo/{}.baniminfo", v)?),
                    1767976113 => files.push(add_param_file("Awareness/{}.bawareness", v)?),
                    713857735 => files.push(add_param_file("BoneControl/{}.bbonectrl", v)?),
                    2863165669 => files.push(add_param_file("Chemical/{}.bchemical", v)?),
                    2307148887 => files.push(add_param_file("DamageParam/{}.bdmgparam", v)?),
                    2189637974 => files.push(add_param_file("DropTable/{}.bdrop", v)?),
                    619158934 => files.push(add_param_file("GeneralParamList/{}.bgparamlist", v)?),
                    414149463 => files.push(add_param_file("LifeCondition/{}.blifecondition", v)?),
                    1096753192 => files.push(add_param_file("LOD/{}.blod", v)?),
                    3086518481 => files.push(add_param_file("ModelList/{}.bmodellist", v)?),
                    1292038778 => files.push(add_param_file("RagdollBlendWeight/{}.brgbw", v)?),
                    1589643025 => files.push(add_param_file("Recipe/{}.brecipe", v)?),
                    2994379201 => files.push(add_param_file("ShopData/{}.bshop", v)?),
                    3926186935 => files.push(add_param_file("UMii/{}.bumii", v)?),
                    110127898 => {
                        // ASUser
                        let aslist = self
                            .parse_pio(&path!(actor_dir / "ASList" / format!("{}.baslist", v)))?;
                        for anim in aslist.list("ASDefines").unwrap().objects.values() {
                            if let aamp::Parameter::String64(filename) =
                                anim.param("Filename").unwrap()
                            {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let as_path = format!("AS/{}.bas", filename);
                                files.push(SarcEntry {
                                    data: self
                                        .parse_pio(&path!(actor_dir / &as_path))?
                                        .to_binary()
                                        .unwrap(),
                                    name: Some(format!("Actor/{}", &as_path)),
                                });
                            }
                        }
                        files.push(SarcEntry {
                            data: aslist.to_binary().unwrap(),
                            name: Some(format!("Actor/ASList/{}.baslist", v)),
                        });
                    }
                    1086735552 => {
                        // AttentionUser
                        let attcllist = self.parse_pio(&path!(
                            actor_dir / "AttClientList" / format!("{}.batcllist", v)
                        ))?;
                        for atcl in attcllist.list("AttClients").unwrap().objects.values() {
                            if let aamp::Parameter::String64(filename) =
                                atcl.param("FileName").unwrap()
                            {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let atcl_path = format!("AttClient/{}.batcl", filename);
                                files.push(SarcEntry {
                                    data: self
                                        .parse_pio(&path!(actor_dir / &atcl_path))?
                                        .to_binary()
                                        .unwrap(),
                                    name: Some(format!("Actor/{}", &atcl_path)),
                                });
                            }
                        }
                        files.push(SarcEntry {
                            data: attcllist.to_binary().unwrap(),
                            name: Some(format!("Actor/AttClientList/{}.batcllist", v)),
                        });
                    }
                    4022948047 => {
                        // RgConfigListUser
                        let rglist = self.parse_pio(&path!(
                            actor_dir / "RagdollConfigList" / format!("{}.brgconfiglist", v)
                        ))?;
                        for impulse in rglist.list("ImpulseParamList").unwrap().objects.values() {
                            if let aamp::Parameter::String64(filename) =
                                impulse.param("FileName").unwrap()
                            {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let impulse_path = format!("RagdollConfig/{}.brgconfig", filename);
                                files.push(SarcEntry {
                                    data: self
                                        .parse_pio(&path!(actor_dir / &impulse_path))?
                                        .to_binary()
                                        .unwrap(),
                                    name: Some(format!("Actor/{}", &impulse_path)),
                                });
                            }
                        }
                        files.push(SarcEntry {
                            data: rglist.to_binary().unwrap(),
                            name: Some(format!("Actor/RagdollConfigList/{}.brgconfiglist", v)),
                        });
                    }
                    2366604039 => {
                        // PhysicsUser
                        let physics_source = path!(self.input / self.content / "Physics");
                        let physics = self
                            .parse_pio(&path!(actor_dir / "Physics" / format!("{}.bphysics", v)))?;
                        let types = &physics.list("ParamSet").unwrap().objects[1258832850];
                        if let aamp::Parameter::Bool(use_ragdoll) =
                            types.param("use_ragdoll").unwrap()
                        {
                            if *use_ragdoll {
                                if let aamp::Parameter::String256(rg_path) = physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .object("Ragdoll")
                                    .unwrap()
                                    .param("ragdoll_setup_file_path")
                                    .unwrap()
                                {
                                    files.push(SarcEntry {
                                        name: Some(format!("Physics/Ragdoll/{}", rg_path)),
                                        data: read(path!(physics_source / "Ragdoll" / rg_path))?,
                                    });
                                }
                            }
                        }
                        if let aamp::Parameter::Bool(use_support) =
                            types.param("use_support_bone").unwrap()
                        {
                            if *use_support {
                                if let aamp::Parameter::String256(support_path) = physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .object("SupportBone")
                                    .unwrap()
                                    .param("support_bone_setup_file_path")
                                    .unwrap()
                                {
                                    files.push(SarcEntry {
                                        name: Some(format!("Physics/SupportBone/{}", support_path)),
                                        data: read(path!(
                                            physics_source / "SupportBone" / support_path
                                        ))?,
                                    });
                                }
                            }
                        }
                        if let aamp::Parameter::Bool(use_cloth) = types.param("use_cloth").unwrap()
                        {
                            if *use_cloth {
                                if let aamp::Parameter::String256(cloth_path) = physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .object("ClothHeader")
                                    .unwrap()
                                    .param("cloth_setup_file_path")
                                    .unwrap()
                                {
                                    files.push(SarcEntry {
                                        name: Some(format!("Physics/Cloth/{}", cloth_path)),
                                        data: read(path!(physics_source / "Cloth" / cloth_path))?,
                                    });
                                }
                            }
                        }
                        if let aamp::Parameter::Int(rigid_num) =
                            types.param("use_rigid_body_set_num").unwrap()
                        {
                            if *rigid_num > 0 {
                                for rigid in physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .list("RigidBodySet")
                                    .unwrap()
                                    .lists
                                    .values()
                                {}
                            }
                        }
                    }
                    _ => continue,
                };
            }
        }

        self.actors.push(Actor {
            name: String::from(link.file_name().unwrap().to_string_lossy()),
            pack: SarcFile {
                byte_order: if self.be {
                    sarc::Endian::Big
                } else {
                    sarc::Endian::Little
                },
                files,
            },
        });
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
