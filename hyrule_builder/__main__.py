import argparse
import textwrap as _textwrap
import re
from . import unbuilder, builder

def main() -> None:
    """ Main Hyrule Builder function """
    parser = argparse.ArgumentParser(description='Builds and unbuilds BOTW mods for Wii U')
    subparsers = parser.add_subparsers(dest='command', help='Command')
    subparsers.required = True

    b_parser = subparsers.add_parser(
        'build',
        description='Builds a mod into a source-like structure for editing',
        aliases=['b']
    )
    b_parser.add_argument('--be', '-B', help='Use big endian where applicable', action='store_true')
    b_parser.add_argument('--no-guess', '-G', help='Do not use RSTB estimates', action='store_true')
    b_parser.add_argument('--output', '-O', help='Output folder for built mod')
    b_parser.set_defaults(func=builder.build_mod)

    u_parser = subparsers.add_parser(
        'unbuild',
        description='Unbuilds a mod into a source-like structure for editing',
        aliases=['u']
    )
    u_parser.add_argument('--output', '-O', help='Output folder for unbuilt mod')
    u_parser.set_defaults(func=unbuilder.unbuild_mod)

    dir_help = """\
The main mod folder. For Wii U, this must contain a `content` folder and/or an `aoc` folder\
(the latter for DLC files). For Switch, you must use the following layout:
atmosphere
 └─ titles
    ├─ 01007EF00011E000 (for base game files)
    │  └─ romfs
    └─ 01007EF00011F001 (for DLC files)
       └─ romfs"""
    for sp in {b_parser, u_parser}:
        sp.formatter_class = PreserveWhiteSpaceWrapRawTextHelpFormatter
        sp.add_argument('directory', help=dir_help)
        sp.add_argument('--single', '-S', help='Run with single thread', action='store_true')
        sp.add_argument('--verbose', '-V', help='Provide more detailed output', action='store_true')

    args = parser.parse_args()
    args.func(args)

class PreserveWhiteSpaceWrapRawTextHelpFormatter(argparse.RawDescriptionHelpFormatter):
    def __add_whitespace(self, idx, iWSpace, text):
        if idx is 0:
            return text
        return (" " * iWSpace) + text

    def _split_lines(self, text, width):
        textRows = text.splitlines()
        for idx,line in enumerate(textRows):
            search = re.search('\s*[0-9\-]{0,}\.?\s*', line)
            if line.strip() is "":
                textRows[idx] = " "
            elif search:
                lWSpace = search.end()
                lines = [self.__add_whitespace(i,lWSpace,x) for i,x in enumerate(_textwrap.wrap(line, width))]
                textRows[idx] = lines

        return [item for sublist in textRows for item in sublist]

if __name__ == "__main__":
    main()
