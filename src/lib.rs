mod sarc_ext;
use aamp::*;
use botw_utils::extensions::*;
use botw_utils::hashes::{Platform, StockHashTable};
use byml::Byml;
use crc::crc32;
use glob::glob;
use path_macro::path;
use path_slash::PathExt;
use pyo3::exceptions::*;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::wrap_pyfunction;
use rayon::prelude::*;
use sarc::SarcEntry;
use sarc_ext::{SarcFile, SarcFileExt};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use yaz0::Yaz0Writer;

type AnyError = dyn Error + Send + Sync;
type GeneralResult<T> = Result<T, Box<AnyError>>;

static TITLE_ACTORS: [&str; 58] = [
    "AncientArrow",
    "Animal_Insect_A",
    "Animal_Insect_B",
    "Animal_Insect_F",
    "Animal_Insect_H",
    "Animal_Insect_M",
    "Animal_Insect_S",
    "Animal_Insect_X",
    "Armor_Default_Extra_00",
    "Armor_Default_Extra_01",
    "BombArrow_A",
    "BrightArrow",
    "BrightArrowTP",
    "CarryBox",
    "DemoXLinkActor",
    "Dm_Npc_Gerudo_HeroSoul_Kago",
    "Dm_Npc_Goron_HeroSoul_Kago",
    "Dm_Npc_RevivalFairy",
    "Dm_Npc_Rito_HeroSoul_Kago",
    "Dm_Npc_Zora_HeroSoul_Kago",
    "ElectricArrow",
    "ElectricWaterBall",
    "EventCameraRumble",
    "EventControllerRumble",
    "EventMessageTransmitter1",
    "EventSystemActor",
    "Explode",
    "Fader",
    "FireArrow",
    "FireRodLv1Fire",
    "FireRodLv2Fire",
    "FireRodLv2FireChild",
    "GameROMPlayer",
    "IceArrow",
    "IceRodLv1Ice",
    "IceRodLv2Ice",
    "Item_Conductor",
    "Item_Magnetglove",
    "Item_Material_01",
    "Item_Material_03",
    "Item_Material_07",
    "Item_Ore_F",
    "NormalArrow",
    "Obj_IceMakerBlock",
    "Obj_SupportApp_Wind",
    "PlayerShockWave",
    "PlayerStole2",
    "RemoteBomb",
    "RemoteBomb2",
    "RemoteBombCube",
    "RemoteBombCube2",
    "SceneSoundCtrlTag",
    "SoundTriggerTag",
    "TerrainCalcCenterTag",
    "ThunderRodLv1Thunder",
    "ThunderRodLv2Thunder",
    "ThunderRodLv2ThunderChild",
    "WakeBoardRope",
];

#[derive(Debug, Clone)]
struct AampKeyError(String);

unsafe impl Send for AampKeyError {}
unsafe impl Sync for AampKeyError {}

impl Error for AampKeyError {}

impl std::fmt::Display for AampKeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Cannot find key {} in AAMP object", self.0)
    }
}

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
    pub name: String,
    pub pack: SarcFile,
}

impl Actor {
    fn get_info(&self) -> BTreeMap<String, Byml> {
        let info: BTreeMap<String, Byml> = BTreeMap::new();
        info
    }

