# pylint: disable=bad-continuation,unsupported-assignment-operation
import argparse
from multiprocessing import set_start_method
from pathlib import Path
import oead
from . import AAMP_EXTS, BYML_EXTS, SARC_EXTS
from .unbuilder import _unbuild_sarc as unsarc


def unbuild_sarc() -> None:
    set_start_method("spawn", True)
    parser = argparse.ArgumentParser(
        description="Unbuild a single SARC file completely"
    )
    parser.add_argument("sarc", help="SARC archive to unbuild")
    parser.add_argument(
        "--output",
        "-O",
        help="Output folder for unbuilt SARC,\
        defaults to file name w/o extension",
    )
    args = parser.parse_args()

    try:
        file: Path = Path(args.sarc)
        data = file.read_bytes()
        if data[0:4] == b"Yaz0":
            data = oead.yaz0.decompress(data)
        unsarc(
            oead.Sarc(data),
            Path(args.output) if args.output else file.with_suffix(""),
            True,
        )
    except (FileNotFoundError, oead.InvalidDataError, OSError) as err:
        print(err)
        return


def build_sarc() -> None:
    set_start_method("spawn", True)
    parser = argparse.ArgumentParser(
        description="Build a SARC file from a single source folder"
    )
    parser.add_argument("source", help="Source folder for SARC")
    parser.add_argument(
        "output",
        help='Path to output SARC file, will auto compress if\
        extension starts with ".s"',
    )
    parser.add_argument(
        "--be", "-B", help="Use big endian where applicable", action="store_true"
    )
    parser.add_argument(
        "--verbose", "-V", help="Provide more detailed output", action="store_true"
    )
    args = parser.parse_args()

    source = Path(args.source)
    output = Path(args.output)

    def build_yaml(file: Path) -> bytes:
        real_file = file.with_suffix("")
        data: bytes
        if real_file.suffix in BYML_EXTS:
            data = oead.byml.to_binary(
                oead.byml.from_text(file.read_text("utf-8")), args.be
            )
        elif real_file.suffix in AAMP_EXTS:
            data = oead.aamp.ParameterIO.from_text(file.read_text("utf-8")).to_binary()
        else:
            raise TypeError("Can only build AAMP or BYML files from YAML")
        if real_file.suffix.startswith(".s"):
            data = oead.yaz0.compress(data)
        return data

    yml_table = {file: build_yaml(file) for file in source.rglob("**/*.yml")}
    all_files = {
        f
        for f in source.rglob("**/*")
        if f.is_file() or (f.is_dir() and f.suffix in SARC_EXTS)
    }
    nest_sarcs = {}

    for nest in sorted(
        {d for d in all_files if d.suffix in SARC_EXTS and d.is_dir()} | {source},
        key=lambda x: len(x.parts),
        reverse=True,
    ):
        if args.verbose:
            print(f"Building {nest.name}...")
        sarc = oead.SarcWriter(
            oead.Endianness.Big if args.be else oead.Endianness.Little
        )
        for file in all_files.copy():
            try:
                rel_path = file.relative_to(nest)
                assert file != nest
            except (ValueError, AssertionError):
                continue
            store_path = (
                rel_path.as_posix()
                if rel_path.suffix != ".yml"
                else rel_path.with_suffix("").as_posix()
            )
            if args.verbose:
                print(f"  Adding {store_path} to {nest.name}")
            sarc.files[store_path] = yml_table.pop(
                file, nest_sarcs.pop(file, None) or file.read_bytes()
            )
            all_files.remove(file)

        if nest.suffix.startswith(".s") and nest.suffix != ".sarc":
            sarc_data = oead.yaz0.compress(sarc.write()[1])
        else:
            sarc_data = sarc.write()[1]
        nest_sarcs[nest] = sarc_data
        if args.verbose:
            print(f"Finished building {nest.name}")

    output.write_bytes(nest_sarcs[source])


if __name__ == "__main__":
    build_sarc()
