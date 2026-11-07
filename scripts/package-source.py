#!/usr/bin/env python3
"""Snapshot project sources, excluding private inputs and generated files.

This archive contains FabSTEP sources and lockfiles, not vendored third-party
sources. It is a local development deliverable, not a complete distribution kit.
"""
import argparse
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
TREES = ("crates", "ui", "scripts", "fixtures", "docs", "packaging", ".github")
EXCLUDED = {"target", "node_modules", "dist", "gen", ".vite", "__pycache__", ".DS_Store"}
SOURCE_SUFFIXES = {".rs", ".toml", ".lock", ".json", ".stl", ".3mf", ".py", ".ts", ".js", ".svelte", ".css", ".svg", ".png", ".html", ".sh", ".md", ".yml", ".yaml", ".txt", ".cpp", ".h", ".hpp"}
SOURCE_NAMES = {"LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "LICENSE_MIT", "LICENSE_APACHE-2.0", ".gitignore"}


def source_files():
    root_files = sorted(ROOT.glob("*.md")) + [ROOT / name for name in ("Cargo.toml", "Cargo.lock", "LICENSE", ".gitignore")]
    paths = list(root_files)
    for tree in TREES:
        for folder, directories, files in os.walk(ROOT / tree):
            directories[:] = sorted(name for name in directories if name not in EXCLUDED)
            for name in directories:
                if (Path(folder) / name).is_symlink():
                    raise ValueError(f"Symbolic directory not allowed: {Path(folder) / name}")
            paths.extend(Path(folder) / name for name in files)
    for path in sorted(set(paths)):
        relative = path.relative_to(ROOT)
        if any(part in EXCLUDED for part in relative.parts) or path.suffix == ".pyc":
            continue
        if path.is_symlink():
            raise ValueError(f"Symbolic link not allowed in source snapshot: {relative}")
        if path.is_file():
            if any(part.startswith(".") and part not in {".github", ".gitignore"} for part in relative.parts):
                raise ValueError(f"Review hidden source file before archiving: {relative}")
            if path.suffix not in SOURCE_SUFFIXES and path.name not in SOURCE_NAMES:
                raise ValueError(f"Review unexpected source file before archiving: {relative}")
            yield path, relative.as_posix()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="New .tar.gz output (never overwritten)")
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(ROOT) and output.relative_to(ROOT).parts[0] in TREES:
        parser.error("Place the archive outside the source trees (for example artifacts/release/).")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    prefix = f"FabSTEP-{version}"
    entries = [(relative, path.read_bytes(), path.stat().st_mode & 0o111 != 0) for path, relative in source_files()]
    manifest = {"version": version, "scope": "FabSTEP project sources; dependencies pinned by lockfiles, not vendored", "files": {name: hashlib.sha256(data).hexdigest() for name, data, _ in entries}}
    entries.append(("SOURCE-MANIFEST.json", (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode(), False))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("xb") as output, gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=0) as compressed, tarfile.open(fileobj=compressed, mode="w|") as archive:
        for name, data, executable in entries:
            info = tarfile.TarInfo(f"{prefix}/{name}")
            info.size, info.mode, info.mtime = len(data), 0o755 if executable else 0o644, 0
            archive.addfile(info, io.BytesIO(data))
    print(json.dumps({"archive": str(args.output), "sha256": hashlib.sha256(args.output.read_bytes()).hexdigest(), "files": len(entries), "version": version}))


if __name__ == "__main__":
    main()
