# pylint: disable=invalid-name,bare-except,missing-docstring,bad-continuation,import-error
# pylint: disable=unsupported-assignment-operation
import json
import shutil
import sys
from dataclasses import dataclass
from io import BytesIO
from functools import partial
from multiprocessing import Pool
from zlib import crc32

from botw.hashes import StockHashTable
from botw.rstb import guess_aamp_size, guess_bfres_size
import oead
import oead.aamp
from oead.yaz0 import compress
import pymsyt
from rstb import ResourceSizeTable, SizeCalculator

from . import (
    AAMP_EXTS,
    BYML_EXTS,
    EXEC_DIR,
    SARC_EXTS,
    STOCK_FILES,
    RSTB_EXCLUDE_EXTS,
    RSTB_EXCLUDE_NAMES,
    Path,
    get_canon_name,
    is_in_sarc,
)


@dataclass
class BuildParams:
    mod: Path
    out: Path
    content: str
    aoc: str
    be: bool
    guess: bool
    verbose: bool
    titles: set
    table: StockHashTable
    warn: bool
    strict: bool

    def warning(self, msg: str):
        if self.strict:
            raise RuntimeError(f"ERROR: {msg}")
        elif self.warn:
            print(f"WARNING: {msg}")


def _should_rstb(f: Path) -> bool:
    f = f.with_suffix(f.suffix.replace(".s", "."))
    return f.suffix not in RSTB_EXCLUDE_EXTS and f.name not in RSTB_EXCLUDE_NAMES


def load_rstb(be: bool, file: Path = None) -> ResourceSizeTable:
    table = ResourceSizeTable(b"", be=be)
    if not file:
        ver = "wiiu" if be else "switch"
        file = EXEC_DIR / "data" / ver / "rstb.json"
    ref_contents = json.loads(file.read_text(), encoding="utf-8")

    def parse_hash(file: str) -> int:
        try:
            return int(file)
        except ValueError:
            return crc32(file.encode("utf8"))

    table.crc32_map = {parse_hash(k): v for k, v in ref_contents["hash_map"].items()}
    table.name_map = {k: v for k, v in ref_contents["name_map"].items()}
    return table


def _get_rstb_val(name: str, data: bytes, should_guess: bool, be: bool) -> int:
    if not hasattr(_get_rstb_val, "calc"):
        setattr(_get_rstb_val, "calc", SizeCalculator())
    ext = name[name.rindex(".") :]
    if data[:4] == b"Yaz0":
        data = oead.yaz0.decompress(data)
    val = _get_rstb_val.calc.calculate_file_size_with_ext(  # pylint: disable=no-member
        data, wiiu=be, ext=ext
    )
    if ext == ".bdmgparam":
        val += 1000
    elif ext == ".hkrb":
        val += 40
    if val == 0 and should_guess:
        if ext in AAMP_EXTS:
            val = guess_aamp_size(data, be, ext)
        elif ext in {".bfres", ".sbfres"}:
            val = guess_bfres_size(data, be, name)
        elif ext == ".baniminfo":
            val = int(
                (((len(data) + 31) & -32) * (1.5 if len(data) > 36864 else 4))
                + 0xE4
                + 0x24C
            )
            if not be:
                val *= 1.5
    return val


def _copy_file(f: Path, params: BuildParams):
    t: Path = params.out / f.relative_to(params.mod)
    if not t.parent.exists():
        t.parent.mkdir(parents=True, exist_ok=True)
    if is_in_sarc(f):
        shutil.copy(f, t)
    else:
        data = f.read_bytes()
        canon = get_canon_name(f.relative_to(params.mod))
        t.write_bytes(data)
        if params.table.is_file_modded(canon, data) and _should_rstb(f):
            return {canon: _get_rstb_val(t.name, data, params.guess, params.be)}
    return {}


def _build_byml(f: Path, be: bool) -> bytes:
    return bytes(
        oead.byml.to_binary(oead.byml.from_text(f.read_text("utf-8")), big_endian=be)
    )


def _build_aamp(f: Path) -> bytes:
    return bytes(oead.aamp.ParameterIO.from_text(f.read_text("utf-8")).to_binary())


