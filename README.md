# Hyrule Builder

A mod building tool for _The Legend of Zelda: Breath of the Wild_.

Hyrule Builder is designed to help BOTW modders more easily manage and edit their project files. It
can "unbuild"/"decompile" game files to a source-like format. All SARCs are extracted, all BYML,
AAMP, or MSYT files are converted to YAML, and actor packs are specially unbuilt using canonical
paths. The whole project can be easily rebuilt into usable mod, with a number of convenience
features to smooth the process.

## Setup

Install Python 3.7+ (**64 bit version**), then run `pip install hyrule_builder`.

## Building and Unbuilding Mods

To start a new Hyrule Builder project, take the mod files you would like to use and make sure they
are placed in a valid folder structure. Hyrule Builder supports both Wii U and Switch mods, but
each has a different format.

Wii U layout:

```none
. (root mod folder)
├── content
│   └── (content folders, e.g. Actor, Pack, etc.)
└── aoc (for DLC files)
    └── (DLC folders, e.g. Map, Pack, etc.)
```

Switch layout (note: title IDs are case sensitive):

```none
. (root mod folder)
├── 01007EF00011E000 (base game files)
│   └── romfs
│       └── (content folders, e.g. Actor, Pack, etc.)
└── 01007EF00011F001 (DLC files)
    └── romfs
        └── (content folders, e.g. Map, Pack, etc.)
```

To convert your existing mod files into a Hyrule Builder project, use the `unbuild` command. Example:

`hyrule_builder unbuild BreathOfTheWild_VeryCleverMod`

### Unbuilding

For details on unbuilding mods, see the help for the `unbuild` command:

```none
usage: hyrule_builder unbuild [-h] [--output OUTPUT] [--single] [--verbose] directory

Unbuilds a mod into a source-like structure for editing

positional arguments:
  directory             The main mod folder. For Wii U, this must contain a `content` folder and/or an `aoc` folder (the latter for DLC files). For Switch, you must use the following layout:
                           .
                           ├─ 01007EF00011E000 (for base game files)
                           │  └─ romfs
                           └─ 01007EF00011F001 (for DLC files)
                              └─ romfs

optional arguments:
  -h, --help            show this help message and exit
  --output OUTPUT, -O OUTPUT
                        Output folder for unbuilt mod
  --single, -S          Run with single thread
  --verbose, -V         Provide more detailed output
```

### Building

For details on building mods, see the help for the `build` command:

```none
usage: hyrule_builder build [-h] [--be] [--no-rstb] [--no-guess] [--title-actors TITLE_ACTORS] [--output OUTPUT] [--single] [--verbose] directory

Builds a mod from a source-like structure into binary game files
Note: Flags can be set using a config.yml file. See readme for details.

positional arguments:
  directory             The main mod folder. For Wii U, this must contain a `content` folder and/or an `aoc` folder (the latter for DLC files). For Switch, you must use the following layout:
                           .
                           ├─ 01007EF00011E000 (for base game files)
                           │  └─ romfs
                           └─ 01007EF00011F001 (for DLC files)
                              └─ romfs

optional arguments:
  -h, --help            show this help message and exit
  --be, -B, -b          Use big endian where applicable
  --no-rstb, -R         Do not auto-update RSTB
  --no-guess, -G        Do not use RSTB estimates
  --no-warn, -W         Ignore warnings, only output success/error
  --hard-warn, -H       Abort on any warning like an error
  --title-actors TITLE_ACTORS, -T TITLE_ACTORS
                        Comma separated list of custom actors to add to TitleBG.pack, e.g.
                        `--title-actors=Weapon_Bow_001,Enemy_Golem_Senior`
  --output OUTPUT, -O OUTPUT
                        Output folder for built mod
  --single, -S          Run with single thread
  --verbose, -V         Provide more detailed output
```

Note that endianness can be inferred on `unbuild`, but using `build` for Wii U/Cemu mods *requires* the `--be` flag.

Unless `--no-rstb` is used, building a mod will automatically generate an updated RSTB file.

As the help says, instead of using command line arguments, you can also configure the build command by providing a `config.yml`
file. It supports up to three sections, each of which is optional. The `Meta` section provides data that will be written
into a `rules.txt` file in the output mod. The `Flags` section turns simple switch flags on by name in a list. The `Options`
section is for flags that store arbitary values, like `title-actors` or `output`. An example config file is included below:

```yaml
Meta: # specify data to go into a rules.txt file here
  name: A Mod
  description: My new mod
Flags: # list the switch flags you want turned on
  - be
  - no-warn
Options: # provide values for customizable flags
  title-actors: Weapon_Bow_001,Enemy_Lizalfos_Senior
  output: test/TestMod_built
```

