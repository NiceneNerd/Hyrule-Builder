# pylint: disable=missing-docstring
from setuptools import setup

with open("README.md", "r") as readme:
    LONG = readme.read()

setup(
    name='hyrule_builder',
    version='0.2.8',
    author='NiceneNerd',
    author_email='macadamiadaze@gmail.com',
    description='A mod builder/unbuilder for The Legend of Zelda: Breath of the Wild',
    long_description=LONG,
    long_description_content_type='text/markdown',
    url='https://github.com/NiceneNerd/Hyrule-Builder',
    include_package_data=True,
    packages=['hyrule_builder'],
    entry_points={
        'console_scripts': [
            'hyrule_builder = hyrule_builder.__main__:main'
        ]
    },
    classifiers=[
        'Development Status :: 3 - Alpha',
        'License :: OSI Approved :: GNU General Public License v3 or later (GPLv3+)',
        'Programming Language :: Python :: 3 :: Only'
    ],
    python_requires='>=3.7',
    install_requires=[
        'aamp>=1.3.0.post1',
        'byml>=2.3.0.post1',
        'pymsyt>=0.1.3',
        'pyyaml>=5.1.2',
        'sarc>=2.0.1',
        'rstb>=1.1.2',
        'wszst-yaz0>=1.2.0.post1',
        'xxhash>=1.3.0'
    ]
)
