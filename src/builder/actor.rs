use crate::util::*;
use anyhow::{Context, Result};
use jstr::jstr;
use path_slash::PathBufExt;
use phf::phf_map;
use roead::{
    aamp::{hash_name, ParamList, ParameterIO},
    sarc::SarcWriter,
    yaz0::compress,
};
use std::{
    fmt::Debug,
    fs,
    path::{Path, PathBuf},
};

pub(crate) static TITLE_ACTORS: &[&str] = &[
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

#[derive(Debug)]
struct Link {
    path: &'static str,
    ext: &'static str,
}

impl Link {
    fn yaml_path(&self, user: &str) -> PathBuf {
        Path::new("Actor")
            .join(self.path)
            .join(jstr!("{user}.{self.ext}.yml"))
    }
}

static ACTOR_LINKS: phf::Map<u32, Link> = phf_map! {
    3293308145u32 => Link {
        path: "AIProgram",
        ext: "baiprog",
    },
    2851261459u32 => Link {
        path: "AISchedule",
        ext: "baischedule",
    },
    1241489578u32 => Link {
        path: "AnimationInfo",
        ext: "baniminfo",
    },
    110127898u32 => Link {
        path: "ASList",
        ext: "baslist",
    },
    1086735552u32 => Link {
        path: "AttClientList",
        ext: "batcllist",
    },
    1767976113u32 => Link {
        path: "Awareness",
        ext: "bawareness",
    },
    713857735u32 => Link {
        path: "BoneControl",
        ext: "bbonectrl",
    },
    2863165669u32 => Link {
        path: "Chemical",
        ext: "bchemical",
    },
    2307148887u32 => Link {
        path: "DamageParam",
        ext: "bdmgparam",
    },
    2189637974u32 => Link {
        path: "DropTable",
        ext: "bdrop",
    },
    619158934u32 => Link {
        path: "GeneralParamList",
        ext: "bgparamlist",
    },
    414149463u32 => Link {
        path: "LifeCondition",
        ext: "blifecondition",
    },
    1096753192u32 => Link {
        path: "LOD",
        ext: "blod",
    },
    3086518481u32 => Link {
        path: "ModelList",
        ext: "bmodellist",
    },
    2366604039u32 => Link {
        path: "Physics",
        ext: "bphysics",
    },
    1292038778u32 => Link {
        path: "RagdollBlendWeight",
        ext: "brgbw",
    },
    4022948047u32 => Link {
        path: "RagdollConfigList",
        ext: "brgconfiglist",
    },
    1589643025u32 => Link {
        path: "Recipe",
        ext: "brecipe",
    },
    2994379201u32 => Link {
        path: "ShopData",
        ext: "bshop",
    },
    3926186935u32 => Link {
        path: "UMii",
        ext: "bumii",
    },
};

pub(crate) struct Actor<'a> {
    builder: &'a super::Builder,
    pub name: String,
    files: Vec<PathBuf>,
}

impl<'a> Debug for Actor<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Actor")
            .field("name", &self.name)
            .field("files", &self.files)
            .finish()
    }
}

