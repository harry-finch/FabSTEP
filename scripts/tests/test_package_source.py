"""Source snapshot regressions, isolated from the real repository and index."""
import contextlib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "package_source", Path(__file__).resolve().parents[1] / "package-source.py"
)
PACKAGE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGE)


class SourceSnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="fabstep-source-test-")
        self.addCleanup(self.temporary.cleanup)
        self.directory = Path(self.temporary.name).resolve()
        self.root = self.directory / "repository"
        self.root.mkdir()
        self.output = self.directory / "snapshot.tar.gz"
        self.root_patch = patch.object(PACKAGE, "ROOT", self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)
        self.git("init", "-q")
        self.git("config", "user.name", "Source snapshot test")
        self.git("config", "user.email", "source-test@example.invalid")
        self.write("Cargo.toml", '[workspace.package]\nversion = "1.0.0"\n')
        self.write("README.md", "Reviewed source\n")
        self.write(".gitignore", "artifacts/\nignored.md\n")
        self.write("scripts/example.sh", "#!/bin/sh\nexit 0\n")
        (self.root / "scripts/example.sh").chmod(0o755)
        self.write("ui/src/example with spaces.ts", "export const value = 1;\n")
        self.commit()

    def git(self, *arguments):
        return subprocess.run(
            ["git", "-C", str(self.root), *arguments],
            check=True, capture_output=True,
        ).stdout

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def commit(self):
        self.git("add", "--all")
        self.git("commit", "-qm", "Reviewed fixture sources")

    def archive(self, output=None):
        output = output or self.output
        with patch.object(sys, "argv", ["package-source.py", str(output)]):
            with contextlib.redirect_stdout(io.StringIO()) as stdout:
                PACKAGE.main()
        return json.loads(stdout.getvalue())

    def assert_refused_without_output(self, message):
        with contextlib.redirect_stderr(io.StringIO()) as stderr:
            with self.assertRaises(SystemExit) as caught:
                self.archive()
        self.assertEqual(caught.exception.code, 2)
        self.assertIn(message, stderr.getvalue())
        self.assertFalse(self.output.exists())

    def test_clean_archive_hashes_modes_and_determinism(self):
        first = self.archive()
        second_path = self.directory / "second.tar.gz"
        second = self.archive(second_path)
        self.assertEqual(self.output.read_bytes(), second_path.read_bytes())
        self.assertEqual(first["sha256"], second["sha256"])
        # The gzip mtime is zero; each tar entry also has stable metadata.
        self.assertEqual(self.output.read_bytes()[4:8], b"\0" * 4)
        with tarfile.open(self.output, "r:gz") as archive:
            names = archive.getnames()
            prefix = "FabSTEP-1.0.0/"
            manifest = json.load(archive.extractfile(prefix + "SOURCE-MANIFEST.json"))
            self.assertEqual(len(names), len(manifest["files"]) + 1)
            for name, digest in manifest["files"].items():
                info = archive.getmember(prefix + name)
                self.assertTrue(info.isfile())
                self.assertEqual(info.mtime, 0)
                self.assertEqual(hashlib.sha256(archive.extractfile(info).read()).hexdigest(), digest)
            self.assertEqual(archive.getmember(prefix + "scripts/example.sh").mode, 0o755)
            self.assertEqual(archive.getmember(prefix + "README.md").mode, 0o644)
            self.assertIn(prefix + "ui/src/example with spaces.ts", names)

    def test_untracked_source_rejected(self):
        self.write("docs/private-notes.md", "Never distribute\n")
        self.output = self.directory / "not-created/snapshot.tar.gz"
        self.assert_refused_without_output("clean Git working tree")
        self.assertFalse(self.output.parent.exists())

    def test_untracked_source_outside_archive_scope_also_rejected(self):
        self.write("other/new-module.rs", "unreviewed source\n")
        self.assert_refused_without_output("clean Git working tree")

    def test_tracked_modification_deletion_and_staged_change_rejected(self):
        cases = ["modified", "deleted", "staged", "added"]
        for case in cases:
            with self.subTest(case=case):
                if case == "added":
                    self.write("ui/src/new-module.ts", "export {};\n")
                    self.git("add", "ui/src/new-module.ts")
                elif case == "deleted":
                    (self.root / "README.md").unlink()
                else:
                    self.write("README.md", f"{case}\n")
                    if case == "staged":
                        self.git("add", "README.md")
                self.assert_refused_without_output("clean Git working tree")
                # Establish the next clean baseline only in this temporary
                # fixture, without a destructive Git reset.
                self.commit()

    def test_ignored_files_and_generated_trees_excluded(self):
        self.write("ignored.md", "Ignored private notes\n")
        self.write("artifacts/private.md", "Private import\n")
        self.write("ui/node_modules/dependency.js", "generated\n")
        self.git("add", "ui/node_modules/dependency.js")
        self.git("commit", "-qm", "Tracked generated fixture")
        # Even accidentally tracked generated directories stay excluded.
        names = [name for _, name in PACKAGE.source_files()]
        self.assertNotIn("ignored.md", names)
        self.assertNotIn("artifacts/private.md", names)
        self.assertNotIn("ui/node_modules/dependency.js", names)

    def test_tracked_symlink_rejected(self):
        (self.root / "docs").mkdir()
        (self.root / "docs/link.md").symlink_to("../README.md")
        self.commit()
        self.assert_refused_without_output("Symbolic link not allowed")

    def test_tracked_directory_symlink_rejected(self):
        (self.root / "crates").symlink_to("scripts", target_is_directory=True)
        self.commit()
        self.assert_refused_without_output("Symbolic link not allowed")

    def test_parent_replaced_by_symlink_after_clean_check_rejected(self):
        source = self.root / "ui/src"
        outside = self.directory / "outside-source"
        source.rename(outside)
        source.symlink_to(outside, target_is_directory=True)
        # Model a replacement after the Git cleanliness check has completed.
        with patch.object(PACKAGE, "require_clean_repository"):
            self.assert_refused_without_output("Symbolic directory not allowed: ui/src")

    def test_tracked_hidden_or_unexpected_source_rejected(self):
        for name in ["docs/.secret.md", "fixtures/private.step"]:
            with self.subTest(name=name):
                self.write(name, "Must be reviewed\n")
                self.commit()
                self.assert_refused_without_output("Review")
                self.git("rm", name)
                self.git("commit", "-qm", "Remove rejected fixture")

    def test_existing_output_never_overwritten(self):
        self.output.write_bytes(b"preserve existing archive")
        with self.assertRaises(FileExistsError):
            self.archive()
        self.assertEqual(self.output.read_bytes(), b"preserve existing archive")

    def test_missing_repository_and_nested_root_rejected(self):
        without_git = self.directory / "plain"
        without_git.mkdir()
        with patch.object(PACKAGE, "ROOT", without_git):
            self.assert_refused_without_output("Git repository is required")
        nested = self.root / "scripts"
        with patch.object(PACKAGE, "ROOT", nested):
            self.assert_refused_without_output("must be the Git repository root")

    def test_output_in_source_tree_rejected(self):
        self.output = self.root / "docs/snapshot.tar.gz"
        self.assert_refused_without_output("outside the source trees")


if __name__ == "__main__":
    unittest.main()
