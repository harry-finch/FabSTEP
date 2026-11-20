"""Build inputs and native qualification for the Windows x64 portable archive."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SDK_NAME = "opencascade-7.9.3-vc14-64"
SDK_URL = f"https://github.com/Open-Cascade-SAS/OCCT/releases/download/V7_9_3/{SDK_NAME}-combined.zip"
SDK_SHA256 = "afbef3457fbc4a2bdca0608e0fe284392f51e4c2c42ccbfb9df7168d8e4eb9b3"


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prepare_sdk():
    destination = Path(os.environ["RUNNER_TEMP"]) / "fabstep-sdk"
    destination.mkdir()
    archive = destination / "occt.zip"
    urllib.request.urlretrieve(SDK_URL, archive)
    if sha256(archive) != SDK_SHA256:
        raise ValueError("Unexpected OCCT archive digest")
    with zipfile.ZipFile(archive) as z:
        z.extractall(destination)
    with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as env:
        print(f"OCCT_ROOT={destination / SDK_NAME}", file=env)


def bundle_dlls(folder):
    """Copy only the dependency closure loaded from this release's SDK."""
    sdk = Path(os.environ["OCCT_ROOT"])
    third = sdk.parent / "3rdparty-vc14-64"
    sources = {
        "opencascade": sdk / "win64/vc14/bin",
        "tbb": third / "tbb-2021.13.0-x64/bin",
        "freetype": third / "freetype-2.13.3-x64/bin",
    }
    notices = {
        "opencascade": [sdk / "LICENSE_LGPL_21.txt", sdk / "OCCT_LGPL_EXCEPTION.txt"],
        "tbb": [sources["tbb"].parent / "LICENSE.txt"],
        "freetype": [sources["freetype"].parent / "LICENSE.TXT", sources["freetype"].parent / "docs/FTL.TXT"],
    }
    candidates = {p.name.lower(): (group, p) for group, directory in sources.items()
                  for p in directory.glob("*.dll")}
    tool = shutil.which("llvm-objdump") or Path(os.environ["ProgramFiles"]) / "LLVM/bin/llvm-objdump.exe"
    platform = Path(os.environ["SystemRoot"]) / "System32"
    pending = list(folder.glob("*.exe"))
    records = {}
    while pending:
        binary = pending.pop()
        imports = subprocess.check_output([tool, "-p", binary], text=True, errors="replace")
        names = re.findall(r"(?im)^\s*DLL Name:\s*(\S+)", imports)
        if not names:
            raise ValueError(f"No PE imports found for {binary}")
        for name in names:
            key = name.lower()
            if key in records:
                continue
            if key not in candidates:
                if key.startswith(("api-ms-", "ext-ms-")) or (platform / name).is_file():
                    continue
                raise ValueError(f"Unresolved DLL {name} for {binary.name}")
            group, src = candidates[key]
            output = folder / src.name
            shutil.copyfile(src, output)
            pending.append(output)
            records[key] = {"component": group, "sha256": sha256(output)}
    for group in {entry["component"] for entry in records.values()}:
        target = folder / "Licenses" / group
        target.mkdir(parents=True)
        for notice in notices[group]:
            shutil.copy2(notice, target / notice.name)
    (folder / "Licenses/windows-sdk.json").write_text(
        json.dumps({"archive": SDK_URL, "sha256": SDK_SHA256, "dlls": records}, indent=2), encoding="utf-8")


def smoke(folder, work):
    for fixture, expected_solids, expected_faces in [
        ("cube.stl", 1, 6), ("r117-cut-sphere.stl", 1, 4), ("m5c_sphere_instances.3mf", 2, 2),
    ]:
        output = work / (fixture + ".step")
        report = json.loads(subprocess.check_output([
            folder / "fabstep-cli.exe", "convert", ROOT / "fixtures" / fixture,
            "--mode", "surfaces", "--json", "-o", output], cwd=work, timeout=90))
        cad = report["cad"]
        if not (report["native_validated"] and cad["solids"] == expected_solids and
                cad["faces"] == expected_faces and cad["volume_mm3"] > 0 and
                b"END-ISO-10303-21;" in output.read_bytes()):
            raise ValueError(f"Unexpected STEP reimport on {fixture}")
    app = subprocess.Popen([folder / "FabSTEP.exe"], cwd=work)
    try:
        subprocess.run(["pwsh", "-NoProfile", "-Command", f"""
            $p = Get-Process -Id {app.pid} -ErrorAction Stop
            for ($i=0; $i -lt 60; $i++) {{
                $p.Refresh()
                if ($p.HasExited) {{ throw "Desktop exited before displaying its main window" }}
                if ($p.MainWindowHandle -ne 0 -and $p.MainWindowTitle -like '*FabSTEP*') {{ exit 0 }}
                Start-Sleep -Seconds 1
            }}
            throw "Main window did not open"
        """], check=True, timeout=75)
    finally:
        if app.poll() is None:
            app.terminate()
        app.wait(timeout=15)


def package():
    release = ROOT / "artifacts/windows"
    release.mkdir(parents=True, exist_ok=True)
    binaries = Path(os.environ["CARGO_TARGET_DIR"]) / "release"
    with tempfile.TemporaryDirectory() as tmp:
        work = Path(tmp)
        folder = work / "FabSTEP-1.0.0-Windows-x86_64"
        folder.mkdir()
        for source, name in [("fabstep-desktop.exe", "FabSTEP.exe"), ("fabstep.exe", "fabstep-cli.exe")]:
            shutil.copy2(binaries / source, folder / name)
        subprocess.run([sys.executable, ROOT / "scripts/release-notices.py", folder / "Licenses"], check=True)
        bundle_dlls(folder)
        smoke(folder, work)
        shutil.make_archive(str(release / folder.name), "zip", work, folder.name)


if __name__ == "__main__":
    {"sdk": prepare_sdk, "package": package}[sys.argv[1]]()