impl<'a> Actor<'a> {
    pub(crate) fn new(builder: &'a super::Builder, file: &Path) -> Result<Option<Self>> {
        let actor_link = ParameterIO::from_text(fs::read_to_string(&file)?)?;
        let root = builder.source_content();
        let files: Vec<PathBuf> = actor_link
            .objects
            .get(hash_name("LinkTarget"))
            .context("Actor link missing LinkTarget")?
            .params()
            .iter()
            .filter(|(k, v)| ACTOR_LINKS.contains_key(k) && v.as_string().unwrap() != "Dummy")
            .map(|(k, v)| -> Result<Vec<PathBuf>> {
                let file = root.join(
                    ACTOR_LINKS
                        .get(k)
                        .unwrap()
                        .yaml_path(v.as_string().unwrap()),
                );
                let mut files: Vec<PathBuf> = vec![file.clone()];
                match k {
                    110127898 => {
                        // ASUser
                        files.extend(process_aslist(&file)?);
                    }
                    1086735552 => {
                        // AttentionUser
                        files.extend(process_attcllist(&file)?);
                    }
                    4022948047 => {
                        // RgConfigListUser
                        files.extend(process_rgconfiglist(&file)?);
                    }
                    2366604039 => {
                        // PhysicsUser
                        files.extend(process_physics(&file)?);
                    }
                    _ => {}
                }
                Ok(files)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .chain([file.to_owned()].into_iter())
            .collect();
        if files
            .iter()
            .chain(&[file.to_owned()])
            .any(|f| builder.modified_files.contains(f))
        {
            let name = file
                .with_extension("")
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            builder.vprint(&jstr!("Actor {&name} modified"));
            Ok(Some(Self {
                builder,
                files,
                name,
            }))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn build(self) -> Result<Vec<u8>> {
        self.builder.vprint(&jstr!("Building actor {&self.name}"));
        let mut pack = SarcWriter::new(self.builder.endian());
        let root = self.builder.source.join(&self.builder.content);
        self.files.into_iter().try_for_each(|f| -> Result<()> {
            let mut filename = f.strip_prefix(&root)?.to_owned();
            if get_ext(&filename)? == "yml" {
                filename = filename.with_extension("");
            }
            match self.builder.get_resource_data(&f) {
                Ok(data) => pack.add_file(
                    &filename.to_slash_lossy(),
                    data,
                ),
                Err(e) => {
                    if let Some(err) = e.downcast_ref::<std::io::Error>() {
                        if let std::io::ErrorKind::NotFound = err.kind() {
                            if filename.starts_with("Physics") {
                                self.builder.warn(&(
                                    jstr!("Havok file {&f.to_slash_lossy()} not found for actor {&self.name}.\n")
                                    + "Ignore if intentionally using a file not in the actor pack."))?;
                                return Ok(())
                            }
                        }
                    }
                    return Err(e);
                }
            }
            Ok(())
        })?;
        self.builder.vprint(&jstr!("Built actor {&self.name}"));
        let data = pack.to_binary();
        self.builder
            .set_resource_size(&jstr!("Actor/Pack/{&self.name}.bactorpack"), &data);
        Ok(compress(data))
    }
}

fn process_aslist(aslist_path: &Path) -> Result<Vec<PathBuf>> {
    let aslist = parse_aamp(aslist_path)?;
    let asroot = aslist_path.parent().unwrap().parent().unwrap().join("AS");
    Ok(aslist
        .lists()
        .get(hash_name("ASDefines"))
        .context("AS list missing ASDefines")?
        .objects()
        .0
        .values()
        .filter_map(|def| {
            def.params()
                .get(&hash_name("Filename"))
                .map(|p| p.as_string().ok())
                .flatten()
                .and_then(|p| {
                    if p == "Dummy" {
                        None
                    } else {
                        Some(asroot.join(jstr!("{p}.bas.yml")))
                    }
                })
        })
        .collect())
}

fn process_attcllist(attcllist_path: &Path) -> Result<Vec<PathBuf>> {
    let attcllist = parse_aamp(attcllist_path)?;
    let attclroot = attcllist_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("AttClient");
    Ok(attcllist
        .lists()
        .get(hash_name("AttClients"))
        .context("ATTCL list missing AttClients")?
        .objects()
        .0
        .values()
        .filter_map(|def| {
            def.params()
                .get(&hash_name("FileName"))
                .map(|p| p.as_string().ok())
                .flatten()
                .and_then(|p| {
                    if p == "Dummy" {
                        None
                    } else {
                        Some(attclroot.join(jstr!("{p}.batcl.yml")))
                    }
                })
        })
        .collect())
}

fn process_rgconfiglist(rgconfig_path: &Path) -> Result<Vec<PathBuf>> {
    let rgconfig = parse_aamp(rgconfig_path)?;
    let rgconfig_root = rgconfig_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("RagdollConfig");
    Ok(rgconfig
        .lists()
        .get(hash_name("ImpulseParamList"))
        .context("RgConfig list missing ImpulseParamList")?
        .objects()
        .0
        .values()
        .filter_map(|def| {
            def.params()
                .get(&hash_name("FileName"))
                .map(|p| p.as_string().ok())
                .flatten()
                .and_then(|p| {
                    if p == "Dummy" {
                        None
                    } else {
                        Some(rgconfig_root.join(jstr!("{p}.brgconfig.yml")))
                    }
                })
        })
        .collect())
}

fn process_physics(physics_path: &Path) -> Result<Vec<PathBuf>> {
    let physics = parse_aamp(physics_path)?;
    let physics_root = physics_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Physics");
    let param_set = physics
        .lists
        .get(hash_name("ParamSet"))
        .context("Physics missing ParamSet")?;
    let types = param_set
        .objects
        .get(&1258832850)
        .context("ParamSet missing 1258832850")?;
    let mut files: Vec<PathBuf> = vec![];
    if types
        .params()
        .get(&hash_name("use_ragdoll"))
        .context("Physics missing use_ragdoll")?
        .as_bool()?
    {
        files.push(
            physics_root.join("Ragdoll").join(
                param_set
                    .objects
                    .get(hash_name("Ragdoll"))
                    .context("Physics missing ragdoll")?
                    .0
                    .get(&hash_name("ragdoll_setup_file_path"))
                    .context("Missing ragdoll_setup_file_path")?
                    .as_string()?,
            ),
        )
    }
    if types
        .params()
        .get(&hash_name("use_support_bone"))
        .context("Physics missing use_support_bone")?
        .as_bool()?
    {
        files.push(
            physics_root.join("SupportBone").join(
                param_set
                    .objects
                    .get(hash_name("SupportBone"))
                    .context("Physics missing SupportBone")?
                    .0
                    .get(&hash_name("support_bone_setup_file_path"))
                    .context("Missing support_bone_setup_file_path")?
                    .as_string()?
                    .to_owned()
                    + ".yml",
            ),
        )
    }
    if types
        .params()
        .get(&hash_name("use_cloth"))
        .context("Physics missing use_cloth")?
        .as_bool()?
    {
        files.push(
            physics_root.join("Cloth").join(
                param_set
                    .lists
                    .get(hash_name("Cloth"))
                    .context("Physics missing Cloth")?
                    .objects
                    .get(hash_name("ClothHeader"))
                    .context("Physics missing ClothHeader")?
                    .0
                    .get(&hash_name("cloth_setup_file_path"))
                    .context("Missing cloth_setup_file_path")?
                    .as_string()?,
            ),
        )
    }
    if types
        .0
        .get(&hash_name("use_rigid_body_set_num"))
        .context("Physics missing use_rigid_body_set_num")?
        .as_int()?
        > 0
    {
        files.extend(
            param_set
                .lists
                .get(hash_name("RigidBodySet"))
                .context("Physics missing RigidBodySet")?
                .lists
                .0
                .values()
                .filter_map(|l| {
                    l.objects
                        .get(&4288596824)
                        .context("RigidBody missing 4288596824")
                        .unwrap()
                        .params()
                        .get(&hash_name("setup_file_path"))
                        .and_then(|p| p.as_string().ok())
                        .map(|p| physics_root.join("RigidBody").join(p))
                }),
        )
    }
    Ok(files)
}
