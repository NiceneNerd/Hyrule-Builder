use botw_utils::extensions::*;
use botw_utils::hashes::{Platform, StockHashTable};
// use chrono::prelude::*;
use aamp::*;
use byml::Byml;
use glob::glob;
use path_macro::path;
use pyo3::exceptions::*;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pyfunction;
use rayon::prelude::*;
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
        return Err(PyValueError::new_err("Invalid folders"));
    }
    let mut file_times: HashMap<PathBuf, u64> = HashMap::new();
    if path!(input / ".done").exists() {
        for line in read_to_string(path!(input / ".done"))?
            .split('\n')
            .filter(|x| *x != "")
        {
            let data: Vec<&str> = line.split(',').collect();
            file_times.insert(
                path!(input / PathBuf::from(data[0])),
                str::parse::<u64>(data[1])?,
            );
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
        fresh_files: vec![],
        all_files: vec![],
        yml_files: vec![],
        other_files: vec![],
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
    fresh_files: Vec<PathBuf>,
    all_files: Vec<PathBuf>,
    yml_files: Vec<PathBuf>,
    other_files: Vec<PathBuf>,
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

    fn parse_pio(&self, file: &PathBuf) -> PyResult<ParameterIO> {
        let mut file = File::open(file)?;
        if let Ok(pio) = ParameterIO::from_binary(&mut file) {
            Ok(pio)
        } else {
            Err(PyValueError::new_err("AAMP file {:?} could not be parsed"))
        }
    }

    fn parse_actor(&self, link: &PathBuf) -> PyResult<Option<Actor>> {
        let yml = read_to_string(link)?;
        let pio: ParameterIO = ParameterIO::from_text(&yml).unwrap();
        let actor_dir = path!(self.input / self.content / "Actor");

        let mut file_map: HashMap<String, PathBuf> = HashMap::new();
        for (k, v) in pio.object("LinkTarget").unwrap().params().iter() {
            if let Parameter::StringRef(v) = v {
                if v == "Dummy" {
                    continue;
                }
                let param_path = match k {
                    3293308145 => "AIProgram/{}.baiprog",
                    2851261459 => "AISchedule/{}.baischedule",
                    1241489578 => "AnimationInfo/{}.baniminfo",
                    1767976113 => "Awareness/{}.bawareness",
                    713857735 => "BoneControl/{}.bbonectrl",
                    2863165669 => "Chemical/{}.bchemical",
                    2307148887 => "DamageParam/{}.bdmgparam",
                    2189637974 => "DropTable/{}.bdrop",
                    619158934 => "GeneralParamList/{}.bgparamlist",
                    414149463 => "LifeCondition/{}.blifecondition",
                    1096753192 => "LOD/{}.blod",
                    3086518481 => "ModelList/{}.bmodellist",
                    1292038778 => "RagdollBlendWeight/{}.brgbw",
                    1589643025 => "Recipe/{}.brecipe",
                    2994379201 => "ShopData/{}.bshop",
                    3926186935 => "UMii/{}.bumii",
                    110127898 => "ASList/{}.baslist",
                    1086735552 => "AttClientList/{}.battcllist",
                    4022948047 => "RagdollConfigList/{}.brgconfiglist",
                    2366604039 => "Physics/{}.bphysics",
                    _ => continue,
                }
                .replace("{}", &v);
                file_map.insert(
                    format!("Actor/{}", &param_path),
                    path!(actor_dir / &param_path),
                );
                match k {
                    110127898 => {
                        // ASUser
                        let aslist = self
                            .parse_pio(&path!(actor_dir / "ASList" / format!("{}.baslist", v)))?;
                        for anim in aslist.list("ASDefines").unwrap().objects.values() {
                            if let Parameter::String64(filename) = anim.param("Filename").unwrap() {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let as_path = format!("AS/{}.bas", filename);
                                file_map.insert(
                                    format!("Actor/{}", &as_path),
                                    path!(actor_dir / &as_path),
                                );
                            }
                        }
                    }
                    1086735552 => {
                        // AttentionUser
                        let attcllist = self.parse_pio(&path!(
                            actor_dir / "AttClientList" / format!("{}.batcllist", v)
                        ))?;
                        for atcl in attcllist.list("AttClients").unwrap().objects.values() {
                            if let Parameter::String64(filename) = atcl.param("FileName").unwrap() {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let atcl_path = format!("AttClient/{}.batcl", filename);
                                file_map.insert(
                                    format!("Actor/{}", &atcl_path),
                                    path!(actor_dir / &atcl_path),
                                );
                            }
                        }
                    }
                    4022948047 => {
                        // RgConfigListUser
                        let rglist = self.parse_pio(&path!(
                            actor_dir / "RagdollConfigList" / format!("{}.brgconfiglist", v)
                        ))?;
                        for impulse in rglist.list("ImpulseParamList").unwrap().objects.values() {
                            if let Parameter::String64(filename) =
                                impulse.param("FileName").unwrap()
                            {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let impulse_path = format!("RagdollConfig/{}.brgconfig", filename);
                                file_map.insert(
                                    format!("Actor/{}", &impulse_path),
                                    path!(actor_dir / &impulse_path),
                                );
                            }
                        }
                    }
                    2366604039 => {
                        // PhysicsUser
                        let physics_source = path!(self.input / self.content / "Physics");
                        let physics = self
                            .parse_pio(&path!(actor_dir / "Physics" / format!("{}.bphysics", v)))?;
                        let types = &physics.list("ParamSet").unwrap().objects[1258832850];
                        if let Parameter::Bool(use_ragdoll) = types.param("use_ragdoll").unwrap() {
                            if *use_ragdoll {
                                if let Parameter::String256(rg_path) = physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .object("Ragdoll")
                                    .unwrap()
                                    .param("ragdoll_setup_file_path")
                                    .unwrap()
                                {
                                    file_map.insert(
                                        format!("Physics/Ragdoll/{}", &rg_path),
                                        path!(physics_source / "Ragdoll" / &rg_path),
                                    );
                                }
                            }
                        }
                        if let Parameter::Bool(use_support) =
                            types.param("use_support_bone").unwrap()
                        {
                            if *use_support {
                                if let Parameter::String256(support_path) = physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .object("SupportBone")
                                    .unwrap()
                                    .param("support_bone_setup_file_path")
                                    .unwrap()
                                {
                                    file_map.insert(
                                        format!("Physics/SupportBone/{}", &support_path),
                                        path!(physics_source / "SupportBone" / &support_path),
                                    );
                                }
                            }
                        }
                        if let Parameter::Bool(use_cloth) = types.param("use_cloth").unwrap() {
                            if *use_cloth {
                                if let Parameter::String256(cloth_path) = physics
                                    .list("ParamSet")
                                    .unwrap()
                                    .object("ClothHeader")
                                    .unwrap()
                                    .param("cloth_setup_file_path")
                                    .unwrap()
                                {
                                    file_map.insert(
                                        format!("Physics/Cloth/{}", &cloth_path),
                                        path!(physics_source / "Cloth" / &cloth_path),
                                    );
                                }
                            }
                        }
                        if let Parameter::Int(rigid_num) =
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
                                {
                                    if let Some(setup_path_param) =
                                        rigid.objects[4288596824].param("setup_file_path")
                                    {
                                        if let Parameter::String256(setup_path) = setup_path_param {
                                            let setup_full_path =
                                                path!(physics_source / "RigidBody" / setup_path);
                                            if setup_full_path.exists() {
                                                file_map.insert(
                                                    format!("Physics/RigidBody/{}", &setup_path),
                                                    path!(
                                                        physics_source
                                                            / "RigidBody"
                                                            / &setup_full_path
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
        }

        if file_map.iter().any(|(_, v)| self.fresh_files.contains(v)) {
            Ok(Some(Actor {
                name: String::from(link.file_name().unwrap().to_string_lossy()),
                pack: SarcFile {
                    byte_order: if self.be {
                        sarc::Endian::Big
                    } else {
                        sarc::Endian::Little
                    },
                    files: file_map
                        .iter()
                        .map(|(k, v)| -> PyResult<SarcEntry> {
                            let bytes = if &v.extension().unwrap().to_string_lossy() == "yml" {
                                let sub_ext = format!(
                                    ".{}",
                                    v.with_extension("").extension().unwrap().to_string_lossy(),
                                );
                                if AAMP_EXTS.contains(&sub_ext.as_str()) {
                                    ParameterIO::from_text(&read_to_string(&v).unwrap())
                                        .unwrap()
                                        .to_binary()
                                        .unwrap()
                                } else if BYML_EXTS.contains(&sub_ext.as_str()) {
                                    Byml::from_text(&read_to_string(&v).unwrap())
                                        .unwrap()
                                        .to_binary(byml::Endian::Big, 2)
                                        .unwrap()
                                } else {
                                    read(&v)?
                                }
                            } else {
                                read(&v)?
                            };
                            Ok(SarcEntry {
                                name: Some(k.to_owned()),
                                data: bytes,
                            })
                        })
                        .collect::<PyResult<Vec<SarcEntry>>>()?,
                },
            }))
        } else {
            Ok(None)
        }
    }

    fn build(&mut self) -> PyResult<()> {
        self.all_files = glob(&path!(self.input / "**" / "*").to_string_lossy())
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
        for file in &self.all_files {
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
                self.fresh_files.push(file.to_owned());
            }
            if let Some(ext) = file.extension() {
                if ext.to_string_lossy() == "yml" {
                    self.yml_files.push(file.to_owned());
                    continue;
                }
            }
            self.other_files.push(file.to_owned());
        }

        self.actors.extend(
            self.yml_files
                .clone()
                .par_iter()
                .filter(|f| {
                    f.file_name()
                        .unwrap()
                        .to_os_string()
                        .to_string_lossy()
                        .contains(".bxml")
                })
                .filter_map(|f| self.parse_actor(f).unwrap_or(None))
                .collect::<Vec<Actor>>(),
        );
        println!("{:?}", self.actors);

        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for file in &self.fresh_files {
            self.file_times.insert(file.to_owned(), time);
        }
        self.save_times()?;
        println!("Mod built successfully!");
        Ok(())
    }
}
