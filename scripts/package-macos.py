#!/usr/bin/env python3
"""Package and verify an offline, ad-hoc-signed Apple Silicon release."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import platform
import plistlib
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parent.parent
MINIMUM = "26.0"
ARCH = "arm64"
APPLE_LIBRARIES = ("/System/Library/", "/usr/lib/")
FREETYPE_LICENSE_SHA = "5a5ee54c5001bbad1cdc1a57cc3dd4c42199b2da09d39c7ee41fab002d02967f"
NATIVE_LICENSES = {
    "freetype": ("LICENSE.TXT",),
    "libpng": ("LICENSE",),
    "opencascade": (
        "share/doc/opencascade/LICENSE_LGPL_21.txt",
        "share/doc/opencascade/OCCT_LGPL_EXCEPTION.txt",
    ),
    "tbb": ("LICENSE.txt",),
}
WEB_PACKAGES = ("@fontsource-variable/inter", "@lucide/svelte", "svelte", "three")
RUST_NOTICES = ROOT / "packaging/macos/rust-notices"


def require(condition, message):
    if not condition:
        raise ValueError(message)


def run(*args, **kwargs):
    return subprocess.run(args, capture_output=True, text=True, check=True, **kwargs).stdout


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def version_tuple(version):
    return tuple(int(part) for part in version.split("."))


def json_read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def json_write(path, content):
    path.write_text(json.dumps(content, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def preflight():
    require(sys.platform == "darwin" and platform.machine() == ARCH, "An Apple Silicon Mac is required.")
    require(version_tuple(platform.mac_ver()[0]) >= version_tuple(MINIMUM), f"macOS {MINIMUM} is required.")
    config = json_read(ROOT / "crates/fabstep-desktop/tauri.conf.json")
    desktop = tomllib.loads((ROOT / "crates/fabstep-desktop/Cargo.toml").read_text())
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text())
    lock = json_read(ROOT / "ui/package-lock.json")
    version = config["version"]
    require(
        version == desktop["package"]["version"] == workspace["workspace"]["package"]["version"]
        == lock["packages"][""]["version"] == json_read(ROOT / "ui/package.json")["version"],
        "Desktop, workspace, Tauri and UI versions must agree.",
    )
    for path, entry in lock["packages"].items():
        if not path:
            continue
        require(path.startswith("node_modules/") and not entry.get("link"), f"Nonportable UI lock entry: {path}")
        manifest = ROOT / "ui" / path / "package.json"
        if not manifest.exists() and entry.get("optional"):
            continue
        require(
            manifest.is_file() and json_read(manifest)["version"] == entry["version"],
            f"UI dependency {path} does not match package-lock.json; run npm --prefix ui ci.",
        )
    return config


def inspect_image(path):
    arch = run("/usr/bin/lipo", "-archs", path).strip()
    require(arch == ARCH, f"Expected {ARCH} only, found {arch}: {path}")
    build = run("/usr/bin/vtool", "-show-build", path)
    minimum = re.findall(r"\bminos ([0-9.]+)", build)
    require(len(minimum) == 1 and re.search(r"\bplatform MACOS\b", build), f"No macOS build version: {path}")
    require(version_tuple(minimum[0]) <= version_tuple(MINIMUM), f"{path} requires macOS {minimum[0]}.")
    names = run("/usr/bin/otool", "-D", path).splitlines()[1:]
    install_id = names[0].strip() if names else None
    dependencies = [
        line.strip().split(" (compatibility", 1)[0]
        for line in run("/usr/bin/otool", "-L", path).splitlines()[1:]
    ]
    if install_id is not None:
        dependencies.remove(install_id)
    load_commands = run("/usr/bin/otool", "-l", path)
    rpaths = re.findall(r"cmd LC_RPATH\s+cmdsize \d+\s+path (.*?) \(offset", load_commands)
    return {"install_id": install_id, "minimum": minimum[0], "dependencies": dependencies, "rpaths": rpaths}


def expand_loader(path, loader):
    if path == "@loader_path":
        return loader
    if path.startswith("@loader_path/"):
        return loader / path.removeprefix("@loader_path/")
    require(path.startswith("/"), f"Unsupported Mach-O search path: {path}")
    return Path(path)


def discover(entry, cellar):
    """Resolve the local Homebrew closure before modifying any load command."""
    pending = [entry]
    images = {}
    while pending:
        path = pending.pop()
        if path in images:
            continue
        image = inspect_image(path)
        resolved = {}
        for name in image["dependencies"]:
            if name.startswith(APPLE_LIBRARIES):
                continue
            if name.startswith("@rpath/"):
                candidates = [
                    expand_loader(rpath, path.parent) / name.removeprefix("@rpath/")
                    for rpath in image["rpaths"]
                ]
            else:
                candidates = [expand_loader(name, path.parent)]
            matches = {candidate.resolve() for candidate in candidates if candidate.is_file()}
            require(len(matches) == 1, f"Cannot uniquely resolve {name} from {path}: {sorted(matches)}")
            dependency = matches.pop()
            require(dependency.is_relative_to(cellar), f"Dependency is outside the Homebrew Cellar: {dependency}")
            resolved[name] = dependency
            pending.append(dependency)
        image["resolved"] = resolved
        images[path] = image
    return images


def library_names(images, entry):
    libraries = {}
    for path, image in images.items():
        if path == entry:
            continue
        require(image["install_id"] is not None, f"No dylib install name: {path}")
        name = Path(image["install_id"]).name
        require(name.endswith(".dylib") and name not in libraries.values(), f"Duplicate or invalid dylib name: {name}")
        libraries[path] = name
    return libraries


def relink(source, destination, image, names, library):
    # copyfile discards Homebrew xattrs and read-only modes before install_name_tool.
    shutil.copyfile(source, destination)
    destination.chmod(0o755)
    changes = []
    if library:
        changes.extend(("-id", "@loader_path/" + destination.name))
    prefix = "@loader_path/" if library else "@loader_path/../Frameworks/"
    for old, dependency in image["resolved"].items():
        new = prefix + names[dependency]
        if old != new:
            changes.extend(("-change", old, new))
    for rpath in image["rpaths"]:
        changes.extend(("-delete_rpath", rpath))
    if changes:
        run("/usr/bin/install_name_tool", *changes, destination)
    run("/usr/bin/codesign", "--force", "--sign", "-", destination)


def copy_notices(resources, libraries, cellar, metadata):
    notices = resources / "Licenses"
    notices.mkdir()
    shutil.copyfile(ROOT / "LICENSE", notices / "FabSTEP-GPL-3.0.txt")
    packages = {}
    for dependency in libraries:
        formula, version = dependency.relative_to(cellar).parts[:2]
        if formula in packages:
            require(packages[formula]["keg_version"] == version, f"Multiple versions for {formula}")
            continue
        require(formula in NATIVE_LICENSES, f"No license policy for native package {formula}")
        keg = cellar / formula / version
        text = (keg / ".brew" / (formula + ".rb")).read_text(encoding="utf-8")
        provenance = {}
        for key in ("license", "url", "sha256"):
            match = re.search(rf'^  {key} "([^"]+)"', text, re.MULTILINE)
            require(match, f"No {key} in Homebrew recipe for {formula}.")
            provenance[key] = match[1]
        folder = notices / formula
        folder.mkdir()
        for path in NATIVE_LICENSES[formula]:
            source = keg / path
            shutil.copyfile(source, folder / source.name)
        if formula == "freetype":
            require(version == "2.14.3", f"No pinned FreeType license for {version}.")
            license_file = ROOT / "packaging/macos/freetype-2.14.3-FTL.txt"
            require(digest(license_file) == FREETYPE_LICENSE_SHA, "The pinned FreeType license has changed.")
            shutil.copyfile(license_file, folder / "FTL.TXT")
            (folder / "CREDIT.txt").write_text(
                "FabSTEP is based in part on the work of the FreeType Team.\nhttps://freetype.org\n",
                encoding="utf-8",
            )
            provenance["license_text_url"] = "https://raw.githubusercontent.com/freetype/freetype/VER-2-14-3/docs/FTL.TXT"
            provenance["license_text_sha256"] = FREETYPE_LICENSE_SHA
        if formula == "opencascade":
            provenance["license_exception"] = "OCCT_LGPL_EXCEPTION.txt"
        packages[formula] = {
            "keg_version": version, "license": provenance.pop("license"), "source": provenance,
            "formula_sha256": digest(keg / ".brew" / (formula + ".rb")),
        }
    lock = json_read(ROOT / "ui/package-lock.json")["packages"]
    web = {}
    for package in WEB_PACKAGES:
        entry = lock["node_modules/" + package]
        folder = notices / "web" / package
        folder.mkdir(parents=True)
        files = notice_files(ROOT / "ui/node_modules" / package)
        require(files, f"Missing license for {package}.")
        for path in files:
            shutil.copyfile(path, folder / path.name)
        web[package] = {key: entry[key] for key in ("version", "license", "resolved", "integrity")}
    json_write(notices / "inventory.json", {"native": packages, "web": web, "rust": rust_notices(notices, metadata)})


def rust_notices(notices, metadata):
    cargo = {}
    rust = json_read(metadata)
    resolve = rust["resolve"]
    graph = {node["id"]: node["deps"] for node in resolve["nodes"]}
    pending = [resolve["root"]]
    selected = set()
    while pending:
        key = pending.pop()
        if key in selected:
            continue
        selected.add(key)
        pending.extend(dep["pkg"] for dep in graph[key] if any(kind["kind"] != "dev" for kind in dep["dep_kinds"]))
    supplements = json_read(RUST_NOTICES / "provenance.json")
    for package in rust["packages"]:
        if package["source"] is None or package["id"] not in selected:
            continue
        key = f"{package['name']}-{package['version']}"
        folder = notices / "rust" / key
        base = Path(package["manifest_path"]).parent
        vcs = base / ".cargo_vcs_info.json"
        revision = json_read(vcs).get("git", {}).get("sha1") if vcs.is_file() else None
        files = set(notice_files(base))
        if package["license_file"]:
            files.add(base / package["license_file"])
        require(key not in cargo, f"Duplicate Cargo license provenance for {key}.")
        locations = {}
        if files:
            folder.mkdir(parents=True)
            for path in sorted(files):
                destination = folder / path.name
                shutil.copyfile(path, destination)
                locations[str(destination.relative_to(notices))] = {"sha256": digest(destination)}
        else:
            repository = (package["repository"] or "").rstrip("/")
            for group, info in supplements.items():
                if info["repository"] != repository or revision not in info["revisions"]:
                    continue
                for name, provenance in info["files"].items():
                    source = RUST_NOTICES / group / name
                    require(digest(source) == provenance["sha256"], f"Changed upstream license text: {source}")
                    destination = notices / "rust" / "upstream" / group / name
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(source, destination)
                    locations[str(destination.relative_to(notices))] = {
                        "sha256": provenance["sha256"], "url": provenance["url"].format(revision=revision),
                    }
            require(locations, f"Missing license texts for Cargo package {key} ({repository}, {revision}).")
        cargo[key] = {
            "license": package["license"],
            "repository": package["repository"],
            "source": package["source"],
            "source_revision": revision,
            "notice_files": locations,
        }
    return cargo


def notice_files(directory):
    return sorted(
        path for path in directory.iterdir()
        if path.is_file() and path.name.upper().startswith(("LICENSE", "LICENCE", "COPYING", "NOTICE", "OFL"))
    )


def verify_app(app):
    contents = app / "Contents"
    executable = contents / "MacOS/FabSTEP"
    frameworks = contents / "Frameworks"
    info = plistlib.loads((contents / "Info.plist").read_bytes())
    manifest = json_read(contents / "Resources/dependencies.json")
    require(info["LSMinimumSystemVersion"] == manifest["minimum_macos"] == MINIMUM, "Incorrect deployment target.")
    require(info["FabSTEPReleaseVersion"] == manifest["version"], "Incorrect version.")
    require(manifest["architecture"] == ARCH, "Incorrect architecture.")
    require(not any(path.is_symlink() for path in app.rglob("*")), "Unexpected symlink in bundle.")
    libraries = {path.name: path for path in frameworks.iterdir()}
    require(libraries and set(libraries) == set(manifest["libraries"]), "Unexpected library set.")
    for path in (executable, *libraries.values()):
        image = inspect_image(path)
        library = path != executable
        record = manifest["libraries"][path.name] if library else manifest["executable"]
        require(image["minimum"] == record["minimum_macos"], f"Deployment target changed: {path.name}")
        require(image["dependencies"] == record["dependencies"], f"Dependencies changed: {path.name}")
        require(not image["rpaths"], f"Unexpected LC_RPATH in {path.name}")
        require(image["install_id"] == ("@loader_path/" + path.name if library else None), f"Incorrect install ID: {path.name}")
        if library:
            require(record["sha256"] == digest(path), f"Library hash mismatch: {path.name}")
        prefix = "@loader_path/" if library else "@loader_path/../Frameworks/"
        for name in image["dependencies"]:
            require(
                name.startswith(APPLE_LIBRARIES) or (name.startswith(prefix) and name[len(prefix):] in libraries),
                f"External or unresolved dependency {name} in {path.name}",
            )
    run("/usr/bin/codesign", "--verify", "--deep", "--strict", app)
    return manifest


def homebrew_sandbox(brew, witness):
    sandbox = Path("/usr/bin/sandbox-exec")
    if not sandbox.exists() or not witness.is_file():
        return []
    profile = f'(version 1)(allow default)(deny file-read* (subpath {json.dumps(str(brew.resolve()))}))'
    probe = subprocess.run(
        (sandbox, "-p", profile, "/bin/sh", "-c", 'test -r "$1"', "probe", witness),
        capture_output=True, text=True,
    )
    # SIP strips DYLD_* when starting sandbox-exec. Set it after entering the profile.
    return [sandbox, "-p", profile, "/bin/sh", "-c", 'DYLD_PRINT_LIBRARIES=1 exec "$@"', "trace"] if probe.returncode == 1 else []


def verify_smoke_conversion(fixture, manifest, surfaces, result, step):
    nominal = json_read(ROOT / "fixtures" / (manifest + "_manifest.json"))["cases"][Path(fixture).stem]
    references = sorted(nominal.get("bodies", [nominal]), key=lambda ref: ref["bounds_mm"])
    bodies = sorted((body["cad"] for body in result["bodies"]), key=lambda body: body["bounds_mm"])
    cad = result["cad"]
    face_types = ("planar_faces", "cylindrical_faces", "conical_faces", "spherical_faces", "toroidal_faces")
    require(
        result["native_validated"] and len(bodies) == cad["solids"] == len(references)
        and cad["faces"] == len(references) * sum(surfaces)
        and all(cad[key] == len(references) * count for key, count in zip(face_types, surfaces)),
        f"Unexpected native solid or surface count: {fixture}",
    )
    for body, reference in zip(bodies, references):
        require(
            body["solids"] == 1 and body["faces"] == sum(surfaces)
            and all(body[key] == count for key, count in zip(face_types, surfaces))
            and math.isclose(body["volume_mm3"], reference["volume_mm3"], rel_tol=1e-7)
            and all(math.isclose(a, b, rel_tol=0.0, abs_tol=3e-6)
                    for actual, nominal in zip(body["bounds_mm"], reference["bounds_mm"])
                    for a, b in zip(actual, nominal)),
            f"Unexpected volume, bounds or surfaces after STEP reimport: {fixture}",
        )
    require(step.startswith(b"ISO-10303-21;"), f"No STEP was exported for {fixture}.")
    entities = (b"PLANE(", b"CYLINDRICAL_SURFACE(", b"CONICAL_SURFACE(", b"SPHERICAL_SURFACE(", b"TOROIDAL_SURFACE(")
    require(
        all(step.count(entity) == len(references) * count for entity, count in zip(entities, surfaces)),
        f"Unexpected analytic STEP entities: {fixture}",
    )
    return {"fixture": fixture, **{key: cad[key] for key in ("solids", "faces", *face_types, "volume_mm3", "bounds_mm")}}


def smoke(cli, app, work, brew, require_sandbox, version):
    images = discover(cli, (brew / "Cellar").resolve())
    names = library_names(images, cli)
    frameworks = app / "Contents/Frameworks"
    require(set(names.values()) <= {path.name for path in frameworks.iterdir()}, "Smoke CLI needs additional dylibs.")
    destination = app / "Contents/MacOS/fabstep-smoke"
    try:
        relink(cli, destination, images[cli], names, False)
        sandbox = homebrew_sandbox(brew, next(iter(names)))
        require(sandbox or not require_sandbox, "The Homebrew read-denial profile is unavailable.")
        cases = [
            ("m5c_icosphere.stl", "m5c", (0, 0, 0, 1, 0)),
            ("m5c_sphere_instances.3mf", "m5c", (0, 0, 0, 1, 0)),
        ]
        if version_tuple(version.split("-", 1)[0]) >= (1, 0, 0):
            cases.extend([
                ("m5d_ring_torus.stl", "m5d", (0, 0, 0, 0, 1)),
                ("m6a_rounded_box_rotated.stl", "m6a", (6, 12, 0, 8, 0)),
            ])
        expected = {str((frameworks / name).resolve()) for name in names.values()}
        results = []
        for fixture, manifest, surfaces in cases:
            output = work / (fixture + ".step")
            process = subprocess.run(
                (*sandbox, destination, "convert", ROOT / "fixtures" / fixture, "--mode", "surfaces", "--json", "-o", output),
                cwd=work, env={"PATH": "/usr/bin:/bin", "DYLD_PRINT_LIBRARIES": "1"},
                capture_output=True, text=True, check=True, timeout=120,
            )
            loaded = [
                path for line in process.stderr.splitlines()
                if (match := re.match(r"dyld\[\d+\]:.*? (/.*)$", line)) and (path := match[1])
            ]
            actual = {str(Path(path).resolve()) for path in loaded if not path.startswith(APPLE_LIBRARIES)}
            require(expected <= actual, "The dyld trace did not confirm the full bundled library closure.")
            require(actual <= expected | {str(destination.resolve())}, f"Unexpected loaded images: {sorted(actual - expected)}")
            results.append(verify_smoke_conversion(fixture, manifest, surfaces, json.loads(process.stdout), output.read_bytes()))
        return {"homebrew_reads_blocked": bool(sandbox), "dyld_closure_confirmed": True, "conversions": results}
    finally:
        destination.unlink(missing_ok=True)


def verify_archive(archive, brew, cli=None, require_sandbox=False):
    checksum = archive.with_suffix(archive.suffix + ".sha256")
    expected = f"{digest(archive)}  {archive.name}\n"
    require(checksum.read_text() == expected, "ZIP SHA-256 mismatch.")
    with zipfile.ZipFile(archive) as zipped:
        for member in zipped.infolist():
            path = Path(member.filename)
            mode = stat.S_IFMT(member.external_attr >> 16)
            require(
                path.parts and path.parts[0] in ("FabSTEP.app", "__MACOSX")
                and ".." not in path.parts and mode in (0, stat.S_IFREG, stat.S_IFDIR),
                f"Unexpected ZIP entry: {member.filename}",
            )
    with tempfile.TemporaryDirectory(prefix="FabSTEP relocation ") as path:
        work = Path(path)
        unpacked = work / "Application déplacée"
        run("/usr/bin/ditto", "-x", "-k", archive, unpacked)
        app = unpacked / "FabSTEP.app"
        require(list(unpacked.iterdir()) == [app], "Expected only FabSTEP.app in the ZIP.")
        manifest = verify_app(app)
        result = {"archive": archive.name, "sha256": expected.split()[0], "libraries": len(manifest["libraries"]),
                  "minimum_macos": MINIMUM, "architecture": ARCH, "signature_verified": True}
        if cli:
            result["smoke"] = smoke(cli.resolve(strict=True), app, work, brew, require_sandbox, manifest["version"])
            verify_app(app)
        return result


def package(args):
    config = preflight()
    version = config["version"]
    numeric = version.split("-", 1)[0]
    require(re.fullmatch(r"\d+\.\d+\.\d+", numeric), f"Invalid version: {version}")
    output = (args.output or ROOT / "artifacts/release" / version).resolve()
    require(not output.exists(), f"Output already exists: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    cellar = (args.brew_prefix / "Cellar").resolve()
    entry = args.desktop.resolve(strict=True)
    images = discover(entry, cellar)
    names = library_names(images, entry)
    require(images[entry]["minimum"] == MINIMUM, f"Desktop must be compiled with MACOSX_DEPLOYMENT_TARGET={MINIMUM}.")
    require(names, "Expected a dynamically linked Open CASCADE library closure.")
    with tempfile.TemporaryDirectory(prefix=".fabstep-release-", dir=output.parent) as path:
        stage = Path(path) / "candidate"
        app = stage / "FabSTEP.app"
        contents = app / "Contents"
        macos = contents / "MacOS"
        frameworks = contents / "Frameworks"
        resources = contents / "Resources"
        for folder in (macos, frameworks, resources):
            folder.mkdir(parents=True)
        executable = macos / "FabSTEP"
        relink(entry, executable, images[entry], names, False)
        libraries = {}
        for source, name in sorted(names.items(), key=lambda item: item[1]):
            destination = frameworks / name
            relink(source, destination, images[source], names, True)
            image = inspect_image(destination)
            libraries[name] = {
                "keg_path": str(source.relative_to(cellar)),
                "original_sha256": digest(source), "sha256": digest(destination),
                "dependencies": image["dependencies"], "minimum_macos": image["minimum"],
            }
        image = inspect_image(executable)
        json_write(resources / "dependencies.json", {
            "architecture": ARCH, "minimum_macos": MINIMUM, "version": version, "libraries": libraries,
            "executable": {"minimum_macos": image["minimum"], "dependencies": image["dependencies"]},
            "lockfiles_sha256": {
                name: digest(ROOT / name) for name in ("crates/fabstep-desktop/Cargo.lock", "ui/package-lock.json")
            },
        })
        copy_notices(resources, names, cellar, args.cargo_metadata)
        shutil.copyfile(ROOT / "crates/fabstep-desktop/icons/icon.png", resources / "icon.png")
        (contents / "Info.plist").write_bytes(plistlib.dumps({
            "CFBundleDisplayName": "FabSTEP", "CFBundleExecutable": "FabSTEP", "CFBundleIconFile": "icon.png",
            "CFBundleIdentifier": config["identifier"], "CFBundleName": "FabSTEP", "CFBundlePackageType": "APPL",
            "CFBundleShortVersionString": numeric, "CFBundleVersion": numeric, "FabSTEPReleaseVersion": version,
            "LSMinimumSystemVersion": MINIMUM, "NSHighResolutionCapable": True,
        }))
        run("/usr/bin/codesign", "--force", "--sign", "-", app)
        verify_app(app)
        archive = stage / f"FabSTEP-{version}-macOS-arm64.zip"
        run("/usr/bin/ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", app, archive)
        archive.with_suffix(".zip.sha256").write_text(f"{digest(archive)}  {archive.name}\n", encoding="ascii")
        validation = verify_archive(archive, args.brew_prefix, args.smoke_cli.resolve(strict=True), args.require_no_homebrew)
        json_write(stage / "validation.json", validation)
        require(not output.exists(), f"Output already exists: {output}")
        stage.rename(output)
    print(output / archive.name)


def main():
    import os

    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("preflight")
    builder = commands.add_parser("package")
    builder.add_argument("desktop", type=Path)
    builder.add_argument("output", type=Path, nargs="?")
    builder.add_argument("--cargo-metadata", type=Path, required=True)
    verifier = commands.add_parser("verify")
    verifier.add_argument("archive", type=Path)
    for command in (builder, verifier):
        command.add_argument("--brew-prefix", type=Path, default=Path(os.environ.get("HOMEBREW_PREFIX", "/opt/homebrew")))
        command.add_argument("--smoke-cli", type=Path, required=command is builder)
        command.add_argument("--require-no-homebrew", action="store_true")
    args = parser.parse_args()
    try:
        if args.command == "preflight":
            preflight()
        elif args.command == "package":
            package(args)
        else:
            require(args.smoke_cli or not args.require_no_homebrew, "--require-no-homebrew requires --smoke-cli.")
            print(json.dumps(verify_archive(args.archive.resolve(), args.brew_prefix, args.smoke_cli, args.require_no_homebrew), indent=2))
    except subprocess.CalledProcessError as error:
        print(f"macOS release: {error}\n{error.stderr}", file=sys.stderr)
        return 1
    except (ValueError, OSError, KeyError, subprocess.TimeoutExpired, zipfile.BadZipFile) as error:
        print(f"macOS release: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