def _build_yml(f: Path, params: BuildParams):
    rv = {}
    try:
        ext = f.with_suffix("").suffix
        t = params.out / f.relative_to(params.mod).with_suffix("")
        if not t.parent.exists():
            t.parent.mkdir(parents=True, exist_ok=True)
        data: bytes
        if ext in BYML_EXTS:
            data = _build_byml(f, params.be)
        elif ext in AAMP_EXTS:
            data = _build_aamp(f)
        t.write_bytes(data if not t.suffix.startswith(".s") else compress(data))
        canon = get_canon_name(t.relative_to(params.out))
        if params.table.is_file_modded(canon, data) and _should_rstb(t):
            return {
                canon: _get_rstb_val(
                    t.name.replace(".s", "."), data, params.guess, params.be
                )
            }
    except Exception as e:  # pylint: disable=broad-except
        raise RuntimeError(
            f"Failed to build {f.relative_to(params.mod).as_posix()}. {e}"
        )
    if params.verbose:
        print(f"Built {f.relative_to(params.mod).as_posix()}")
    return rv


LINK_MAP = {
    3293308145: "AIProgram/*.baiprog",
    2851261459: "AISchedule/*.baischedule",
    1241489578: "AnimationInfo/*.baniminfo",
    1767976113: "Awareness/*.bawareness",
    713857735: "BoneControl/*.bbonectrl",
    2863165669: "Chemical/*.bchemical",
    2307148887: "DamageParam/*.bdmgparam",
    2189637974: "DropTable/*.bdrop",
    619158934: "GeneralParamList/*.bgparamlist",
    414149463: "LifeCondition/*.blifecondition",
    1096753192: "LOD/*.blod",
    3086518481: "ModelList/*.bmodellist",
    1292038778: "RagdollBlendWeight/*.brgbw",
    1589643025: "Recipe/*.brecipe",
    2994379201: "ShopData/*.bshop",
    3926186935: "UMii/*.bumii",
}

TITLE_ACTORS = {
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
}


