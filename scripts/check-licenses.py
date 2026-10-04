#!/usr/bin/env python3
"""Check workspace license links and, optionally, Cargo package contents."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile

LICENSES = ("LICENSE-APACHE", "LICENSE-MIT")
ROOT = Path(__file__).resolve().parents[1]


def check(packages, target_directory, archives):
    errors = []
    canonical = {name: (ROOT / name).read_bytes() for name in LICENSES}
    index = subprocess.check_output(
        ["git", "ls-files", "--stage", "-z"], cwd=ROOT
    ).decode()
    modes = {}
    for entry in index.split("\0"):
        if entry:
            metadata, path = entry.split("\t", 1)
            mode, _, stage = metadata.split()
            if stage == "0":
                modes[path] = mode

    for package in packages:
        directory = Path(package["manifest_path"]).parent
        for name, content in canonical.items():
            path = directory / name
            relative = path.relative_to(ROOT).as_posix()
            expected = os.path.relpath(ROOT / name, directory).replace(os.sep, "/")
            if modes.get(relative) != "120000":
                errors.append(f"{relative}: must be committed as a Git symlink (120000)")
            if not path.is_symlink():
                errors.append(
                    f"{relative}: not a real symlink; use a symlink-enabled checkout "
                    "before packaging (see docs/licensing.md)"
                )
                continue
            if os.readlink(path) != expected:
                errors.append(f"{relative}: expected link target {expected!r}")
                continue
            if not path.is_file() or path.read_bytes() != content:
                errors.append(f"{relative}: does not resolve to the root license contents")

        if not archives:
            continue
        prefix = f"{package['name']}-{package['version']}"
        archive = Path(target_directory) / "package" / f"{prefix}.crate"
        try:
            with tarfile.open(archive, "r:gz") as crate:
                for name, content in canonical.items():
                    member = crate.getmember(f"{prefix}/{name}")
                    if not member.isfile():
                        errors.append(f"{archive.name}: {name} is not a regular file")
                        continue
                    with crate.extractfile(member) as source:
                        if source.read() != content:
                            errors.append(f"{archive.name}: {name} differs from the root license")
        except (OSError, tarfile.TarError, KeyError) as error:
            errors.append(f"{archive}: {error}")
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", action="append", help="check only this workspace package")
    parser.add_argument(
        "--archives", action="store_true", help="also check existing target/package/*.crate files"
    )
    args = parser.parse_args()
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=ROOT
        )
    )
    packages = [
        package
        for package in metadata["packages"]
        if package["id"] in metadata["workspace_members"] and package["publish"] != []
    ]
    if args.package:
        requested = set(args.package)
        unknown = requested - {package["name"] for package in packages}
        if unknown:
            parser.error(f"unknown publishable workspace package(s): {', '.join(sorted(unknown))}")
        packages = [package for package in packages if package["name"] in requested]
    errors = check(packages, metadata["target_directory"], args.archives)
    if errors:
        print("License integrity check failed:\n" + "\n".join(errors), file=sys.stderr)
        return 1
    scope = "links and package archives" if args.archives else "links"
    print(f"Verified license {scope} for {len(packages)} workspace packages.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
