# Hyrule Builder

A mod building tool for _The Legend of Zelda: Breath of the Wild_.

Hyrule Builder is designed to help BOTW modders more easily manage and edit their project files. It
can "unbuild"/"decompile" game files to a source-like format. All SARCs are extracted, all BYML,
AAMP, or MSYT files are converted to YAML, and actor packs are specially unbuilt using canonical
paths. The whole project can be easily rebuilt into usable mod, with a number of convenience
features to smooth the process.

## Setup

Download the [latest release](https://github.com/NiceneNerd/Hyrule-Builder/releases/latest) and
extract the Hyrule Builder executable.

## Project Management

Each mod you work on with Hyrule Builder is handled as its own project. Projects can be initialized
from an existing mod, which will be unbuilt to the Hyrule Builder project format, or created fresh.
Once a project is setup, it can be edited and built, with support for fast incremental rebuilds.

Hyrule Builder supports both Wii U and Switch mods, but each has a different format.

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

### Create a New Proejct

To create a new, blank mod project, run `hyrule_builder init`, which will prepare a new project in
the current working directory. Pass the `--be` flag if creating a Wii U mod. The basic folder 
structure and an empty database of mod files will be created.

### Initialize from Existing Mod Files

To turn existing mod files into a Hyrule Builder project, first make sure they are in the correct
format as described above. Then run the `init` command with the `-s/--source` flag, e.g.:

`hyrule_builder init -s BreathOfTheWild_VeryCleverMod`

### Further Usage Details

For details on initializing projects, see the help for the `init` command:

```none
hyrule_builder init 0.9.0
Create a new source-like mod project

USAGE:
    hyrule-builder init [FLAGS] [OPTIONS] [directory]

FLAGS:
    -b, --be         Use big endian/Wii U mode
    -c, --config     Create default config.yml
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -s, --source <source>    Source mod folder to unbuild

ARGS:
    <directory>    Target folder to create project in [default: .]
```

### Building

By default, when you build a project, the output files will be stored in the `build` folder of the
project. For full information on building and its options, consult the usage info below:

```none
hyrule_builder build 0.9.0
Build a mod from a source-like structure into binary game files 
Note: Flags can be set using a config.yml file. See readme for details

USAGE:
    hyrule-builder build [FLAGS] [OPTIONS] [--] [source]

FLAGS:
    -b, --be                 Use big endian/Wii U mode
    -h, --hard-warnings      Treat warnings as errors and abort
        --help               Prints help information
    -i, --ignore-warnings    Suppress warnings, show only errors
    -V, --version            Prints version information

OPTIONS:
    -o, --output <output>                   Output folder for built mod
    -t, --title-actors <title-actors>...    Comma separated list of custom actors to add to TitleBG.pack, e.g.
                                            `--title-actors=Weapon_Bow_001,Enemy_Golem_Senior`

ARGS:
    <source>    Source mod folder to build
```

Building a mod will automatically generate an updated RSTB file.

As the help says, instead of using command line arguments, you can also configure the build command
by providing a `config.yml` file. It supports up to three sections, each of which is optional. The
`Meta` section provides data that will be written into a `rules.txt` file in the output mod. The
`Flags` section turns simple switchflags on by name in a list. The `Options` section is for flags
that store arbitary values, like `title-actors` or `output`. An example config file is included
below:

```yaml
Meta: # specify data to go into a rules.txt file here
  name: A Mod
  description: My new mod
Flags: # list the switch flags you want turned on
  - be
  - ignore-warnings
Options: # provide values for customizable flags
  title-actors: Weapon_Bow_001,Enemy_Lizalfos_Senior
  output: test/TestMod_built
```

### Adding Files to Projects

While it is possible to manually create new mod files or copy them in an unbuilt for from your game
dump when adding new content, Hyrule Builder also includes asset management commands to simplify
the process. To use this, first you will need to configure your game dump settings using the
`config` command, e.g. `hyrule_builder config import --from-bcml`. (For more info on the `config`
command, check the usage information with `--help`.)

Once your game dump settings are configured, you can use the `add` command to add content from your
dump. Usage information:

```none
hyrule_builder add 0.9.0
Add new content to the active mod project

USAGE:
    hyrule-builder add [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -p, --project <project>    Project folder to add to [default: .]

SUBCOMMANDS:
    actor        Add an actor to the current project, either modifying a vanilla actor or duplicating it as a new
                 one
    actorinfo    Adds unbuilt actor info to the current project
    event        Add an event to the current project, either modifying a vanilla event or duplicating it as a new
                 one
    help         Prints this message or the help of the given subcommand(s)
    map          Add a map unit to the current project
    pack         Add a root game pack to the mod (e.g. `Bootup.pack`, `AocMainField.pack`, etc.)
```

## Notes on Project Layout

Most of a Hyrule Builder project layout will be familiar to anyone who has worked with BOTW mods,
especially graphic packs, before. However, the following aspects are unique to Hyrule Builder:

- `ActorInfo.product.sbyml` becomes the `Actor/ActorInfo` folder with individual YAML files for
  each actor. The hash list is handled automatically. Simply adding a new YAML file to the folder
  will add it to the actor list and hash list.
- `EventInfo.product.sbyml` works roughly the same way, under `Event/EventInfo`.
- Most SARC files are unbuilt in place with their original filename, e.g. `Pack/TitleBG.pack`
  simply becomes a folder with the same name. However, actor packs are unbuilt fully into the 
  `Actor` folder and rebuilt from their actor link files. For example, if you were to make a new
  copy of `Actor/ActorLink/Enemy_Lizalfos_Senior.bxml.yml` in your project and name it
  `Enemy_Lizalfos_Geezer.bxml.yml`, this change alone would cause the build process to create a new
  actor pack at `Actor/Pack/Enemy_Lizalfos_Geezer.sbactorpack`.
- Some SARC files which are parsed by standard Nintendo libraries instead of the BOTW resource
  system will not be unbuilt for safety reasons.
  ```

## License

This software is licensed under the terms of the GNU General Public License, version 3 or later.
