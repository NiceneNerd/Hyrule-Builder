# pylint: disable=invalid-name,bare-except,missing-docstring,no-name-in-module,bad-continuation
import json
from datetime import datetime
from functools import partial
from multiprocessing import Pool, cpu_count
from os import system
from pathlib import Path
from shutil import rmtree
from typing import Union
from zlib import crc32

import oead
from oead import aamp
from oead.yaz0 import decompress  # pylint: disable=import-error
import pymsyt
from rstb import ResourceSizeTable
from rstb.util import read_rstb

from . import (
    AAMP_EXTS,
    BYML_EXTS,
    SARC_EXTS,
    STOCK_FILES,
    get_canon_name,
    NAMES,
    RSTB_EXCLUDE_EXTS,
    RSTB_EXCLUDE_NAMES,
)

HANDLED = {"ResourceSizeTable.product.srsizetable", "ActorInfo.product.sbyml"}


def _if_unyaz(data: bytes) -> bytes:
    return data if data[0:4] != b"Yaz0" else decompress(data)


def _unbuild_file(f: Path, out: Path, content: str, mod: Path, verbose: bool) -> set:
    of = out / f.relative_to(mod)
    if not of.parent.exists():
        of.parent.mkdir(parents=True, exist_ok=True)
    names = set()
    canon = get_canon_name(f.relative_to(mod))
    if canon:
        names.add(canon)
    if f.name in HANDLED:
        pass
    elif f.suffix in AAMP_EXTS:
        of.with_suffix(f"{f.suffix}.yml").write_bytes(_aamp_to_yml(f.read_bytes()))
    elif f.suffix in BYML_EXTS:
        of.with_suffix(f"{f.suffix}.yml").write_bytes(_byml_to_yml(f.read_bytes()))
    elif f.suffix in SARC_EXTS:
        with f.open("rb") as file:
            data = file.read()
            if data[0:4] == b"Yaz0":
                data = decompress(data)
            if data[0:4] != b"SARC":
                return names
            s = oead.Sarc(data)
        if "bactorpack" in f.suffix:
            names.update(_unbuild_actorpack(s, out / content))
        else:
            names.update(_unbuild_sarc(s, of))
        del s
    else:
        of.write_bytes(f.read_bytes())
    if verbose:
        print(f"Unbuilt {f.relative_to(mod).as_posix()}")
    return names


def rstb_to_json(rstb: ResourceSizeTable, output: Path, names: set):
    hash_map = {crc32(h.encode("utf8")): h for h in STOCK_FILES}
    found_names = {
        crc32(name.encode("utf8")): name
        for name in names
        if name[name.rindex(".") :] not in RSTB_EXCLUDE_EXTS
        and name not in RSTB_EXCLUDE_NAMES
    }
    hash_map.update(found_names)
    saved_names = json.load(NAMES.open("r", encoding="utf-8")) if NAMES.exists() else {}
    hash_map.update(saved_names)

    def hash_to_name(crc: int) -> str:
        return hash_map[crc] if crc in hash_map else str(crc)

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(
            {
                "hash_map": {hash_to_name(k): v for k, v in rstb.crc32_map.items()},
                "name_map": rstb.name_map,
            },
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
        )
    )

    NAMES.parent.mkdir(parents=True, exist_ok=True)
    NAMES.write_text(
        json.dumps({**found_names, **saved_names}, ensure_ascii=False), encoding="utf-8"
    )


def _unbuild_rstb(content: Path, be: bool, out: Path, mod: Path, names: set):
    f: Path = content / "System" / "Resource" / "ResourceSizeTable.product.srsizetable"
    table = read_rstb(str(f), be=be)
    rstb_to_json(table, out / f.relative_to(mod).with_suffix(".json"), names)


def _unbuild_actorinfo(mod: Path, content: str, out: Path):
    file = mod / content / "Actor" / "ActorInfo.product.sbyml"
    actor_info = oead.byml.from_binary(decompress(file.read_bytes()))
    actor_info_dir = out / content / "Actor" / "ActorInfo"
    actor_info_dir.mkdir(parents=True, exist_ok=True)
    for actor in actor_info["Actors"]:
        output = actor_info_dir / f"{actor['name']}.info.yml"
        output.write_text(oead.byml.to_text(actor), encoding="utf-8")


def _unbuild_actorpack(s: oead.Sarc, output: Path):
    output.mkdir(parents=True, exist_ok=True)
    for f in {f for f in s.get_files() if "Actor" in f.name}:
        out = (output / f.name).with_suffix(f"{Path(f.name).suffix}.yml")
        out.parent.mkdir(parents=True, exist_ok=True)
        if f.data[0:4] == b"AAMP":
            out.write_bytes(_aamp_to_yml(f.data))
        elif f.data[0:2] in [b"BY", b"YB"]:
            out.write_bytes(_byml_to_yml(f.data))
    for f in {
        f for f in s.get_files() if "Physics" in f.name and "Actor" not in f.name
    }:
        out = output / f.name
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_bytes(f.data)
    return {f.name for f in s.get_files()}