def _build_actor(link: Path, params: BuildParams):
    pack = oead.SarcWriter(
        endian=oead.Endianness.Big if params.be else oead.Endianness.Little
    )
    actor_name = link.stem
    actor = oead.aamp.ParameterIO.from_binary(link.read_bytes())
    actor_path = params.out / params.content / "Actor"
    targets = actor.objects["LinkTarget"]
    modified = False
    rvs = {}
    try:
        files = {f"Actor/ActorLink/{actor_name}.bxml": link}
        for p, name in targets.params.items():
            name = name.v
            if name == "Dummy":
                continue
            if p.hash in LINK_MAP:
                path = LINK_MAP[p.hash].replace("*", name)
                files["Actor/" + path] = actor_path / path
            elif p == 110127898:  # ASUser
                list_path = actor_path / "ASList" / f"{name}.baslist"
                aslist_bytes = list_path.read_bytes()
                files[f"Actor/ASList/{name}.baslist"] = list_path
                aslist = oead.aamp.ParameterIO.from_binary(aslist_bytes)
                for _, anim in aslist.lists["ASDefines"].objects.items():
                    filename = anim.params["Filename"].v
                    if filename != "Dummy":
                        files[f"Actor/AS/{filename}.bas"] = (
                            actor_path / "AS" / f"{filename}.bas"
                        )
            elif p == 1086735552:  # AttentionUser
                list_path = actor_path / "AttClientList" / f"{name}.batcllist"
                atcllist_bytes = list_path.read_bytes()
                files[f"Actor/AttClientList/{name}.batcllist"] = list_path
                atcllist = oead.aamp.ParameterIO.from_binary(atcllist_bytes)
                for _, atcl in atcllist.lists["AttClients"].objects.items():
                    filename = atcl.params["FileName"].v
                    if filename != "Dummy":
                        files[f"Actor/AttClient/{filename}.batcl"] = (
                            actor_path / "AttClient" / f"{filename}.batcl"
                        )
            elif p == 4022948047:  # RgConfigListUser
                list_path = actor_path / "RagdollConfigList" / f"{name}.brgconfiglist"
                rgconfiglist_bytes = list_path.read_bytes()
                files[f"Actor/RagdollConfigList/{name}.brgconfiglist"] = list_path
                rgconfiglist = oead.aamp.ParameterIO.from_binary(rgconfiglist_bytes)
                for _, impl in rgconfiglist.lists["ImpulseParamList"].objects.items():
                    filename = impl.params["FileName"].v
                    if filename != "Dummy":
                        files[f"Actor/RagdollConfig/{filename}.brgconfig"] = (
                            actor_path / "RagdollConfig" / f"{filename}.brgconfig"
                        )
            elif p == 2366604039:  # PhysicsUser
                phys_source = params.out / params.content / "Physics"
                phys_path = actor_path / "Physics" / f"{name}.bphysics"
                phys_bytes = phys_path.read_bytes()
                files[f"Actor/Physics/{name}.bphysics"] = phys_path
                phys = oead.aamp.ParameterIO.from_binary(phys_bytes)
                types = phys.lists["ParamSet"].objects[1258832850]
                if types.params["use_ragdoll"].v:
                    rg_path = str(
                        phys.lists["ParamSet"]
                        .objects["Ragdoll"]
                        .params["ragdoll_setup_file_path"]
                        .v
                    )
                    if (phys_source / "Ragdoll" / rg_path).exists():
                        files[f"Physics/Ragdoll/{rg_path}"] = (
                            phys_source / "Ragdoll" / rg_path
                        )
                    else:
                        params.warning(
                            f"Havok ragdoll file Physics/Ragdoll/{rg_path} not found "
                            f"for actor {actor_name}. Ignore if intentionally using a file not "
                            "in the actor pack."
                        )
                if types.params["use_support_bone"].v:
                    sb_path = str(
                        phys.lists["ParamSet"]
                        .objects["SupportBone"]
                        .params["support_bone_setup_file_path"]
                        .v
                    )
                    if (phys_source / "SupportBone" / sb_path).exists():
                        files[f"Physics/SupportBone/{sb_path}"] = (
                            phys_source / "SupportBone" / sb_path
                        )
                    else:
                        params.warning(
                            f"Havok support bone file Physics/SupportBone/{sb_path} "
                            f"not found for actor {actor_name}. Ignore if intentionally using "
                            "a file not in the actor pack."
                        )
                if types.params["use_cloth"].v:
                    cloth_path = str(
                        phys.lists["ParamSet"]
                        .lists["Cloth"]
                        .objects["ClothHeader"]
                        .params["cloth_setup_file_path"]
                        .v
                    )
                    if (phys_source / "Cloth" / cloth_path).exists():
                        files[f"Physics/Cloth/{cloth_path}"] = (
                            phys_source / "Cloth" / cloth_path
                        )
                    else:
                        params.warning(
                            f"Havok cloth file Physics/Cloth/{cloth_path} not found "
                            f"for actor {actor_name}. Ignore if intentionally using a file not "
                            "in the actor pack."
                        )
                if types.params["use_rigid_body_set_num"].v > 0:
                    for _, rigid in (
                        phys.lists["ParamSet"].lists["RigidBodySet"].lists.items()
                    ):
                        try:
                            rigid_path = str(
                                rigid.objects[4288596824].params["setup_file_path"].v
                            )
                            if (phys_source / "RigidBody" / rigid_path).exists():
                                files[f"Physics/RigidBody/{rigid_path}"] = (
                                    phys_source / "RigidBody" / rigid_path
                                )
                            else:
                                params.warning(
                                    "Havok rigid body file "
                                    f"Physics/RigidBody/{rigid_path} not found for actor "
                                    f"{actor_name}. Ignore if intentionally using a file not in"
                                    " the actor pack."
                                )
                        except KeyError:
                            continue
        for name, path in files.items():
            data = path.read_bytes()
            pack.files[name] = data
            canon = name.replace(".s", ".")
            if params.table.is_file_modded(canon, memoryview(data), True):
                if not modified:
                    modified = True
                rvs.update({canon: _get_rstb_val(canon, data, params.guess, params.be)})
    except FileNotFoundError as e:
        params.warning(
            f'Failed to build actor "{actor_name}". Could not find linked file "'
            f'{Path(e.filename).relative_to(params.out)}".'
        )
        return {}
    _, sb = pack.write()
    dest: Path
    if actor_name in TITLE_ACTORS | params.titles:
        dest = (
            params.out
            / params.content
            / "Pack"
            / "TitleBG.pack"
            / "Actor"
            / "Pack"
            / f"{actor_name}.sbactorpack"
        )
    else:
        dest = (
            params.out / params.content / "Actor" / "Pack" / f"{actor_name}.sbactorpack"
        )
    if not dest.parent.exists():
        dest.parent.mkdir(parents=True, exist_ok=True)
    dest.write_bytes(compress(sb))
    if modified:
        rvs.update(
            {
                f"Actor/Pack/{actor_name}.bactorpack": _get_rstb_val(
                    f"{actor_name}.sbactorpack", sb, params.guess, params.be
                )
            }
        )
    return rvs


