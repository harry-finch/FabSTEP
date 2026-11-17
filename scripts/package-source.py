#!/usr/bin/env python3
"""Snapshot reviewed Git-tracked sources from a clean working tree.

This archive contains FabSTEP sources and lockfiles, not vendored third-party
sources. It is a local development deliverable, not a complete distribution kit.
"""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
TREES = ("crates", "ui", "scripts", "fixtures", "docs", "packaging", ".github")
EXCLUDED = {"target", "node_modules", "dist", "gen", ".vite", "__pycache__", ".DS_Store"}
SOURCE_SUFFIXES = {".rs", ".toml", ".lock", ".json", ".stl", ".3mf", ".py", ".ts", ".js", ".svelte", ".css", ".svg", ".png", ".html", ".sh", ".md", ".yml", ".yaml", ".txt", ".cpp", ".h", ".hpp"}
SOURCE_NAMES = {"LICENSE", "LICENSE-MIT", "LICENSE-APACHE", "LICENSE_MIT", "LICENSE_APACHE-2.0", ".gitignore"}


def git_output(*arguments):
    try:
        return subprocess.run(
            ["git", "--no-optional-locks", "-C", str(ROOT), *arguments],
            check=True, capture_output=True,
        ).stdout
    except (OSError, subprocess.CalledProcessError) as error:
        raise ValueError("A readable Git repository is required for the source snapshot.") from error


def require_clean_repository():
    top_level = Path(git_output("rev-parse", "--show-toplevel").decode().strip())
    if top_level.resolve() != ROOT.resolve():
        raise ValueError("The source root must be the Git repository root.")
    if git_output("status", "--porcelain=v1", "-z", "--untracked-files=all"):
        raise ValueError(
            "Source snapshot requires a clean Git working tree, including untracked files; "
            "review and commit the intended sources first. No archive was created."
        )


def source_files():
    require_clean_repository()
    tracked = git_output("ls-files", "--cached", "-z").split(b"\0")
    for name in sorted(set(tracked) - {b""}):
        relative = Path(name.decode("utf-8"))
        root_file = len(relative.parts) == 1 and (
            relative.suffix == ".md" or relative.name in {"Cargo.toml", "Cargo.lock", "LICENSE", ".gitignore"}
        )
        if not root_file and relative.parts[0] not in TREES:
            continue
        path = ROOT / relative
        if any(part in EXCLUDED for part in relative.parts) or path.suffix == ".pyc":
            continue
        for parent in relative.parents:
            if parent != Path(".") and (ROOT / parent).is_symlink():
                raise ValueError(f"Symbolic directory not allowed: {parent}")
        if path.is_symlink():
            raise ValueError(f"Symbolic link not allowed in source snapshot: {relative}")
        if path.is_file():
            if any(part.startswith(".") and part not in {".github", ".gitignore"} for part in relative.parts):
                raise ValueError(f"Review hidden source file before archiving: {relative}")
            if path.suffix not in SOURCE_SUFFIXES and path.name not in SOURCE_NAMES:
                raise ValueError(f"Review unexpected source file before archiving: {relative}")
            yield path, relative.as_posix()
        else:
            raise ValueError(f"Tracked source is not a regular file: {relative}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="New .tar.gz output (never overwritten)")
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(ROOT) and output.relative_to(ROOT).parts[0] in TREES:
        parser.error("Place the archive outside the source trees (for example artifacts/release/).")
    try:
        entries = [(relative, path.read_bytes(), path.stat().st_mode & 0o111 != 0) for path, relative in source_files()]
        version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    except ValueError as error:
        parser.error(str(error))
    prefix = f"FabSTEP-{version}"
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
