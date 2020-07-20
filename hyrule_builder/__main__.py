import argparse
import textwrap as _textwrap
import re
from multiprocessing import set_start_method
from . import unbuilder, builder
from .__version__ import USER_VERSION


def main() -> None:
    """ Main Hyrule Builder function """
    set_start_method("spawn", True)
    parser = argparse.ArgumentParser(
        description="Builds and unbuilds BOTW mods for Wii U"
    )
    parser.add_argument("-V", "--version", action="store_true")
    subparsers = parser.add_subparsers(dest="command", help="Command")

    b_parser = subparsers.add_parser(
        "build",
        description="Builds a mod from a source-like structure into binary game files",
        aliases=["b"],
    )
    b_parser.add_argument(
        "--be", "-B", "-b", help="Use big endian where applicable", action="store_true"
    )
    b_parser.add_argument(
        "--no-rstb", "-R", help="Do not auto-update RSTB", action="store_true"
    )
    b_parser.add_argument(
        "--no-guess", "-G", help="Do not use RSTB estimates", action="store_true"
    )
    b_parser.add_argument(
        "--no-warn",
        "-W",
        help="Ignore warnings, only output success/error",
        action="store_true",
    )
    b_parser.add_argument(
        "--hard-warn",
        "-H",
        help="Abort on any warning like an error",
        action="store_true"
    )
    b_parser.add_argument(
        "--title-actors",
        "-T",
        help="Comma separated list of custom actors to add to TitleBG.pack, "
        "e.g.\n`--title-actors=Weapon_Bow_001,Enemy_Golem_Senior`",
        default="",
    )
    b_parser.add_argument("--output", "-O", help="Output folder for built mod")
    b_parser.set_defaults(func=builder.build_mod)

    u_parser = subparsers.add_parser(
        "unbuild",
        description="Unbuilds a mod into a source-like structure for editing",
        aliases=["u"],
    )
    u_parser.add_argument("--output", "-O", help="Output folder for unbuilt mod")
    u_parser.set_defaults(func=unbuilder.unbuild_mod)

    dir_help = """\
The main mod folder. For Wii U, this must contain a `content` folder and/or an `aoc` folder \
(the latter for DLC files). For Switch, you must use the following layout:
   .
   ├─ 01007EF00011E000 (for base game files)
   │  └─ romfs
   └─ 01007EF00011F001 (for DLC files)
      └─ romfs"""
    for sub in {b_parser, u_parser}:
        sub.formatter_class = PreserveWhiteSpaceWrapRawTextHelpFormatter
        sub.add_argument("directory", help=dir_help)
        sub.add_argument(
            "--single", "-S", help="Run with single thread", action="store_true"
        )
        sub.add_argument(
            "--verbose", "-V", help="Provide more detailed output", action="store_true"
        )

    args = parser.parse_args()
    if hasattr(args, "func"):
        args.func(args)
    else:
        if args.version:
            print(f"Hyrule Builder: version {USER_VERSION}")
        else:
            parser.print_help()


class PreserveWhiteSpaceWrapRawTextHelpFormatter(argparse.RawDescriptionHelpFormatter):
    def __add_whitespace(self, idx, i_whitespace, text):
        if idx == 0:
            return text
        return (" " * i_whitespace) + text

    def _split_lines(self, text, width):
        rows = text.splitlines()
        for idx, line in enumerate(rows):
            search = re.search(r"\s*[0-9\-]*\.?\s*", line)
            if line.strip() == "":
                rows[idx] = " "
            elif search:
                last_whitespace = search.end()
                lines = [
                    self.__add_whitespace(i, last_whitespace, x)
                    for i, x in enumerate(_textwrap.wrap(line, width))
                ]
                rows[idx] = lines

        return [item for sublist in rows for item in sublist]


if __name__ == "__main__":
    main()