def _build_sarc(d: Path, params: BuildParams):
    rvs = {}
    for f in {
        f for f in (params.mod / d.relative_to(params.out)).rglob("**/*") if f.is_file()
    }:
        canon = get_canon_name(f.relative_to(params.mod), True)
        if params.table.is_file_modded(canon, f.read_bytes()):
            modified = True
            break
    else:
        modified = False
    try:
        s = oead.SarcWriter(
            endian=oead.Endianness.Big if params.be else oead.Endianness.Little
        )
        lead = ""
        if (d / ".align").exists():
            alignment = int((d / ".align").read_text())
            s.set_mode(oead.SarcWriter.Mode.Legacy)
            s.set_min_alignment(alignment)
            (d / ".align").unlink()
        if (d / ".slash").exists():
            lead = "/"
            (d / ".slash").unlink()

        f: Path
        for f in {f for f in d.rglob("**/*") if f.is_file()}:
            path = f.relative_to(d).as_posix()
            canon = path.replace(".s", ".")
            data = f.read_bytes()
            if (
                f.suffix not in SARC_EXTS | AAMP_EXTS | BYML_EXTS | RSTB_EXCLUDE_EXTS
                and d.suffix not in {".sarc", ".ssarc"}
            ) and params.table.is_file_modded(canon, data, flag_new=True):
                rvs.update(
                    {
                        canon: _get_rstb_val(
                            path, data, params.guess, params.be
                        )
                    }
                )
            s.files[lead + path] = data
            f.unlink()

        shutil.rmtree(d)
        _, sb = s.write()
        if modified and _should_rstb(d):
            rvs.update(
                {
                    get_canon_name(d.relative_to(params.out)): _get_rstb_val(
                        d.name, sb, params.guess, params.be
                    )
                }
            )
        d.write_bytes(
            sb
            if not (d.suffix.startswith(".s") and d.suffix != ".sarc")
            else compress(sb)
        )
    except Exception as err:  # pylint: disable=broad-except
        params.warning(f"Failed to build {d.relative_to(params.out).as_posix()}. {err}")
        return {}
    else:
        if params.verbose:
            print(f"Built {d.relative_to(params.out).as_posix()}")
        return rvs


def _build_actorinfo(params: BuildParams):
    actors = []
    for actor_file in (params.mod / params.content / "Actor" / "ActorInfo").glob(
        "*.info.yml"
    ):
        actors.append(oead.byml.from_text(actor_file.read_text(encoding="utf-8")))
    hashes = oead.byml.Array(
        [
            oead.S32(crc) if crc < 2147483648 else oead.U32(crc)
            for crc in sorted({crc32(a["name"].encode("utf8")) for a in actors})
        ]
    )
    actors.sort(key=lambda actor: crc32(actor["name"].encode("utf8")))
    actor_info = oead.byml.Hash({"Actors": oead.byml.Array(actors), "Hashes": hashes})
    info_path = params.out / params.content / "Actor" / "ActorInfo.product.sbyml"
    info_path.parent.mkdir(exist_ok=True, parents=True)
    info_path.write_bytes(
        compress(oead.byml.to_binary(actor_info, big_endian=params.be))
    )


