# pylint: disable=missing-docstring
from setuptools import setup
from hyrule_builder.__version__ import VERSION

with open("README.md", "r") as readme:
    LONG = readme.read()


setup(
    name='hyrule_builder',
    version=VERSION,
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
            'hyrule_builder = hyrule_builder.__main__:main',
            'rstb_to_json = hyrule_builder.rstb_main:rstb_to_json',
            'json_to_rstb = hyrule_builder.rstb_main:json_to_rstb',
            'unbuild_sarc = hyrule_builder.sarc_main:unbuild_sarc',
            'build_sarc = hyrule_builder.sarc_main:build_sarc'
        ]
    },
    classifiers=[
        'Development Status :: 3 - Alpha',
        'License :: OSI Approved :: GNU General Public License v3 or later (GPLv3+)',
        'Programming Language :: Python :: 3 :: Only'
    ],
    python_requires='>=3.7',
    install_requires=[
        'botw-utils>=0.2.2',
        'oead>=0.11.2',
        'pymsyt>=0.1.4',
        'rstb>=1.1.3',
        'xxhash>=1.3.0'
    ]
)
