"""Dependency manifest checks that also run on hosts without Mach-O tools."""

import importlib.util
import json
from pathlib import Path
import plistlib
import sys
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "package_macos", Path(__file__).resolve().parents[1] / "package-macos.py"
)
PACKAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGE)


class MacReleaseTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform == "darwin", "macOS atomic directory publication")
    def test_publication_preserves_existing_empty_directory(self):
        with tempfile.TemporaryDirectory() as parent:
            output = Path(parent) / "release"
            output.mkdir()
            with tempfile.TemporaryDirectory(dir=parent) as private:
                candidate = Path(private) / "candidate"
                candidate.mkdir()
                (candidate / "payload").write_bytes(b"new")
                with self.assertRaises(FileExistsError):
                    PACKAGE.publish_directory(candidate, output)
                self.assertEqual((candidate / "payload").read_bytes(), b"new")
            self.assertEqual(list(Path(parent).iterdir()), [output])
            self.assertEqual(list(output.iterdir()), [])

    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.app = Path(temporary.name) / "FabSTEP.app"
        self.contents = self.app / "Contents"
        self.frameworks = self.contents / "Frameworks"
        self.macos = self.contents / "MacOS"
        self.resources = self.contents / "Resources"
        for folder in (self.frameworks, self.macos, self.resources):
            folder.mkdir(parents=True)
        self.images = {}
        self.executables = {}
        self.libraries = {}
        for name, dependencies in [
            ("shared.dylib", []),
            ("worker-only.dylib", ["@loader_path/shared.dylib"]),
            ("FabSTEP", ["@loader_path/../Frameworks/shared.dylib"]),
            ("fabstep-worker", ["@loader_path/../Frameworks/worker-only.dylib"]),
        ]:
            library = name.endswith(".dylib")
            path = (self.frameworks if library else self.macos) / name
            path.write_bytes(name.encode())
            self.images[path] = {
                "install_id": "@loader_path/" + name if library else None,
                "minimum": PACKAGE.MINIMUM,
                "dependencies": dependencies,
                "rpaths": [],
            }
            record = {"minimum_macos": PACKAGE.MINIMUM, "dependencies": list(dependencies)}
            if name != "FabSTEP":
                record["sha256"] = PACKAGE.digest(path)
            (self.libraries if library else self.executables)[name] = record
        self.manifest = {
            "schema_version": 2,
            "version": "1.0.0",
            "architecture": PACKAGE.ARCH,
            "minimum_macos": PACKAGE.MINIMUM,
            "executables": self.executables,
            "libraries": self.libraries,
        }
        (self.contents / "Info.plist").write_bytes(
            plistlib.dumps(
                {
                    "FabSTEPReleaseVersion": "1.0.0",
                    "LSMinimumSystemVersion": PACKAGE.MINIMUM,
                }
            )
        )

    def verify(self):
        (self.resources / "dependencies.json").write_text(json.dumps(self.manifest))
        with (
            patch.object(PACKAGE, "inspect_image", side_effect=self.images.__getitem__),
            patch.object(PACKAGE, "run"),
        ):
            return PACKAGE.verify_app(self.app)

    def test_two_roots_with_shared_and_worker_only_libraries(self):
        self.assertEqual(self.verify(), self.manifest)

    def test_v2_requires_worker_with_matching_hash(self):
        (self.macos / "fabstep-worker").write_bytes(b"changed")
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            self.verify()
        (self.macos / "fabstep-worker").unlink()
        with self.assertRaisesRegex(ValueError, "executable set"):
            self.verify()
        del self.executables["fabstep-worker"]
        with self.assertRaisesRegex(ValueError, "requires the desktop and its worker"):
            self.verify()

    def test_worker_cannot_use_build_host_dylibs(self):
        external = ["/opt/homebrew/lib/worker-only.dylib"]
        self.executables["fabstep-worker"]["dependencies"] = external
        self.images[self.macos / "fabstep-worker"]["dependencies"] = external
        with self.assertRaisesRegex(ValueError, "External or unresolved"):
            self.verify()

    def test_historical_single_executable_manifest(self):
        del self.manifest["schema_version"]
        self.manifest["executable"] = self.manifest.pop("executables")["FabSTEP"]
        (self.macos / "fabstep-worker").unlink()
        (self.frameworks / "worker-only.dylib").unlink()
        del self.libraries["worker-only.dylib"]
        self.assertEqual(self.verify(), self.manifest)

    def test_cargo_notices_include_dependencies_unique_to_each_executable(self):
        roots = {}
        for role in ("desktop", "worker"):
            root = f"fabstep-{role}"
            metadata = self.resources / f"{role}.json"
            roots[root] = metadata
            packages = []
            ids = (root, "fabstep-core", "shared", role + "-only")
            for key in ids:
                directory = self.resources / key
                directory.mkdir(exist_ok=True)
                (directory / "LICENSE").write_text("MIT")
                packages.append(
                    {
                        "name": key,
                        "id": key,
                        "version": "1",
                        "source": None if key.startswith("fabstep-") else "registry",
                        "manifest_path": str(directory / "Cargo.toml"),
                        "license": "MIT",
                        "license_file": None,
                        "repository": None,
                    }
                )
            nodes = [
                {
                    "id": key,
                    "features": ["large-stl"] if (role, key) == ("worker", "fabstep-core") else [],
                    "deps": [{"pkg": dep, "dep_kinds": [{"kind": None}]} for dep in ids[1:]]
                    if key == root
                    else [],
                }
                for key in ids
            ]
            metadata.write_text(json.dumps({"packages": packages, "resolve": {"root": root, "nodes": nodes}}))
        self.assertEqual(
            set(PACKAGE.rust_notices(self.resources, roots)), {"shared-1", "desktop-only-1", "worker-only-1"}
        )
        desktop = roots["fabstep-desktop"]
        data = json.loads(desktop.read_text())
        data["resolve"]["nodes"][1]["features"].append("large-stl")
        desktop.write_text(json.dumps(data))
        with self.assertRaisesRegex(ValueError, "source-size profile"):
            list(PACKAGE.cargo_packages(desktop, "fabstep-desktop"))