def _unbuild_sarc(s: oead.Sarc, output: Path, skip_actorpack: bool = False):
    SKIP_SARCS = {
        "tera_resource.Cafe_Cafe_GX2.release.ssarc",
        "tera_resource.Nin_NX_NVN.release.ssarc",
    }

    output.mkdir(parents=True, exist_ok=True)
    if any(f.name.startswith("/") for f in s.get_files()):
        (output / ".slash").write_bytes(b"")

    names = set()
    for sarc_file in s.get_files():
        sf = sarc_file.name
        osf = output / sf
        names.add(sf.replace(".s", "."))
        if sf.startswith("/"):
            osf = output / sf[1:]
        osf.parent.mkdir(parents=True, exist_ok=True)
        ext = osf.suffix
        if ext in SARC_EXTS:
            if osf.name in SKIP_SARCS:
                osf.write_bytes(sarc_file.data)
                continue
            try:
                ss = oead.Sarc(_if_unyaz(sarc_file.data))
                if (
                    "bactorpack" in ext
                    and output.stem == "TitleBG"
                    and not skip_actorpack
                ):
                    names.update(_unbuild_actorpack(ss, output.parent.parent))
                else:
                    names.update(_unbuild_sarc(ss, osf))
                del ss
            except ValueError:
                osf.write_bytes(b"")
        elif ext in AAMP_EXTS:
            if osf.with_suffix(f"{osf.suffix}.yml").exists():
                continue
            osf.with_suffix(f"{osf.suffix}.yml").write_bytes(
                _aamp_to_yml(sarc_file.data)
            )
        elif ext in BYML_EXTS:
            osf.with_suffix(f"{osf.suffix}.yml").write_bytes(
                _byml_to_yml(sarc_file.data)
            )
        else:
            osf.write_bytes(sarc_file.data)

    if "Msg_" in output.name:
        pymsyt.export(output, output)
        rmtree(output)
        output.with_suffix("").rename(output)
    if output.suffix in {".ssarc", ".sarc"}:
        (output / ".align").write_text(str(s.guess_min_alignment()))
    return names


def _byml_to_yml(file_bytes: bytes) -> bytes:
    if file_bytes[0:4] == b"Yaz0":
        file_bytes = decompress(file_bytes)
    return oead.byml.to_text(oead.byml.from_binary(file_bytes)).encode("utf8")


def _aamp_to_yml(file_bytes: bytes) -> bytes:
    if file_bytes[0:4] == b"Yaz0":
        file_bytes = decompress(file_bytes)
    return aamp.ParameterIO.from_binary(file_bytes).to_text().encode("utf8")


def unbuild_mod(args) -> None:
    mod = Path(args.directory)
    if not any(
        d.exists()
        for d in {
            mod / "content",
            mod / "aoc",
            mod / "01007EF00011E000/romfs",
            mod / "01007EF00011F001/romfs",
        }
    ):
        print("The specified directory is not valid: no base game or DLC folder found")
        exit(1)
    out = mod.with_name(f"{mod.name}_unbuilt") if not args.output else Path(args.output)
    if out.exists():
        rmtree(out, True)
    be = (mod / "content").exists() or (mod / "aoc").exists()
    content = "content" if be else "01007EF00011E000/romfs"

    print("Analying files...")
    files = {f for f in mod.rglob("**/*") if f.is_file()}
    t = min(len(files), cpu_count())
    names = set()

    print("Unbuilding...")
    if args.single or t < 2:
        for f in files:
            names.update(_unbuild_file(f, out, content, mod, args.verbose))
    else:
        with Pool(processes=t) as p:
            result = p.map(
                partial(
                    _unbuild_file,
                    mod=mod,
                    content=content,
                    out=out,
                    verbose=args.verbose,
                ),
                files,
            )
            for r in result:
                if r:
                    names.update(r)

    try:
        (out / content / "Actor" / "Pack").rmdir()
    except PermissionError:
        pass

    print("Unpacking actor info...")
    if (mod / content / "Actor" / "ActorInfo.product.sbyml").exists():
        _unbuild_actorinfo(mod, content, out)

    print("Dumping RSTB...")
    if (
        mod / content / "System" / "Resource" / "ResourceSizeTable.product.srsizetable"
    ).exists():
        _unbuild_rstb(mod / content, be, out, mod, names)

    (out / ".done").write_text(str(datetime.now().timestamp()))
    try:
        system(f'attrib +h "{str(out / ".done")}"')
    except:
        pass

    print("Unbuilding complete")
