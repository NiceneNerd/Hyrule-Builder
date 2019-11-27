# pylint: disable=invalid-name,bare-except
""" Functions for building BOTW mods """
from dataclasses import dataclass
from functools import partial
import json
from pathlib import Path
from multiprocessing import Pool, cpu_count
import shutil
import yaml

import aamp
from aamp import yaml_util as ay
import byml
from byml import yaml_util as by
import pymsyt
from rstb import SizeCalculator, ResourceSizeTable
import sarc
from xxhash import xxh32
from . import AAMP_EXTS, BYML_EXTS, SARC_EXTS, EXEC_DIR, guess, decompress, compress

@dataclass
class BuildParams:
    mod: Path
    out: Path
    be: bool
    guess: bool
    verbose: bool
    hashes: dict

def _is_in_sarc(f: Path) -> bool:
    return any(Path(p).suffix in SARC_EXTS for p in f.parts[:-1])

def _get_canon_name(file: str, allow_no_source: bool = False) -> str:
    name = str(file).replace("\\", "/").replace('.s', '.')\
        .replace('Content', 'content').replace('Aoc', 'aoc')
    if 'aoc/' in name:
        return name.replace('aoc/content', 'aoc').replace('aoc', 'Aoc')
    elif 'content/' in name and '/aoc' not in name:
        return name.replace('content/', '')
    elif allow_no_source:
        return name

def _load_rstb(be: bool) -> ResourceSizeTable:
    table = ResourceSizeTable(b'', be=be)
    ver = 'wiiu' if be else 'switch'
    ref_contents = json.loads((EXEC_DIR / 'data' / ver / 'rstb.json').read_text(), encoding='utf-8')
    table.crc32_map = {int(k): v for k, v in ref_contents['hash_map'].items()}
    table.name_map = {k: v for k, v in ref_contents['name_map'].items()}
    return table

def _get_rstb_val(ext: str, data: bytes, should_guess: bool, be: bool) -> int:
    if not hasattr(_get_rstb_val, 'calc'):
        setattr(_get_rstb_val, 'calc', SizeCalculator())
    val = _get_rstb_val.calc.calculate_file_size_with_ext(data, wiiu=be, ext=ext) # pylint: disable=no-member
    if val == 0 and should_guess:
        if ext in AAMP_EXTS:
            val = guess.guess_aamp_size(data, ext)
        elif ext in {'.bfres', '.sbfres'}:
            val = guess.guess_bfres_size(data, ext)
    return val

def _copy_file(f: Path, params: BuildParams):
    t = params.out / f.relative_to(params.mod)
    if not t.parent.exists():
        t.parent.mkdir(parents=True, exist_ok=True)
    if _is_in_sarc(f):
        shutil.copy(f, t)
        return {}
    else:
        data = f.read_bytes()
        canon = _get_canon_name(f.relative_to(params.mod).as_posix())
        xh = xxh32(
            data if not data[0:4] == b'Yaz0' else decompress(data)
        ).hexdigest()
        t.write_bytes(data)
        if (canon in params.hashes and xh not in params.hashes[canon]) \
           or canon not in params.hashes:
            return {
                canon: _get_rstb_val(t.suffix, data, params.guess, params.be)
            }
        else:
            return {}

def _build_byml(f: Path, be: bool):
    if not hasattr(_build_byml, 'loader'):
        _build_byml.loader = yaml.CLoader
        by.add_constructors(_build_byml.loader)

    with f.open('r', encoding='utf-8') as bf:
        data = yaml.load(bf, Loader=_build_byml.loader)
    file_bytes = byml.Writer(data, be).get_bytes()
    return file_bytes

def _build_aamp(f: Path):
    if not hasattr(_build_aamp, 'loader'):
        _build_aamp.loader = yaml.CLoader
        ay.register_constructors(_build_aamp.loader)

    with f.open('r', encoding='utf-8') as af:
        data = yaml.load(af, Loader=_build_aamp.loader)
    file_bytes = aamp.Writer(data).get_bytes()
    return file_bytes

def _build_yml(f: Path, params: BuildParams):
    rv = {}
    try:
        ext = f.with_suffix('').suffix
        t = params.out / f.relative_to(params.mod).with_suffix('')
        if not t.parent.exists():
            t.parent.mkdir(parents=True)
        data: bytes
        if ext in BYML_EXTS:
            data = _build_byml(f, params.be)
        elif ext in AAMP_EXTS:
            data = _build_aamp(f)
        if not _is_in_sarc(f):
            canon = _get_canon_name(t.relative_to(params.out).as_posix())
            xh = xxh32(data).hexdigest()
            if (canon in params.hashes and xh not in params.hashes[canon]) \
               or canon not in params.hashes:
                return {
                    canon: _get_rstb_val(t.suffix.replace('.s', ''), data, params.guess, params.be)
                }
        t.write_bytes(data if not t.suffix.startswith('.s') else compress(data))
    except:
        print(f'Failed to build {f.relative_to(params.mod).as_posix()}')
        return {}
    else:
        if params.verbose:
            print(f'Built {f.relative_to(params.mod).as_posix()}')
        return rv

