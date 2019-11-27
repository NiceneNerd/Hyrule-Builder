from . import unbuilder, builder

def main() -> None:
    """ Main Hyrule Builder function """
    import argparse
    parser = argparse.ArgumentParser(description='Builds and unbuilds BOTW mods for Wii U')
    subparsers = parser.add_subparsers(dest='command', help='Command')
    subparsers.required = True

    b_parser = subparsers.add_parser(
        'build',
        description='Builds a mod into a source-like structure for editing',
        aliases=['b']
    )
    b_parser.add_argument(
        'directory',
        help='The main mod directory (containing `content` and/or `aoc` folder)'
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
    u_parser.add_argument(
        'directory',
        help='The main mod directory (containing `content` and/or `aoc` folder)'
    )
    u_parser.add_argument('--output', '-O', help='Output folder for unbuilt mod')
    u_parser.set_defaults(func=unbuilder.unbuild_mod)

    for sp in {b_parser, u_parser}:
        sp.add_argument('--single', '-S', help='Run with single thread', action='store_true')
        sp.add_argument('--verbose', '-V', help='Provide more detailed output', action='store_true')

    args = parser.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()
