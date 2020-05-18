import argparse
from pathlib import Path

from rstb import ResourceSizeTable, util
from . import builder, unbuilder


def rstb_to_json():
    parser = argparse.ArgumentParser(description="Converts a binary RSTB file to JSON.")
    parser.add_argument(
        "-b",
        "--be",
        action="store_true",
        help="Read the RSTB as big endian for Wii U, otherwise little endian for Switch",
    )
    parser.add_argument("rstb", help="Path to a binary RSTB file", nargs="?")
    parser.add_argument("-o", "--output", help="Path to output JSON file", nargs="?")
    args = parser.parse_args()

    in_file = Path(args.rstb).resolve()
    table = util.read_rstb(str(in_file), be=args.be)
    output = (
        Path(args.output).resolve()
        if args.output
        else in_file.with_suffix(".rsizetable.json")
    )
    unbuilder.rstb_to_json(table, output, set())


def json_to_rstb():
    parser = argparse.ArgumentParser(description="Converts a JSON RSTB file to binary.")
    parser.add_argument(
        "-b",
        "--be",
        action="store_true",
        help="Write the RSTB as big endian for Wii U, otherwise little endian for Switch",
    )
    parser.add_argument("json", help="Path to a JSON RSTB file", nargs="?")
    parser.add_argument("-o", "--output", help="Path to output RSTB binary", nargs="?")
    args = parser.parse_args()

    in_file = Path(args.json).resolve()
    table = builder.load_rstb(be=args.be, file=in_file)
    output = (
        Path(args.output).resolve()
        if args.output
        else in_file.with_suffix("").with_suffix(".srsizetable")
    )
    util.write_rstb(table, str(output), be=args.be)
