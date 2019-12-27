# pylint: disable=invalid-name,bare-except,missing-docstring
from datetime import datetime
from multiprocessing import Pool, cpu_count
from pathlib import Path
from shutil import rmtree
from typing import Union
from zlib import crc32

import aamp.converters as ac
import byml
from byml import yaml_util as by
import pymsyt
import sarc
from syaz0 import decompress
import yaml
from rstb.util import read_rstb

from . import AAMP_EXTS, BYML_EXTS, SARC_EXTS, get_canon_name
from .files import STOCK_FILES

HANDLED = {'ResourceSizeTable.product.srsizetable', 'ActorInfo.product.sbyml'}

def _if_unyaz(data: bytes) -> bytes:
    return data if data[0:4] != b'Yaz0' else decompress(data)

def _unbuild_file(f: Path, out: Path, mod: Path, verbose: bool) -> set:
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
        of.with_suffix(f'{f.suffix}.yml').write_bytes(ac.aamp_to_yml(f.read_bytes()))
    elif f.suffix in BYML_EXTS:
        of.with_suffix(f'{f.suffix}.yml').write_bytes(_byml_to_yml(f.read_bytes()))
    elif f.suffix in SARC_EXTS:
        with f.open('rb') as file:
            s = sarc.read_file_and_make_sarc(file)
        if not s:
            return
        names.update(_unbuild_sarc(s, of))
        del s
    else:
        of.write_bytes(f.read_bytes())
    if verbose:
        print(f'Unbuilt {f.relative_to(mod).as_posix()}')
    return names

def _unbuild_rstb(content: Path, be: bool, out: Path, mod: Path, names: set):
    f = content / 'System' / 'Resource' / 'ResourceSizeTable.product.srsizetable'
    import json
    hash_map = {
        crc32(h.encode('utf8')): h for h in STOCK_FILES
    }
    hash_map.update({
        crc32(name.encode('utf8')): name for name in names
    })
    table = read_rstb(str(f), be)

    def hash_to_name(crc: int) -> Union[str, int]:
        return hash_map[crc] if crc in hash_map else crc

    (out / f.relative_to(mod)).parent.mkdir(exist_ok=True, parents=True)
    (out / f.relative_to(mod).with_suffix('.json')).write_text(
        json.dumps({
            'hash_map': {hash_to_name(k): v for k, v in table.crc32_map.items()},
            'name_map': table.name_map
        }, ensure_ascii=False, indent=2)
    )


def _unbuild_actorinfo(mod: Path, content: str, out: Path):
    if not hasattr(_byml_to_yml, 'dumper'):
        _byml_to_yml.dumper = yaml.CDumper
        by.add_representers(_byml_to_yml.dumper)
    file = mod / content / 'Actor' / 'ActorInfo.product.sbyml'
    actor_info = byml.Byml(decompress(file.read_bytes())).parse()
    for actor in actor_info['Actors']:
        output = out / content / 'Actor' / 'ActorInfo' / f"{actor['name']}.info.yml"
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(
            yaml.dump(actor, allow_unicode=True, Dumper=_byml_to_yml.dumper)
        )

def _unbuild_sarc(s: sarc.SARC, output: Path):
    SKIP_SARCS = {
        'tera_resource.Cafe_Cafe_GX2.release.ssarc', 'tera_resource.Nin_NX_NVN.release.ssarc'
    }

    output.mkdir(parents=True, exist_ok=True)
    if any(f.startswith('/') for f in s.list_files()):
        (output / '.slash').write_bytes(b'')

    names = set()
    for sf in s.list_files():
        osf = output / sf
        names.add(sf.replace('.s', '.'))
        if sf.startswith('/'):
            osf = output / sf[1:]
        osf.parent.mkdir(parents=True, exist_ok=True)
        ext = osf.suffix
        if ext in SARC_EXTS:
            if osf.name in SKIP_SARCS:
                osf.write_bytes(s.get_file_data(sf).tobytes())
                continue
            try:
                ss = sarc.SARC(_if_unyaz(s.get_file_data(sf).tobytes()))
                names.update(_unbuild_sarc(ss, osf))
                del ss
            except ValueError:
                osf.write_bytes(b'')
        elif ext in AAMP_EXTS:
            osf.with_suffix(f'{osf.suffix}.yml').write_bytes(
                ac.aamp_to_yml(s.get_file_data(sf).tobytes())
            )
        elif ext in BYML_EXTS:
            osf.with_suffix(f'{osf.suffix}.yml').write_bytes(
                _byml_to_yml(s.get_file_data(sf).tobytes())
            )
        else:
            osf.write_bytes(s.get_file_data(sf).tobytes())

    if 'Msg_' in output.name:
        pymsyt.export(output, output)
        rmtree(output)
        output.with_suffix('').rename(output)
    if output.suffix in {'.ssarc', '.sarc'}:
        (output / '.align').write_text(str(s.guess_default_alignment()))
    return names

def _byml_to_yml(file_bytes: bytes) -> bytes:
    if not hasattr(_byml_to_yml, 'dumper'):
        _byml_to_yml.dumper = yaml.CDumper
        by.add_representers(_byml_to_yml.dumper)
    return yaml.dump(
        byml.Byml(_if_unyaz(file_bytes)).parse(),
        Dumper=_byml_to_yml.dumper,
        allow_unicode=True,
        encoding='utf-8',
        default_flow_style=False
    )

def unbuild_mod(args) -> None:
    mod = Path(args.directory)
    if not any(d.exists() for d in {
            mod / 'content', mod / 'aoc', \
            mod / 'atmosphere/titles/01007EF00011E000/romfs',
            mod / 'atmosphere/titles/01007EF00011F001/romfs'
    }):
        print('The specified directory is not valid: no base or DLC folder found')
        exit(1)
    out = mod.with_name(f'{mod.name}_unbuilt') if not args.output else Path(args.output)
    be = (mod / 'content').exists() or (mod / 'aoc').exists()

    print('Analying files...')
    files = {f for f in mod.rglob('**/*') if f.is_file()}
    t = min(len(files), cpu_count())
    names = set()

    print('Unbuilding...')
    if args.single or t < 2:
        for f in files:
            names.update(
                _unbuild_file(f, out, mod, args.verbose)
            )
    else:
        from functools import partial
        p = Pool(processes=t)
        result = p.map(partial(_unbuild_file, mod=mod, out=out, verbose=args.verbose), files)
        p.close()
        p.join()
        for r in result:
            if r:
                names.update(r)

    content = 'content' if be else 'atmosphere/titles/01007EF00011E000/romfs'

    print('Unpacking actor info...')
    if (mod / content / 'Actor' / 'ActorInfo.product.sbyml').exists():
        _unbuild_actorinfo(mod, content, out)

    print('Dumping RSTB...')
    if (mod / content / 'System' / 'Resource' / 'ResourceSizeTable.product.srsizetable').exists():
        _unbuild_rstb(
            mod / content,
            be,
            out,
            mod,
            names
        )

    (out / '.done').write_text(
        str(datetime.now().timestamp())
    )
    try:
        import os
        os.system(f'attrib +h "{str(out / ".done")}"')
    except:
        pass

    print('Unbuilding complete')
