"""Collect the notices for the native desktop and CLI built on this host."""
import argparse
import runpy
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("destination", type=Path, help="New Licenses directory in the package")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    packaging = runpy.run_path(str(root / "scripts/package-macos.py"))
    target = next(line.removeprefix("host: ") for line in
                  subprocess.check_output(["rustc", "-vV"], text=True).splitlines()
                  if line.startswith("host: "))
    args.destination.parent.mkdir(parents=True, exist_ok=True)
    if args.destination.exists():
        parser.error("The destination must be new.")
    with tempfile.TemporaryDirectory(dir=args.destination.parent) as stage:
        stage = Path(stage)
        metadata = {}
        for crate in ("fabstep-desktop", "fabstep-cli"):
            command = ["cargo", "metadata", "--locked", "--format-version", "1",
                       "--manifest-path", str(root / "crates" / crate / "Cargo.toml"),
                       "--filter-platform", target]
            if crate == "fabstep-desktop":
                command.extend(["--features", "custom-protocol"])
            metadata[crate] = stage / (crate + ".json")
            with metadata[crate].open("wb") as output:
                subprocess.run(command, stdout=output, check=True)
        packaging["copy_notices"](stage, [], None, metadata)
        (stage / "Licenses").rename(args.destination)


if __name__ == "__main__":
    main()
