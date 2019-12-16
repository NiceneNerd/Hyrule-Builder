# pylint: disable=invalid-name
""" Functions for unbuilding BOTW mods """
from multiprocessing import Pool, cpu_count
from pathlib import Path
from shutil import rmtree
from typing import Union
from zlib import crc32

import aamp.converters as ac
import byml
from byml import yaml_util as by
import pymsyt
from rstb.util import read_rstb
import sarc
import yaml
from . import AAMP_EXTS, BYML_EXTS, SARC_EXTS, EXEC_DIR, decompress, get_canon_name

def _if_unyaz(data: bytes) -> bytes:
    return data if data[0:4] != b'Yaz0' else decompress(data)

def _unbuild_file(f: Path, out: Path, mod: Path, be: bool, verbose: bool) -> set:
    of = out / f.relative_to(mod)
    if not of.parent.exists():
        of.parent.mkdir(parents=True, exist_ok=True)
    names = set()
    canon = get_canon_name(f.relative_to(mod).as_posix())
    if canon:
        names.add(canon)
    if f.name == 'ResourceSizeTable.product.srsizetable':
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

def _unbuild_rstb(f: Path, be: bool, out: Path, mod: Path, names: set):
    import json
    ver = 'wiiu' if be else 'switch'
    hash_map = {
        crc32(h.encode('utf8')): h for h in json.loads(
            (EXEC_DIR / 'data' / ver / 'hashes.json').read_text(encoding='utf-8')
        )
    }
    hash_map.update({
        crc32(name.encode('utf8')): name for name in names
    })
    table = read_rstb(str(f), be)

    def hash_to_name(crc: int) -> Union[str, int]:
        return hash_map[crc] if crc in hash_map else crc

    (out / f.relative_to(mod).with_suffix('.json')).write_text(
        json.dumps({
            'hash_map': {hash_to_name(k): v for k, v in table.crc32_map.items()},
            'name_map': table.name_map
        }, ensure_ascii=False, indent=2)
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
        if ext not in {*SARC_EXTS, *AAMP_EXTS, *BYML_EXTS}:
            osf.write_bytes(s.get_file_data(sf).tobytes())
        elif ext in AAMP_EXTS:
            osf.with_suffix(f'{osf.suffix}.yml').write_bytes(
                ac.aamp_to_yml(s.get_file_data(sf).tobytes())
            )
        elif ext in BYML_EXTS:
            osf.with_suffix(f'{osf.suffix}.yml').write_bytes(
                _byml_to_yml(s.get_file_data(sf).tobytes())
            )
        else:
            if osf.name in SKIP_SARCS:
                osf.write_bytes(s.get_file_data(sf).tobytes())
                continue
            try:
                ss = sarc.SARC(_if_unyaz(s.get_file_data(sf).tobytes()))
                names.update(_unbuild_sarc(ss, osf))
                del ss
            except ValueError:
                osf.write_bytes(b'')

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
    ).encode('utf8')

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
                _unbuild_file(f, out, mod, be, args.verbose)
            )
    else:
        from functools import partial
        p = Pool(processes=t)
        result = p.map(partial(_unbuild_file, mod=mod, be=be, out=out, verbose=args.verbose), files)
        p.close()
        p.join()
        for r in result:
            if r:
                names.update(r)
    
    content = 'content' if be else 'atmosphere/titles/01007EF00011E000/romfs'
    if (mod / content / 'System' / 'Resource' / 'ResourceSizeTable.product.srsizetable').exists():
        _unbuild_rstb(
            mod / content / 'System' / 'Resource' / 'ResourceSizeTable.product.srsizetable',
            be,
            out,
            mod,
            names
        )

    print('Unbuilding complete')