def _build_sarc(d: Path, params: BuildParams):
    rvs = {}
    try:
        s = sarc.SARCWriter(params.be)
        lead = ''
        if (d / '.align').exists():
            alignment = int((d / '.align').read_text())
            s.set_default_alignment(alignment)
            (d / '.align').unlink()
        if (d / '.slash').exists():
            lead = '/'
            (d / '.slash').unlink()

        f: Path
        for f in {f for f in d.rglob('**/*') if f.is_file()}:
            path = f.relative_to(d).as_posix()
            data = f.read_bytes()
            xhash = xxh32(data if not data[0:4] == b'Yaz0' \
                          else decompress(data)).hexdigest()
            canon = path.replace('.s', '.')
            if ((canon in params.hashes and xhash not in params.hashes[canon]) \
               or canon not in params.hashes) and not d.suffix in {'.ssarc', '.sarc'}:
                rvs.update({
                    _get_canon_name(path, allow_no_source=True): _get_rstb_val(
                        Path(path).suffix, data, params.guess, params.be
                    )
                })
            s.add_file(lead + path, data)
            f.unlink()

        shutil.rmtree(d)
        sb = s.get_bytes()
        canon = _get_canon_name(d.relative_to(params.out).as_posix())
        if (canon in params.hashes and xxh32(sb).hexdigest() not in params.hashes[canon]) \
           or canon not in params.hashes and not _is_in_sarc(d):
            rvs.update({
                _get_canon_name(d.relative_to(params.out).as_posix()): _get_rstb_val(
                    d.suffix, sb, params.guess, params.be
                )
            })
        d.write_bytes(sb if not (d.suffix.startswith('.s') and d.suffix != '.sarc') \
                      else compress(sb))
    except:
        print(f'Failed to build {d.relative_to(params.out).as_posix()}')
        return {}
    else:
        if params.verbose:
            print(f'Built {d.relative_to(params.out).as_posix()}')
        return rvs

def build_mod(args):
    mod = Path(args.directory)
    if not ((mod / 'content').exists() or (mod / 'aoc').exists()):
        print('The specified directory is not valid: no content or aoc folder found')
        exit(1)
    out = mod.with_name(f'{mod.name}_build') if not args.output else Path(args.output)
    if out.exists():
        print('Removing old build...')
        shutil.rmtree(out)

    print('Loading hash table...')
    ver = 'wiiu' if args.be else 'switch'
    hashes = json.loads((EXEC_DIR / 'data' / ver / 'hashes.json').read_text())
    params = BuildParams(mod, out, args.be, not args.no_guess, args.verbose, hashes)

    print('Scanning source files...')
    files = {f for f in mod.rglob('**/*') if f.is_file()}
    other_files = {f for f in files if f.suffix not in {'.yml', '.msyt'}}
    yml_files = {f for f in files if f.suffix == '.yml'}
    f: Path
    rvs = {}

    print('Copying miscellaneous files...')
    if args.single or len(other_files) < 2:
        for f in other_files:
            rvs.update(_copy_file(f, params))
    else:
        p = Pool(processes=min(len(other_files), cpu_count()))
        results = p.map(partial(_copy_file, params=params), other_files)
        p.close()
        p.join()
        for r in results:
            rvs.update(r)

    if (mod / 'content').exists():
        msg_dirs = {d for d in mod.glob('content/Pack/Bootup_*.pack') \
                    if d.is_dir() and not d.name == 'Bootup_Graphics.pack'}
        if msg_dirs:
            print('Building MSBT files...')
        for d in msg_dirs:
            msg_dir = next(d.glob('Message/*'))
            new_dir = out / msg_dir.relative_to(mod).with_suffix('.ssarc.ssarc')
            pymsyt.create(msg_dir, new_dir)

    print('Building BYML files...')
    if args.single or len(yml_files) < 2:
        for f in yml_files:
            rvs.update(_build_yml(f, params))
    else:
        p = Pool(processes=min(len(yml_files), cpu_count()))
        results = p.map(partial(_build_yml, params=params), yml_files)
        p.close()
        p.join()
        for r in results:
            rvs.update(r)

    print('Building SARC files...')
    dirs = {d for d in out.rglob('**/*') if d.is_dir()}
    sarc_folders = {d for d in dirs if d.suffix in SARC_EXTS and d.suffix != '.pack'}
    pack_folders = {d for d in dirs if d.suffix == '.pack'}
    if args.single or (len(sarc_folders) + len(pack_folders)) < 3:
        for d in sarc_folders:
            rvs.update(_build_sarc(d, params))
        for d in pack_folders:
            rvs.update(_build_sarc(d, params))
    else:
        sarc_func = partial(_build_sarc, params=params)
        threads = min(len(sarc_folders), cpu_count())
        p = Pool(processes=threads)
        results = p.map(sarc_func, sarc_folders)
        p.close()
        p.join()
        for r in results:
            rvs.update(r)
        p = Pool(processes=threads)
        results = p.map(sarc_func, pack_folders)
        for r in results:
            rvs.update(r)
        p.close()
        p.join()

    if rvs:
        print('Updating RSTB...')
        rp = out / 'content' / 'System' / 'Resource' / 'ResourceSizeTable.product.srsizetable'
        table: ResourceSizeTable
        if rp.exists():
            table = ResourceSizeTable(decompress(rp.read_bytes()), args.be)
        else:
            table = _load_rstb(args.be)
        for p, v in rvs.items():
            if not p:
                continue
            if table.is_in_table(p):
                if v > table.get_size(p) > 0:
                    table.set_size(p, v)
                    msg = f'Updated {p} to {v}'
                elif v == 0:
                    table.delete_entry(p)
                    msg = f'Deleted {p}'
                else:
                    msg = f'Skipped {p}'
            else:
                if v > 0 and p not in hashes:
                    table.set_size(p, v)
                    msg = f'Added {p}, set to {v}'
            if args.verbose and msg:
                print(msg)

    print('Mod built successfully')
