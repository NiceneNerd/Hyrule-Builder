# pylint: disable=missing-docstring
from pathlib import Path
from typing import Union
import wszst_yaz0

def guess_bfres_size(file: Union[Path, bytes], name: str = '') -> int:
    real_bytes = file if isinstance(file, bytes) else file.read_bytes()
    if real_bytes[0:4] == b'Yaz0':
        real_bytes = wszst_yaz0.decompress(real_bytes)
    real_size = int(len(real_bytes) * 1.1)
    del real_bytes
    if name == '':
        if isinstance(file, Path):
            name = file.name
        else:
            raise ValueError('BFRES name must not be blank if passing file as bytes.')
    if '.Tex' in name:
        if real_size < 100:
            return real_size * 9
        if 100 < real_size <= 2000:
            return real_size * 7
        if 2000 < real_size <= 3000:
            return real_size * 5
        if 3000 < real_size <= 4000:
            return real_size * 4
        if 4000 < real_size <= 8500:
            return real_size * 3
        if 8500 < real_size <= 12000:
            return real_size * 2
        if 12000 < real_size <= 17000:
            return int(real_size * 1.75)
        if 17000 < real_size <= 30000:
            return int(real_size * 1.5)
        if 30000 < real_size <= 45000:
            return int(real_size * 1.3)
        if 45000 < real_size <= 100000:
            return int(real_size * 1.2)
        if 100000 < real_size <= 150000:
            return int(real_size * 1.1)
        if 150000 < real_size <= 200000:
            return int(real_size * 1.07)
        if 200000 < real_size <= 250000:
            return int(real_size * 1.045)
        if 250000 < real_size <= 300000:
            return int(real_size * 1.035)
        if 300000 < real_size <= 600000:
            return int(real_size * 1.03)
        if 600000 < real_size <= 1000000:
            return int(real_size * 1.015)
        if 1000000 < real_size <= 1800000:
            return int(real_size * 1.009)
        if 1800000 < real_size <= 4500000:
            return int(real_size * 1.005)
        if 4500000 < real_size <= 6000000:
            return int(real_size * 1.002)
        return int(real_size * 1.0015)
    if real_size < 500:
        return real_size * 7
    if 500 < real_size <= 750:
        return real_size * 4
    if 750 < real_size <= 2000:
        return real_size * 3
    if 2000 < real_size <= 400000:
        return int(real_size * 1.75)
    if 400000 < real_size <= 600000:
        return int(real_size * 1.7)
    if 600000 < real_size <= 1500000:
        return int(real_size * 1.6)
    if 1500000 < real_size <= 3000000:
        return int(real_size * 1.5)
    return int(real_size * 1.25)


def guess_aamp_size(file: Union[Path, bytes], ext: str = '') -> int:
    real_bytes = file if isinstance(file, bytes) else file.read_bytes()
    if real_bytes[0:4] == b'Yaz0':
        real_bytes = wszst_yaz0.decompress(real_bytes)
    real_size = int(len(real_bytes) * 1.1)
    del real_bytes
    if ext == '':
        if isinstance(file, Path):
            ext = file.suffix
        else:
            raise ValueError('AAMP extension must not be blank if passing file as bytes.')
    ext = ext.replace('.s', '.')
    if ext == '.baiprog':
        if real_size <= 380:
            return real_size * 7
        if 380 < real_size <= 400:
            return real_size * 6
        if 400 < real_size <= 450:
            return int(real_size * 5.5)
        if 450 < real_size <= 600:
            return real_size * 5
        if 600 < real_size <= 1000:
            return real_size * 4
        if 1000 < real_size <= 1750:
            return int(real_size * 3.5)
        return real_size * 3
    if ext == '.bgparamlist':
        if real_size <= 100:
            return real_size * 20
        if 100 < real_size <= 150:
            return real_size * 12
        if 150 < real_size <= 250:
            return real_size * 10
        if 250 < real_size <= 350:
            return real_size * 8
        if 350 < real_size <= 450:
            return real_size * 7
        return real_size * 6
    if ext == '.bdrop':
        if real_size < 200:
            return int(real_size * 8.5)
        if 200 < real_size <= 250:
            return real_size * 7
        if 250 < real_size <= 350:
            return real_size * 6
        if 350 < real_size <= 450:
            return int(real_size * 5.25)
        if 450 < real_size <= 850:
            return int(real_size * 4.5)
        return real_size * 4
    if ext == '.bxml':
        if real_size < 350:
            return real_size * 6
        if 350 < real_size <= 450:
            return real_size * 5
        if 450 < real_size <= 550:
            return int(real_size * 4.5)
        if 550 < real_size <= 650:
            return real_size * 4
        if 650 < real_size <= 800:
            return int(real_size * 3.5)
        return real_size * 3
    if ext == '.brecipe':
        if real_size < 100:
            return int(real_size * 12.5)
        if 100 < real_size <= 160:
            return int(real_size * 8.5)
        if 160 < real_size <= 200:
            return int(real_size * 7.5)
        if 200 < real_size <= 215:
            return real_size * 7
        return int(real_size * 6.5)
    if ext == '.bshop':
        if real_size < 200:
            return int(real_size * 7.25)
        if 200 < real_size <= 400:
            return real_size * 6
        if 400 < real_size <= 500:
            return real_size * 5
        return int(real_size * 4.05)
    if ext == '.bas':
        if real_size < 100:
            return real_size * 20
        if 100 < real_size <= 200:
            return int(real_size * 12.5)
        if 200 < real_size <= 300:
            return real_size * 10
        if 300 < real_size <= 600:
            return real_size * 8
        if 600 < real_size <= 1500:
            return real_size * 6
        if 1500 < real_size <= 2000:
            return int(real_size * 5.5)
        if 2000 < real_size <= 15000:
            return real_size * 5
        return int(real_size * 4.5)
    if ext == '.baslist':
        if real_size < 100:
            return real_size * 15
        if 100 < real_size <= 200:
            return real_size * 10
        if 200 < real_size <= 300:
            return real_size * 8
        if 300 < real_size <= 500:
            return real_size * 6
        if 500 < real_size <= 800:
            return real_size * 5
        if 800 < real_size <= 4000:
            return real_size * 4
        return int(real_size * 3.5)
    if ext == '.bdmgparam':
        return int(((-0.0018 * real_size) + 6.6273) * real_size) + 500
    return 0