    // fn get_params(&self, ext: &str) -> Option<ParameterIO> {
    //     if let Some(file) = self
    //         .pack
    //         .files
    //         .iter()
    //         .find(|f| f.name.is_some() && f.name.as_ref().unwrap().ends_with(ext))
    //     {
    //         let mut reader = Cursor::new(&file.data);
    //         if let Ok(pio) = ParameterIO::from_binary(&mut reader) {
    //             return Some(pio);
    //         }
    //     }
    //     None
    // }
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
        for line in fs::read_to_string(path!(input / ".done"))?
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
        titles: TITLE_ACTORS
            .iter()
            .map(|x| x.to_owned())
            .chain(args.title_actors.split(','))
            .into_iter()
            .map(|x| x.to_owned())
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
    match builder.build() {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{}", e);
            Ok(())
        }
    }
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
    fn warn(&self, msg: &str) -> GeneralResult<()> {
        if self.strict {
            Err(Box::<AnyError>::from(msg.to_owned()))
        } else {
            if self.warn {
                println!("{}", msg);
            }
            Ok(())
        }
    }

    fn vprint<S: AsRef<str>>(&self, msg: S) -> () {
        if self.verbose {
            println!("{}", msg.as_ref());
        }
    }

    fn parse_pio(&self, file: &PathBuf) -> GeneralResult<ParameterIO> {
        match file.extension().unwrap().to_str().unwrap() {
            "yml" => match ParameterIO::from_text(&fs::read_to_string(file)?) {
                Ok(pio) => Ok(pio),
                Err(e) => Err(Box::from(format!(
                    "Could not parse {}, error {:?}",
                    file.to_string_lossy(),
                    e
                ))),
            },
            _ => {
                let mut fo = fs::File::open(file)?;
                match ParameterIO::from_binary(&mut fo) {
                    Ok(pio) => Ok(pio),
                    Err(e) => Err(Box::from(format!(
                        "Could not parse {}, error {:?}",
                        file.to_string_lossy(),
                        e
                    ))),
                }
            }
        }
    }

    fn parse_actor(&self, link: &PathBuf) -> GeneralResult<Option<Actor>> {
        let yml = fs::read_to_string(link)?;
        let pio: ParameterIO = match ParameterIO::from_text(&yml) {
            Ok(pio) => pio,
            Err(e) => return Err(Box::from(format!("{:?}", e))),
        };
        let actor_dir = path!(self.input / self.content / "Actor");

        let mut file_map: HashMap<String, PathBuf> = HashMap::new();
        for (k, v) in pio
            .object("LinkTarget")
            .ok_or(format!("No LinkTarget found in {:?}", link))?
            .params()
            .iter()
        {
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
                    1086735552 => "AttClientList/{}.batcllist",
                    4022948047 => "RagdollConfigList/{}.brgconfiglist",
                    2366604039 => "Physics/{}.bphysics",
                    _ => continue,
                }
                .replace("{}", &v);
                file_map.insert(
                    format!("Actor/{}", &param_path),
                    path!(actor_dir / (param_path + ".yml")),
                );
                match k {
                    110127898 => {
                        // ASUser
                        let aslist = self.parse_pio(&path!(
                            actor_dir / "ASList" / format!("{}.baslist.yml", v)
                        ))?;
                        for anim in aslist.list("ASDefines").unwrap().objects.values() {
                            if let Parameter::String64(filename) = anim.param("Filename").unwrap() {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let as_path = format!("AS/{}.bas", filename);
                                file_map.insert(
                                    format!("Actor/{}", &as_path),
                                    path!(actor_dir / (as_path + ".yml")),
                                );
                            }
                        }
                    }
                    1086735552 => {
                        // AttentionUser
                        let attcllist = self.parse_pio(&path!(
                            actor_dir / "AttClientList" / format!("{}.batcllist.yml", v)
                        ))?;
                        for atcl in attcllist.list("AttClients").unwrap().objects.values() {
                            if let Parameter::String64(filename) = atcl.param("FileName").unwrap() {
                                if filename == "Dummy" {
                                    continue;
                                }
                                let atcl_path = format!("AttClient/{}.batcl", filename);
                                file_map.insert(
                                    format!("Actor/{}", &atcl_path),
                                    path!(actor_dir / (atcl_path + ".yml")),
                                );
                            }
                        }
                    }
                    4022948047 => {
                        // RgConfigListUser
                        let rglist = self.parse_pio(&path!(
                            actor_dir / "RagdollConfigList" / format!("{}.brgconfiglist.yml", v)
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
                                    path!(actor_dir / (impulse_path + ".yml")),
                                );
                            }
                        }
                    }
                    2366604039 => {
                        // PhysicsUser
                        let physics_source = path!(self.input / self.content / "Physics");
                        let physics = self.parse_pio(&path!(
                            actor_dir / "Physics" / format!("{}.bphysics.yml", v)
                        ))?;
                        let types = &physics
                            .list("ParamSet")
                            .ok_or_else(|| AampKeyError("ParamSet".to_owned()))?
                            .objects
                            .get(&1258832850u32)
                            .ok_or_else(|| AampKeyError("1258832850".to_owned()))?;
                        if let Parameter::Bool(use_ragdoll) = types.param("use_ragdoll").unwrap() {
                            if *use_ragdoll {
                                if let Parameter::String256(rg_path) = physics
                                    .list("ParamSet")
                                    .ok_or_else(|| AampKeyError("ParamSet".to_owned()))?
                                    .object("Ragdoll")
                                    .ok_or_else(|| AampKeyError("Ragdoll".to_owned()))?
                                    .param("ragdoll_setup_file_path")
                                    .ok_or_else(|| {
                                        AampKeyError("ragdoll_setup_file_path".to_owned())
                                    })?
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
                                    .ok_or_else(|| AampKeyError("ParamSet".to_owned()))?
                                    .object("SupportBone")
                                    .ok_or_else(|| AampKeyError("SupportBone".to_owned()))?
                                    .param("support_bone_setup_file_path")
                                    .ok_or_else(|| {
                                        AampKeyError("support_bone_setup_file_path".to_owned())
                                    })?
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
                                    .ok_or_else(|| AampKeyError("ParamSet".to_owned()))?
                                    .list("Cloth")
                                    .ok_or_else(|| AampKeyError("Cloth".to_owned()))?
                                    .object("ClothHeader")
                                    .ok_or_else(|| AampKeyError("ClothHeader".to_owned()))?
                                    .param("cloth_setup_file_path")
                                    .ok_or_else(|| {
                                        AampKeyError("cloth_setup_file_path".to_owned())
                                    })?
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
                                    .ok_or_else(|| AampKeyError("ParamSet".to_owned()))?
                                    .list("RigidBodySet")
                                    .ok_or_else(|| AampKeyError("RigidBodySet".to_owned()))?
                                    .lists
                                    .values()
                                {
                                    if let Some(setup_path_param) = rigid
                                        .objects
                                        .get(&4288596824)
                                        .ok_or_else(|| AampKeyError("4288596824".to_owned()))?
                                        .param("setup_file_path")
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

        if self.fresh_files.contains(&link)
            || file_map.iter().any(|(_, v)| self.fresh_files.contains(&v))
        {
            Ok(Some(Actor {
                name: link
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .replace(".bxml", ""),
                pack: SarcFile {
                    byte_order: if self.be {
                        sarc::Endian::Big
                    } else {
                        sarc::Endian::Little
                    },
                    files: file_map
                        .iter()
                        .map(|(k, v)| -> GeneralResult<Option<SarcEntry>> {
                            let ext = &v.extension().unwrap().to_string_lossy();
                            let bytes = if ext == "yml" {
                                let sub_ext = format!(
                                    ".{}",
                                    v.with_extension("").extension().unwrap().to_string_lossy(),
                                );
                                if AAMP_EXTS.contains(&sub_ext.as_str()) {
                                    ParameterIO::from_text(&fs::read_to_string(&v).unwrap())
                                        .unwrap()
                                        .to_binary()
                                        .unwrap()
                                } else if BYML_EXTS.contains(&sub_ext.as_str()) {
                                    Byml::from_text(&fs::read_to_string(&v).unwrap())
                                        .unwrap()
                                        .to_binary(byml::Endian::Big, 2)
                                        .unwrap()
                                } else {
                                    fs::read(&v).unwrap()
                                }
                            } else {
                                match fs::read(&v).map_err(|_| {
                                    Box::<AnyError>::from(format!(
                                        "Cannot read file {}",
                                        v.to_string_lossy()
                                    ))
                                }) {
                                    Ok(data) => data,
                                    Err(e) => {
                                        if ext.starts_with("hk") {
                                            self.warn(&format!("{:?}", e))?;
                                            return Ok(None);
                                        } else {
                                            return Err(e);
                                        }
                                    }
                                }
                            };
                            Ok(Some(SarcEntry {
                                name: Some(k.to_owned()),
                                data: bytes,
                            }))
                        })
                        .filter_map(|f| match f {
                            Ok(_) => f.transpose(),
                            Err(e) => Some(Err(e)),
                        })
                        .collect::<GeneralResult<Vec<SarcEntry>>>()?,
                },
            }))
        } else {
            Ok(None)
        }
    }

    fn save_times(&mut self) -> GeneralResult<()> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for file in &self.fresh_files {
            self.file_times.insert(file.to_owned(), time);
        }
        fs::write(
            path!(self.input / ".done"),
            &self
                .file_times
                .iter()
                .map(|(f, t)| {
                    format!(
                        "{},{}\n",
                        match f.strip_prefix(&self.input) {
                            Ok(path) => path,
                            Err(_) => f,
                        }
                        .to_string_lossy(),
                        t
                    )
                })
                .collect::<String>(),
        )
        .unwrap();
        Ok(())
    }

    fn sort_files(&mut self) -> GeneralResult<()> {
        self.all_files = glob(&path!(self.input / "**" / "*.*").to_string_lossy())
            .expect("Weird, a glob error")
            .filter_map(|x| {
                if let Ok(path) = x {
                    if path.is_file()
                        && !path
                            .components()
                            .map(|c| c.as_os_str().to_string_lossy())
                            .any(|c| c == "build" || c.starts_with('.'))
                    {
                        return Some(path);
                    }
                }
                None
            })
            .collect();
        for file in &self.all_files {
            let modified = fs::metadata(&file).unwrap().modified().unwrap();
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
        if self.fresh_files.is_empty() {
            Err(box_any_error("No files need building"))
        } else {
            Ok(())
        }
    }

    fn build_actorinfo(&mut self) -> GeneralResult<()> {
        let actorinfo_dir = path!(&self.input / &self.content / "Actor" / "ActorInfo");
        let is_info = |f: &PathBuf| {
            f.file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with(".info.yml")
        };
        if actorinfo_dir.exists()
            && (self.fresh_files.par_iter().any(is_info) || !self.actors.is_empty())
        {
            println!("Building actor info...");
            let modded_actors: Vec<String> =
                self.actors.iter().map(|a| a.name.to_owned()).collect();
            let mut actorinfo: BTreeMap<String, Byml> = BTreeMap::new();
            let actorlist: Arc<Mutex<Vec<Byml>>> = Arc::new(Mutex::new(Vec::new()));
            glob(&path!(actorinfo_dir / "*.yml").to_string_lossy())
                .expect("Weird, a glob error")
                .filter_map(|f| f.ok())
                .collect::<Vec<PathBuf>>()
                .par_iter()
                .map(|f| -> GeneralResult<()> {
                    if let Byml::Hash(mut info) = Byml::from_text(&fs::read_to_string(&f)?)
                        .map_err(|e| Box::<AnyError>::from(format!("{}", e)))?
                    {
                        let actor_name = info["name"].as_string()?.clone();
                        if modded_actors.contains(&actor_name) {
                            info.extend(
                                self.actors
                                    .iter()
                                    .find(|a| a.name == actor_name)
                                    .expect("Weird")
                                    .get_info(),
                            )
                        }
                        actorlist.lock().unwrap().push(Byml::Hash(info));
                    }
                    // let mut info: BTreeMap<String, Byml> = std::mem::take(&mut byml);
                    Ok(())
                })
                .collect::<GeneralResult<()>>()?;
            let mut actorlist = actorlist.lock().unwrap().to_owned();
            let mut hashlist: BTreeSet<u32> = BTreeSet::new();
            actorlist.sort_by_key(|a| {
                let name = a.as_hash().unwrap()["name"].as_string().unwrap();
                let hash = crc32::checksum_ieee(name.as_bytes());
                hashlist.insert(hash);
                hash
            });
            actorinfo.insert("Actors".to_owned(), Byml::Array(actorlist));
            actorinfo.insert(
                "Hashes".to_owned(),
                Byml::Array(
                    hashlist
                        .par_iter()
                        .map(|h| {
                            if h < &2147483648 {
                                Byml::Int(*h as i32)
                            } else {
                                Byml::UInt(*h)
                            }
                        })
                        .collect(),
                ),
            );
            fs::create_dir_all(path!(&self.output / &self.content / "Actor"))?;
            fs::write(
                path!(&self.output / &self.content / "Actor" / "ActorInfo.product.sbyml"),
                Byml::Hash(actorinfo).to_compressed_binary(
                    if self.be {
                        byml::Endian::Big
                    } else {
                        if actorinfo_dir.exists() {
                            println!("Building actor info...");
                            let modded_actors: Vec<String> =
                                self.actors.iter().map(|a| a.name.to_owned()).collect();
                            let mut actorinfo: BTreeMap<String, Byml> = BTreeMap::new();
                            let actorlist: Arc<Mutex<Vec<Byml>>> = Arc::new(Mutex::new(Vec::new()));
                            glob(&path!(actorinfo_dir / "*.yml").to_string_lossy())
                                .expect("Weird, a glob error")
                                .filter_map(|f| f.ok())
                                .collect::<Vec<PathBuf>>()
                                .par_iter()
                                .map(|f| -> GeneralResult<()> {
                                    if let Byml::Hash(mut info) =
                                        Byml::from_text(&fs::read_to_string(&f)?)
                                            .map_err(|e| Box::<AnyError>::from(format!("{}", e)))?
                                    {
                                        let actor_name = info["name"].as_string()?.clone();
                                        if modded_actors.contains(&actor_name) {
                                            info.extend(
                                                self.actors
                                                    .iter()
                                                    .find(|a| a.name == actor_name)
                                                    .expect("Weird")
                                                    .get_info(),
                                            )
                                        }
                                        actorlist.lock().unwrap().push(Byml::Hash(info));
                                    }
                                    // let mut info: BTreeMap<String, Byml> = std::mem::take(&mut byml);
                                    Ok(())
                                })
                                .collect::<GeneralResult<()>>()?;
                            let mut actorlist = actorlist.lock().unwrap().to_owned();
                            let mut hashlist: BTreeSet<u32> = BTreeSet::new();
                            actorlist.sort_by_key(|a| {
                                let name = a.as_hash().unwrap()["name"].as_string().unwrap();
                                let hash = crc32::checksum_ieee(name.as_bytes());
                                hashlist.insert(hash);
                                hash
                            });
                            actorinfo.insert("Actors".to_owned(), Byml::Array(actorlist));
                            actorinfo.insert(
                                "Hashes".to_owned(),
                                Byml::Array(
                                    hashlist
                                        .par_iter()
                                        .map(|h| {
                                            if h < &2147483648 {
                                                Byml::Int(*h as i32)
                                            } else {
                                                Byml::UInt(*h)
                                            }
                                        })
                                        .collect(),
                                ),
                            );
                            fs::create_dir_all(path!(&self.output / &self.content / "Actor"))?;
                            fs::write(
                                path!(
                                    &self.output
                                        / &self.content
                                        / "Actor"
                                        / "ActorInfo.product.sbyml"
                                ),
                                Byml::Hash(actorinfo).to_compressed_binary(
                                    if self.be {
                                        byml::Endian::Big
                                    } else {
                                        byml::Endian::Little
                                    },
                                    2,
                                )?,
                            )?;
                        }
                        byml::Endian::Little
                    },
                    2,
                )?,
            )?;
        }
        Ok(())
    }

    fn build_actors(&mut self) -> GeneralResult<()> {
        println!("Building actors...");
        self.actors
            .par_iter()
            .filter(|a| !self.titles.contains(&a.name))
            .map(|a| {
                let out = path!(
                    &self.output
                        / &self.content
                        / "Actor"
                        / "Pack"
                        / format!("{}.sbactorpack", &a.name)
                );
                fs::create_dir_all(out.parent().unwrap())?;
                write_yaz0_sarc_to_file(&a.pack, &out).map_err(|e| {
                    Box::<AnyError>::from(format!("Error {:?} writing actor pack {}", e, &a.name))
                })?;
                Ok(())
            })
            .collect::<GeneralResult<()>>()?;

        let title_actors: Vec<&Actor> = self
            .actors
            .par_iter()
            .filter(|a| self.titles.contains(&a.name))
            .collect();
        if !title_actors.is_empty() {
            let title_path = path!(&self.output / &self.content / "Pack" / "TitleBG.pack");
            let mut sarc = if title_path.exists() {
                SarcFile::read_from_file(&title_path).map_err(box_any_error)?
            } else {
                SarcFile {
                    byte_order: if self.be {
                        sarc::Endian::Big
                    } else {
                        sarc::Endian::Little
                    },
                    files: vec![],
                }
            };
            sarc.add_entries(
                &title_actors
                    .into_par_iter()
                    .map(|a| {
                        self.vprint(format!("Building actor {}", &a.name));
                        let mut bytes: Vec<u8> = vec![];
                        a.pack.write_yaz0(&mut bytes).map_err(box_any_error)?;
                        let actor = SarcEntry {
                            name: Some(format!("Actor/Pack/{}.sbactorpack", a.name)),
                            data: bytes,
                        };
                        self.vprint(format!("Built actor {}", &a.name));
                        Ok(actor)
                    })
                    .collect::<GeneralResult<Vec<SarcEntry>>>()?,
            );
            fs::create_dir_all(title_path.parent().unwrap())?;
            sarc.write_to_file(&title_path).map_err(box_any_error)?;
        }
        Ok(())
    }

    fn build_yaml(&mut self) -> GeneralResult<()> {
        println!("Building misc YAML files...");
        let actorpath = path!(&self.input / &self.content / "Actor");
        self.yml_files
            .par_iter()
            .filter(|f| {
                self.fresh_files.contains(&f)
                    && !(f.ancestors().map(|p| p.to_owned()).any(|x| x == actorpath)
                        || f.file_name().unwrap().to_string_lossy() == "config.yml")
            })
            .map(|f| {
                self.vprint(format!("Building {}", f.to_slash_lossy()));
                let ext = format!(
                    ".{}",
                    f.with_extension("")
                        .extension()
                        .unwrap_or_else(|| panic!(
                            "File {} missing extension",
                            &f.as_os_str().to_string_lossy()
                        ))
                        .to_string_lossy()
                );
                let out = path!(&self.output / f.strip_prefix(&self.input)?.with_extension(""));
                fs::create_dir_all(out.parent().unwrap())?;
                if BYML_EXTS.contains(&ext.as_str()) {
                    let byml = Byml::from_text(&fs::read_to_string(&f)?).map_err(box_any_error)?;
                    let endian = if self.be {
                        byml::Endian::Big
                    } else {
                        byml::Endian::Little
                    };
                    fs::write(
                        out,
                        if ext.starts_with(".s") {
                            byml.to_compressed_binary(endian, 2)?
                        } else {
                            byml.to_binary(endian, 2)?
                        },
                    )?;
                } else if AAMP_EXTS.contains(&ext.as_str()) {
                    fs::write(out, self.parse_pio(&f)?.to_binary().map_err(box_any_error)?)?;
                }
                Ok(())
            })
            .collect::<GeneralResult<()>>()?;
        Ok(())
    }

    fn add_folder_to_sarc(
        &self,
        root: &PathBuf,
        dir: &PathBuf,
        sarc: &mut SarcFile,
    ) -> GeneralResult<()> {
        for item in glob(path!(dir / "*").to_string_lossy().as_ref())
            .expect("Weird, a glob error")
            .filter_map(|f| f.ok())
        {
            if item.is_file() {
                if !self.fresh_files.contains(&item) {
                    continue;
                }
                let bytes = if &item.extension().unwrap().to_string_lossy() == "yml" {
                    let sub_ext = format!(
                        ".{}",
                        item.with_extension("")
                            .extension()
                            .unwrap()
                            .to_string_lossy(),
                    );
                    if AAMP_EXTS.contains(&sub_ext.as_str()) {
                        ParameterIO::from_text(&fs::read_to_string(&item).unwrap())
                            .unwrap()
                            .to_binary()
                            .unwrap()
                    } else if BYML_EXTS.contains(&sub_ext.as_str()) {
                        Byml::from_text(&fs::read_to_string(&item).unwrap())
                            .unwrap()
                            .to_binary(
                                if self.be {
                                    byml::Endian::Big
                                } else {
                                    byml::Endian::Little
                                },
                                2,
                            )
                            .unwrap()
                    } else {
                        fs::read(&item)?
                    }
                } else {
                    fs::read(&item)?
                };
                sarc.add_file(item.strip_prefix(root)?.to_slash_lossy().as_str(), &bytes);
            } else if item.is_dir() {
                if let Some(ext) = item.extension() {
                    if SARC_EXTS.contains(&format!(".{}", ext.to_string_lossy()).as_str()) {
                        let name: String = item.strip_prefix(root)?.to_slash_lossy().into();
                        sarc.add_file(
                            name.as_str(),
                            self.build_sarc(
                                &item,
                                if let Some(entry) =
                                    sarc.files.iter().find(|e| e.name == Some(name.clone()))
                                {
                                    SarcFile::read(&entry.data).map_err(box_any_error)?
                                } else {
                                    SarcFile {
                                        byte_order: if self.be {
                                            sarc::Endian::Big
                                        } else {
                                            sarc::Endian::Little
                                        },
                                        files: vec![],
                                    }
                                },
                            )?
                            .as_slice(),
                        );
                        continue;
                    }
                }
                self.add_folder_to_sarc(root, &item, sarc)?;
            }
        }
        Ok(())
    }

    fn build_sarc(&self, f: &PathBuf, mut sarc: SarcFile) -> GeneralResult<Vec<u8>> {
        self.vprint(format!("Building {}", f.to_slash_lossy()));
        self.add_folder_to_sarc(f, f, &mut sarc)?;
        if sarc.files.is_empty() {
            return Ok(vec![]);
        }
        let mut data: Vec<u8> = vec![];
        sarc.write(&mut data)?;
        let ext = f.extension().unwrap().to_string_lossy();
        if ext.starts_with(".s") && ext != "sarc" {
            let mut compressed: Vec<u8> = vec![];
            let ywrite = Yaz0Writer::new(&mut compressed);
            ywrite.compress_and_write(
                &data.as_slice(),
                yaz0::CompressionLevel::Lookahead { quality: 10 },
            )?;
            Ok(compressed)
        } else {
            Ok(data)
        }
    }

    fn build_sarcs(&mut self) -> GeneralResult<()> {
        println!("Loading SARCs to build...");
        let sarcs = glob(path!(&self.input / "**" / "*").to_str().unwrap())
            .expect("Weird, a glob error")
            .filter_map(|d| {
                if let Ok(path) = d {
                    if path.is_dir()
                        && !path.starts_with(path!(&self.input / &self.content / "Actor" / "Pack"))
                        && nest_level(&path) == 0
                    {
                        if let Some(ext) = path.extension() {
                            let ext = format!(".{}", ext.to_string_lossy());
                            if SARC_EXTS.contains(&ext.as_str())
                                && self.fresh_files.par_iter().any(|f| f.starts_with(&path))
                            {
                                return Some(path);
                            }
                        }
                    }
                }
                None
            })
            .collect::<Vec<PathBuf>>();

        println!("Building SARCs...");
        sarcs
            .par_iter()
            .map(|f| {
                let out = path!(&self.output / f.strip_prefix(&self.input)?);
                fs::create_dir_all(out.parent().unwrap())?;
                fs::write(
                    &out,
                    self.build_sarc(
                        f,
                        if !out.exists() {
                            SarcFile {
                                byte_order: if self.be {
                                    sarc::Endian::Big
                                } else {
                                    sarc::Endian::Little
                                },
                                files: vec![],
                            }
                        } else {
                            println!("{}", out.to_string_lossy());
                            SarcFile::read_from_file(&out).map_err(box_any_error)?
                        },
                    )?,
                )?;
                Ok(())
            })
            .collect::<GeneralResult<()>>()?;

        self.other_files.retain(|f| nest_level(f) < 1);
        self.yml_files.retain(|f| nest_level(f) < 1);
        Ok(())
    }

    fn build(&mut self) -> GeneralResult<()> {
        self.sort_files()?;

        println!("Loading actors to build...");
        self.actors.extend(
            glob(
                &path!(self.input / self.content / "Actor" / "ActorLink" / "*.bxml.yml")
                    .to_string_lossy(),
            )
            .expect("Weird, a glob error")
            .filter_map(|f| f.ok())
            .collect::<Vec<PathBuf>>()
            .par_iter()
            .filter(|f| f.to_string_lossy().contains("ActorLink"))
            .filter_map(|f| self.parse_actor(f).transpose())
            .collect::<GeneralResult<Vec<Actor>>>()?,
        );

        self.build_actorinfo()?;
        self.build_actors()?;
        self.build_sarcs()?;
        self.build_yaml()?;

        println!("Copying miscellaneous files...");
        self.other_files
            .par_iter()
            .map(|f| {
                if self.fresh_files.contains(f) {
                    let out = path!(&self.output / f.strip_prefix(&self.input).unwrap());
                    fs::create_dir_all(out.parent().unwrap())?;
                    fs::copy(f, out)
                        .map_err(|e| Box::from(format!("{:?} at {:?}", e, f)))
                        .map(|_| ())
                } else {
                    Ok(())
                }
            })
            .collect::<GeneralResult<()>>()?;

        if !self.meta.is_empty() {
            let text = format!(
                "[Definition]
titleIds = 00050000101C9300,00050000101C9400,00050000101C9500
{}version = 4",
                self.meta
                    .iter()
                    .map(|(k, v)| format!("{} = {}\n", k, v))
                    .collect::<String>()
            );
            fs::write(path!(&self.output / "rules.txt"), text)?;
        }

        self.save_times()?;
        println!("Mod built successfully!");
        Ok(())
    }
}

fn nest_level<P: AsRef<Path>>(file: P) -> usize {
    let file = file.as_ref();
    file.ancestors()
        .filter(|d| {
            if d == &file {
                return false;
            }
            if let Some(ext) = d.extension() {
                SARC_EXTS.contains(&format!(".{}", ext.to_string_lossy()).as_str())
            } else {
                false
            }
        })
        .count()
}

fn write_yaz0_sarc_to_file<P: AsRef<Path>>(sarc: &SarcFile, path: P) -> GeneralResult<()> {
    let mut file = fs::File::create(path.as_ref())?;
    let writer = Yaz0Writer::new(&mut file);
    let mut temp = vec![];
    sarc.write(&mut temp)?;
    writer
        .compress_and_write(&temp, yaz0::CompressionLevel::Lookahead { quality: 10 })
        .map_err(Box::from)
}

fn box_any_error<D: std::fmt::Debug>(error: D) -> Box<AnyError> {
    Box::<AnyError>::from(format!("{:?}", error).trim_matches('"'))
}
