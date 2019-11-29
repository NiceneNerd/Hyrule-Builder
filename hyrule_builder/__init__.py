# pylint: skip-file
import os
from pathlib import Path

SARC_EXTS = {'.sarc', '.pack', '.bactorpack', '.bmodelsh', '.beventpack', '.stera', '.stats',
             '.ssarc', '.spack', '.sbactorpack', '.sbmodelsh', '.sbeventpack', '.sstera', '.sstats'}
AAMP_EXTS = {'.bxml', '.sbxml', '.bas', '.sbas', '.baglblm', '.sbaglblm', '.baglccr', '.sbaglccr',
             '.baglclwd', '.sbaglclwd', '.baglcube', '.sbaglcube', '.bagldof', '.sbagldof',
             '.baglenv', '.sbaglenv', '.baglenvset', '.sbaglenvset', '.baglfila', '.sbaglfila',
             '.bagllmap', '.sbagllmap', '.bagllref', '.sbagllref', '.baglmf', '.sbaglmf',
             '.baglshpp', '.sbaglshpp', '.baiprog', '.sbaiprog', '.baslist', '.sbaslist',
             '.bassetting', '.sbassetting', '.batcl', '.sbatcl', '.batcllist', '.sbatcllist',
             '.bawareness', '.sbawareness', '.bawntable', '.sbawntable', '.bbonectrl',
             '.sbbonectrl', '.bchemical', '.sbchemical', '.bchmres', '.sbchmres', '.bdemo',
             '.sbdemo', '.bdgnenv', '.sbdgnenv', '.bdmgparam', '.sbdmgparam', '.bdrop', '.sbdrop',
             '.bgapkginfo', '.sbgapkginfo', '.bgapkglist', '.sbgapkglist', '.bgenv', '.sbgenv',
             '.bglght', '.sbglght', '.bgmsconf', '.sbgmsconf', '.bgparamlist', '.sbgparamlist',
             '.bgsdw', '.sbgsdw', '.bksky', '.sbksky', '.blifecondition', '.sblifecondition',
             '.blod', '.sblod', '.bmodellist', '.sbmodellist', '.bmscdef', '.sbmscdef', '.bmscinfo',
             '.sbmscinfo', '.bnetfp', '.sbnetfp', '.bphyscharcon', '.sbphyscharcon',
             '.bphyscontact', '.sbphyscontact', '.bphysics', '.sbphysics', '.bphyslayer',
             '.sbphyslayer', '.bphysmaterial', '.sbphysmaterial', '.bphyssb', '.sbphyssb',
             '.bphyssubmat', '.sbphyssubmat', '.bptclconf', '.sbptclconf', '.brecipe', '.sbrecipe',
             '.brgbw', '.sbrgbw', '.brgcon', '.sbrgcon', '.brgconfig', '.sbrgconfig',
             '.brgconfiglist', '.sbrgconfiglist', '.bsfbt', '.sbsfbt', '.bsft', '.sbsft', '.bshop',
             '.sbshop', '.bumii', '.sbumii', '.bvege', '.sbvege', '.bactcapt', '.sbactcapt'}
BYML_EXTS = {'.bgdata', '.sbgdata', '.bquestpack', '.sbquestpack', '.byml', '.sbyml', '.mubin',
             '.smubin', '.baischedule', '.sbaischedule', '.baniminfo', '.sbaniminfo', '.bgsvdata',
             '.sbgsvdata'}
EXEC_DIR = Path(os.path.dirname(os.path.realpath(__file__)))

def get_canon_name(file: str, allow_no_source: bool = False) -> str:
    name = str(file)\
        .replace("\\", "/")\
        .replace('atmosphere/titles/01007EF00011E000/romfs', 'content')\
        .replace('atmosphere/titles/01007EF00011E001/romfs', 'aoc/0010')\
        .replace('atmosphere/titles/01007EF00011E002/romfs', 'aoc/0010')\
        .replace('atmosphere/titles/01007EF00011F001/romfs', 'aoc/0010')\
        .replace('atmosphere/titles/01007EF00011F002/romfs', 'aoc/0010')\
        .replace('.s', '.')\
        .replace('Content', 'content')\
        .replace('Aoc', 'aoc')
    if 'aoc/' in name:
        return name.replace('aoc/content', 'aoc').replace('aoc', 'Aoc')
    elif 'content/' in name and '/aoc' not in name:
        return name.replace('content/', '')
    elif allow_no_source:
        return name

try:
    import libyaz0.yaz0_cy
    decompress = libyaz0.yaz0_cy.DecompressYaz
    def compress(data: bytes) -> bytes:
        compressed_data = libyaz0.yaz0_cy.CompressYaz(bytes(data), 10)
        result = bytearray(b'Yaz0')
        result += len(data).to_bytes(4, "big")
        result += (0).to_bytes(4, "big")
        result += b'\0\0\0\0'
        result += compressed_data
        return result
except (ImportError, NameError):
    from wszst_yaz0 import decompress, compress
