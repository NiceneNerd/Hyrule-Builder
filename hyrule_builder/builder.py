# pylint: disable=invalid-name,bare-except
""" Functions for building BOTW mods """
from dataclasses import dataclass
from datetime import datetime
from functools import partial
import json
from multiprocessing import Pool, cpu_count
import shutil
from zlib import crc32

import aamp
from aamp import yaml_util as ay
import byml
from byml import yaml_util as by
import pymsyt
from rstb import SizeCalculator, ResourceSizeTable
from rstb.util import write_rstb
import sarc
from xxhash import xxh64_hexdigest
import yaml
from . import (AAMP_EXTS, BYML_EXTS, SARC_EXTS, EXEC_DIR, guess, decompress, compress,
               get_canon_name, Path, is_in_sarc)
from .files import STOCK_FILES

RSTB_EXCLUDE_EXTS = ['.pack', '.bgdata', '.txt', '.bgsvdata', '.yml', '.json', '.ps1', '.bak',
                     '.bat', '.ini', '.png', '.bfstm', '.py', '.sh', '.old', '.stera']
RSTB_EXCLUDE_NAMES = ['ActorInfo.product.byml', '.done']

@dataclass
class BuildParams:
    mod: Path
    out: Path
    content: str
    aoc: str
    be: bool
    guess: bool
    verbose: bool
    ch_date: datetime


def _should_rstb(f: Path) -> bool:
    f = f.with_suffix(f.suffix.replace('.s', '.'))
    return f.suffix not in RSTB_EXCLUDE_EXTS and f.name not in RSTB_EXCLUDE_NAMES


def _load_rstb(be: bool, file: Path = None) -> ResourceSizeTable:
    table = ResourceSizeTable(b'', be=be)
    if not file:
        ver = 'wiiu' if be else 'switch'
        file = EXEC_DIR / 'data' / ver / 'rstb.json'
    ref_contents = json.loads(file.read_text(), encoding='utf-8')

    def parse_hash(file: str) -> int:
        try:
            return int(file)
        except ValueError:
            return crc32(file.encode('utf8'))

    table.crc32_map = {parse_hash(k): v for k, v in ref_contents['hash_map'].items()}
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
    if is_in_sarc(f):
        shutil.copy(f, t)
    else:
        data = f.read_bytes()
        canon = get_canon_name(f.relative_to(params.mod))
        t.write_bytes(data)
    if f.modified_date() > params.ch_date and _should_rstb(f):
        return {
            canon: _get_rstb_val(t.suffix, data, params.guess, params.be)
        }
    return {}

def _build_byml(f: Path, be: bool):
    # pylint: disable=no-member
    if not hasattr(_build_byml, 'loader'):
        setattr(_build_byml, 'loader', yaml.CSafeLoader)
        by.add_constructors(_build_byml.loader)

    with f.open('r', encoding='utf-8') as bf:
        data = yaml.load(bf, Loader=_build_byml.loader)
    file_bytes = byml.Writer(data, be, version=2).get_bytes()
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
        t.write_bytes(data if not t.suffix.startswith('.s') else compress(data))
        if f.modified_date() > params.ch_date and _should_rstb(t):
            canon = get_canon_name(t.relative_to(params.out))
            return {
                canon: _get_rstb_val(
                    t.suffix.replace('.s', ''), data, params.guess, params.be
                )
            }
    except Exception as e:
        print(f'Failed to build {f.relative_to(params.mod).as_posix()}: {e}')
        return {}
    if params.verbose:
        print(f'Built {f.relative_to(params.mod).as_posix()}')
    return rv

def _build_sarc(d: Path, params: BuildParams):
    rvs = {}
    for f in {
        f for f in (params.mod / d.relative_to(params.out)).rglob('**/*') if f.is_file()
    }:
        if f.modified_date() > params.ch_date:
            modified = True
            break
    else:
        modified = False
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
            s.add_file(lead + path, data)
            f.unlink()

        shutil.rmtree(d)
        sb = s.get_bytes()
        if modified and _should_rstb(d):
            rvs.update({
                get_canon_name(d.relative_to(params.out)): _get_rstb_val(
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
    content = 'content' if args.be else 'atmosphere/titles/01007EF00011E000/romfs'
    aoc = 'aoc' if args.be else 'atmosphere/titles/01007EF00011F001/romfs'
    mod = Path(args.directory)
    if not ((mod / content).exists() or (mod / aoc).exists()):
        print('The specified directory is not valid: no content or aoc folder found')
        exit(1)
    out = mod.with_name(f'{mod.name}_build') if not args.output else Path(args.output)
    if out.exists():
        print('Removing old build...')
        shutil.rmtree(out)

    ch_date = datetime.fromtimestamp(float(
        (mod / '.done').read_text()
    ))
    params = BuildParams(mod=mod, out=out, be=args.be, guess=not args.no_guess,
                         verbose=args.verbose, content=content, aoc=aoc,
                         ch_date=ch_date)

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
        msg_dirs = {d for d in mod.glob(f'{content}/Pack/Bootup_*.pack') \
                    if d.is_dir() and not d.name == 'Bootup_Graphics.pack'}
        if msg_dirs:
            print('Building MSBT files...')
        for d in msg_dirs:
            msg_dir = next(d.glob('Message/*'))
            new_dir = out / msg_dir.relative_to(mod).with_suffix('.ssarc.ssarc')
            pymsyt.create(msg_dir, new_dir)

    print('Building AAMP and BYML files...')
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

    if rvs and not (len(rvs) == 1 and list(rvs.keys())[0] is None):
        print('Updating RSTB...')
        rp = out / content / 'System' / 'Resource' / 'ResourceSizeTable.product.json'
        table: ResourceSizeTable
        if args.no_rstb:
            if rp.exists():
                table = _load_rstb(args.be, file=rp)
        else:
            if rp.exists():
                table = _load_rstb(args.be, file=rp)
            else:
                table = _load_rstb(args.be)
                rp.parent.mkdir(parents=True, exist_ok=True)
            for p, v in rvs.items():
                if not p:
                    continue
                msg: str = ''
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
                    if v > 0 and p not in STOCK_FILES:
                        table.set_size(p, v)
                        msg = f'Added {p}, set to {v}'
                if args.verbose and msg:
                    print(msg)
        write_rstb(table, str(rp.with_suffix('.srsizetable')), args.be)
        if rp.exists():
            rp.unlink()

    print('Mod built successfully')