def build_mod(args):
    content = "content" if args.be else "01007EF00011E000/romfs"
    aoc = "aoc" if args.be else "01007EF00011F001/romfs"
    mod = Path(args.directory)
    if not ((mod / content).exists() or (mod / aoc).exists()):
        print(
            "The specified directory does not appear to have a valid folder structure."
        )
        print("Run `hyrule_builder build --help` for more information.")
        sys.exit(2)
    out = mod.with_name(f"{mod.name}_build") if not args.output else Path(args.output)
    if out.exists():
        print("Removing old build...")
        shutil.rmtree(out)

    params = BuildParams(
        mod=mod,
        out=out,
        be=args.be,
        guess=not args.no_guess,
        verbose=args.verbose,
        content=content,
        aoc=aoc,
        titles=set(args.title_actors.split(",")),
        table=StockHashTable(args.be),
        warn=not args.no_warn,
        strict=args.hard_warn,
    )

    print("Scanning source files...")
    files = {
        f
        for f in mod.rglob("**/*")
        if f.is_file()
        and "ActorInfo" not in f.parts
        and not str(f.relative_to(mod)).startswith(".")
    }
    other_files = {f for f in files if f.suffix not in {".yml", ".msyt"}}
    yml_files = {f for f in files if f.suffix == ".yml"}
    f: Path
    rvs = {}
    if not args.single:
        p = Pool()

    print("Copying miscellaneous files...")
    if args.single or len(other_files) < 2:
        for f in other_files:
            rvs.update(_copy_file(f, params))
    else:
        results = p.map(partial(_copy_file, params=params), other_files)
        for r in results:
            rvs.update(r)

    if (mod / content).exists():
        print("Building actor info...")
        if (mod / content / "Actor" / "ActorInfo").is_dir():
            _build_actorinfo(params)

        msg_dirs = {
            d
            for d in mod.glob(f"{content}/Pack/Bootup_*.pack")
            if d.is_dir() and not d.name == "Bootup_Graphics.pack"
        }
        if msg_dirs:
            print("Building MSBT files...")
        for d in msg_dirs:
            msg_dir = next(d.glob("Message/*"))
            new_dir = out / msg_dir.relative_to(mod).with_suffix(".ssarc.ssarc")
            pymsyt.create(msg_dir, new_dir)

    print("Building AAMP and BYML files...")
    if args.single or len(yml_files) < 2:
        for f in yml_files:
            rvs.update(_build_yml(f, params))
    else:
        try:
            results = p.map(partial(_build_yml, params=params), yml_files)
        except RuntimeError as err:
            print(err)
            sys.exit(1)
        for r in results:
            rvs.update(r)

    main_aoc = Path(aoc) / ("0010" if params.be else "")
    if (params.out / main_aoc / "Map" / "MainField").exists():
        (params.out / main_aoc / "Pack").mkdir(parents=True, exist_ok=True)
        (params.out / main_aoc / "Pack" / "AocMainField.pack").write_bytes(b"")

    actors = {f for f in (out / content / "Actor" / "ActorLink").glob("*.bxml")}
    if actors:
        (out / content / "Actor" / "Pack").mkdir(parents=True, exist_ok=True)
        print("Building actor packs...")
        if args.single or len(actors) < 2:
            for a in actors:
                rvs.update(_build_actor(a, params))
        else:
            try:
                results = p.map(partial(_build_actor, params=params), actors)
            except RuntimeError as err:
                print(err)
                sys.exit(1)
            for r in results:
                rvs.update(r)
    for d in (out / content / "Physics").glob("*"):
        if d.stem not in ["StaticCompound", "TeraMeshRigidBody"]:
            shutil.rmtree(d)
    {  # pylint: disable=expression-not-assigned
        shutil.rmtree(d)
        for d in (out / content / "Actor").glob("*")
        if d.is_dir() and d.stem != "Pack"
    }

    print("Building SARC files...")
    dirs = {d for d in out.rglob("**/*") if d.is_dir()}
    sarc_folders = {d for d in dirs if d.suffix in SARC_EXTS and d.suffix != ".pack"}
    pack_folders = {d for d in dirs if d.suffix == ".pack"}
    if args.single or (len(sarc_folders) + len(pack_folders)) < 3:
        for d in sarc_folders:
            rvs.update(_build_sarc(d, params))
        for d in pack_folders:
            rvs.update(_build_sarc(d, params))
    else:
        sarc_func = partial(_build_sarc, params=params)
        results = p.map(sarc_func, sarc_folders)
        for r in results:
            rvs.update(r)
        results = p.map(sarc_func, pack_folders)
        for r in results:
            rvs.update(r)

    if p:
        p.close()
        p.join()

    rp = out / content / "System" / "Resource" / "ResourceSizeTable.product.json"
    if rp.exists() or rvs:
        print("Updating RSTB...")
        table: ResourceSizeTable
        if args.no_rstb:
            if rp.exists():
                table = load_rstb(args.be, file=rp)
        else:
            if rp.exists():
                table = load_rstb(args.be, file=rp)
            else:
                table = load_rstb(args.be)
                rp.parent.mkdir(parents=True, exist_ok=True)
            if rvs and not (len(rvs) == 1 and list(rvs.keys())[0] is None):
                for p, v in rvs.items():
                    if not p:
                        continue
                    msg: str = ""
                    if table.is_in_table(p):
                        if v > table.get_size(p) > 0:
                            table.set_size(p, v)
                            msg = f"Updated {p} to {v}"
                        elif v == 0:
                            table.delete_entry(p)
                            msg = f"Deleted {p}"
                        else:
                            msg = f"Skipped {p}"
                    else:
                        if v > 0 and p not in STOCK_FILES:
                            table.set_size(p, v)
                            msg = f"Added {p}, set to {v}"
                    if args.verbose and msg:
                        print(msg)
        buf = BytesIO()
        table.write(buf, args.be)
        rp.with_suffix(".srsizetable").write_bytes(compress(buf.getvalue()))
        if rp.exists():
            rp.unlink()

    print("Mod built successfully")