### Notes on Project Layout

Most of a Hyrule Builder project layout will be familiar to anyone who has worked with BOTW mods, especially graphic packs, before. However, the following aspects are unique to Hyrule Builder:

- `ActorInfo.product.sbyml` becomes the `Actor/ActorInfo` folder with individual YAML files for each actor. The hash list is handled automatically. Simply adding a new YAML file to the folder will add it to the actor list and hash list.
- Most SARC files are unbuilt in place with their original filename, e.g. `Pack/TitleBG.pack` simply becomes a folder with the same name. However, actor packs are unbuilt fully into the `Actor` folder and rebuilt from their actor link files. For example, if you were to make a new copy of `Actor/ActorLink/Enemy_Lizalfos_Senior.bxml.yml` in your project and name it `Enemy_Lizalfos_Geezer.bxml.yml`, this change alone would cause the build process to create a new actor pack at `Actor/Pack/Enemy_Lizalfos_Geezer.sbactorpack`.
- Some SARC files which are parsed by standard Nintendo libraries instead of the BOTW resource system will not be unbuilt for safety reasons.
- The RSTB is unbuilt to a JSON file for easy editing. Format sample:
  ```json
  {
      "hash_map": {
          "Actor/ActorLink/Armor_064_Head.bxml": 2152,
          "Actor/ActorLink/FidObj_SecretBase_TorchStand_A_01.bxml": 2068,
          "165741": 78080,
          "Game/Stats/archive/I-7.00.stats": 392384,
          "Actor/ModelList/FldObj_SecretBaseRockBlock_E_01.bmodellist": 2640,
          "Actor/Physics/FldObj_HyliaStoneRuinGate_A_01.bphysics": 2864,
          "Actor/ModelList/WaterPump.bmodellist": 2576,
          "Actor/Pack/TwnObj_HyruleCastleObject_WeaponStand_B_01.bactorpack": 2560,
          "Actor/ModelList/TwnObj_Village_SheikerWallScroll_A_01.bmodellist": 2640,
          "Actor/Pack/TwnObj_Village_HatenoBanner_A_03.bactorpack": 2432,
          "Actor/ModelList/FldObj_WaterFallLakeFloria_A_S_02_Far.bmodellist": 2640,
          // etc...
      },
      "name_map": {
          "Actor/Physics/SwitchStepL.bphysics": 9652,
          "Actor/Physics/TwnObj_HyruleCastleObject_StoneStatue_B_01.bphysics": 2800,
          "Model/Item_Roast_08.Tex1.bfres": 28672,
          "Model/Item_Roast_08.Tex2.bfres": 28928,
          // and so on...
      }
  }
  ```

## Other Usage

Hyrule Builder also includes a couple additional CLI tools implementing some of its features for separate use. This includes an RSTB<->JSON converter and a complete builder/unbuilder for individual SARC files. Usage details follows:

### RSTB to JSON

```none
usage: rstb_to_json [-h] [-b] [-o [OUTPUT]] [rstb]

Converts a binary RSTB file to JSON.

positional arguments:
  rstb                  Path to a binary RSTB file

optional arguments:
  -h, --help            show this help message and exit
  -b, --be              Read the RSTB as big endian for Wii U, otherwise little endian for Switch
  -o [OUTPUT], --output [OUTPUT]
                        Path to output JSON file
```

### JSON to RSTB

```none
usage: json_to_rstb [-h] [-b] [-o [OUTPUT]] [json]

Converts a JSON RSTB file to binary.

positional arguments:
  json                  Path to a JSON RSTB file

optional arguments:
  -h, --help            show this help message and exit
  -b, --be              Write the RSTB as big endian for Wii U, otherwise little endian for Switch
  -o [OUTPUT], --output [OUTPUT]
                        Path to output RSTB binary
```

### Unbuild SARC

```none
usage: unbuild_sarc [-h] [--output OUTPUT] sarc

Unbuild a single SARC file completely

positional arguments:
  sarc                  SARC archive to unbuild

optional arguments:
  -h, --help            show this help message and exit
  --output OUTPUT, -O OUTPUT
                        Output folder for unbuilt SARC, defaults to file name w/o extension
```

### Build SARC

```none
usage: build_sarc [-h] [--be] [--verbose] source output

Build a SARC file from a single source folder

positional arguments:
  source         Source folder for SARC
  output         Path to output SARC file, will auto compress if extension starts with ".s"

optional arguments:
  -h, --help     show this help message and exit
  --be, -B       Use big endian where applicable
  --verbose, -V  Provide more detailed output
```

## License

This software is licensed under the terms of the GNU General Public License, version 3 or later.
