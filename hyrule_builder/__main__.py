import argparse
import textwrap as _textwrap
import re
from . import unbuilder, builder


def main() -> None:
    """ Main Hyrule Builder function """
    parser = argparse.ArgumentParser(description='Builds and unbuilds BOTW mods for Wii U')
    parser.add_argument('-V', '--version', action='store_true')
    subparsers = parser.add_subparsers(dest='command', help='Command')

    b_parser = subparsers.add_parser(
        'build',
        description='Builds a mod into a source-like structure for editing',
        aliases=['b']
    )
    b_parser.add_argument('--be', '-B', help='Use big endian where applicable', action='store_true')
    b_parser.add_argument('--no-rstb', '-R', help='Do not auto-update RSTB', action='store_true')
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
The main mod folder. For Wii U, this must contain a `content` folder and/or an `aoc` folder \
(the latter for DLC files). For Switch, you must use the following layout:
atmosphere
└─ contents
   ├─ 01007EF00011E000 (for base game files)
   │  └─ romfs
   └─ 01007EF00011F001 (for DLC files)
      └─ romfs"""
    for sub in {b_parser, u_parser}:
        sub.formatter_class = PreserveWhiteSpaceWrapRawTextHelpFormatter
        sub.add_argument('directory', help=dir_help)
        sub.add_argument('--single', '-S', help='Run with single thread', action='store_true')
        sub.add_argument('--verbose', '-V', help='Provide more detailed output', action='store_true')

    args = parser.parse_args()
    if hasattr(args, 'func'):
        args.func(args)
    else:
        if args.version:
            from .__version__ import USER_VERSION
            print(f'Hyrule Builder: version {USER_VERSION}')
        else:
            parser.print_help()


class PreserveWhiteSpaceWrapRawTextHelpFormatter(argparse.RawDescriptionHelpFormatter):
    def __add_whitespace(self, idx, iWSpace, text):
        if idx is 0:
            return text
        return (" " * iWSpace) + text

    def _split_lines(self, text, width):
        textRows = text.splitlines()
        for idx, line in enumerate(textRows):
            search = re.search(r'\s*[0-9\-]*\.?\s*', line)
            if line.strip() is "":
                textRows[idx] = " "
            elif search:
                lWSpace = search.end()
                lines = [self.__add_whitespace(i, lWSpace, x) for i, x in enumerate(_textwrap.wrap(line, width))]
                textRows[idx] = lines

        return [item for sublist in textRows for item in sublist]


if __name__ == "__main__":
    main()
