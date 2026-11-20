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

ROOT = Path(__file__).resolve().parents[1]
OCCT_REVISION = "a016080bf6738d6aeae020badee4e888ad1540a5"


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def prepare_sdk():
    sdk = Path(os.environ["OCCT_ROOT"])
    src = ROOT / "occt-source"
    build = Path(os.environ["RUNNER_TEMP"]) / "occt-build"
    head = subprocess.check_output(["git", "-C", src, "rev-parse", "HEAD"], text=True).strip()
    if head != OCCT_REVISION:
        raise ValueError("Unexpected OCCT revision")
    # OCCT expands these four toolkits into their complete dependency closure.
    modules = "FoundationClasses ModelingData ModelingAlgorithms Visualization ApplicationFramework DataExchange DETools Draw"
    subprocess.run([
        "cmake", "-S", src, "-B", build, "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release",
        f"-DINSTALL_DIR={sdk.as_posix()}", "-DINSTALL_DIR_INCLUDE=inc",
        "-DINSTALL_DIR_LIB=win64/vc14/lib", "-DINSTALL_DIR_BIN=win64/vc14/bin",
        "-DBUILD_ADDITIONAL_TOOLKITS=TKDESTEP TKOffset TKFillet TKMesh",
        *(f"-DBUILD_MODULE_{module}=OFF" for module in modules.split()),
        "-DUSE_FREETYPE=OFF", "-DUSE_TK=OFF", "-DUSE_OPENGL=OFF", "-DUSE_TBB=OFF",
    ], check=True)
    subprocess.run(["cmake", "--build", build, "--parallel", "4"], check=True)
    subprocess.run(["cmake", "--install", build], check=True)
    for name in ("LICENSE_LGPL_21.txt", "OCCT_LGPL_EXCEPTION.txt"):
        shutil.copy2(src / name, sdk / name)


def bundle_dlls(folder):
    """Copy only the dependency closure loaded from this release's SDK."""
    sdk = Path(os.environ["OCCT_ROOT"])
    candidates = {p.name.lower(): p for p in (sdk / "win64/vc14/bin").glob("*.dll")}
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
            src = candidates[key]
            output = folder / src.name
            shutil.copyfile(src, output)
            pending.append(output)
            records[key] = {"component": "OCCT", "sha256": sha256(output)}
    if "tkdestep.dll" not in records:
        raise ValueError("OCCT STEP toolkit missing from native imports")
    target = folder / "Licenses" / "opencascade"
    target.mkdir(parents=True)
    for name in ("LICENSE_LGPL_21.txt", "OCCT_LGPL_EXCEPTION.txt"):
        shutil.copy2(sdk / name, target / name)
    (target / "inventory.json").write_text(json.dumps({
        "source": f"https://github.com/Open-Cascade-SAS/OCCT/tree/{OCCT_REVISION}",
        "revision": OCCT_REVISION, "version": "7.9.3", "dlls": records,
    }, indent=2), encoding="utf-8")


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
    app_env = dict(os.environ, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--force-renderer-accessibility")
    app = subprocess.Popen([folder / "FabSTEP.exe"], cwd=work, env=app_env)
    try:
        subprocess.run(["powershell", "-NoProfile", "-Command", f"""
            $ErrorActionPreference = 'Stop'
            Add-Type -AssemblyName UIAutomationClient
            Add-Type -AssemblyName UIAutomationTypes
            $button = New-Object System.Windows.Automation.PropertyCondition(
                [System.Windows.Automation.AutomationElement]::ControlTypeProperty,
                [System.Windows.Automation.ControlType]::Button
            )
            $p = Get-Process -Id {app.pid} -ErrorAction Stop
            for ($i=0; $i -lt 60; $i++) {{
                $p.Refresh()
                if ($p.HasExited) {{ throw "Desktop exited before displaying its main window" }}
                if ($p.MainWindowHandle -ne 0 -and $p.MainWindowTitle -like '*FabSTEP*') {{
                    $window = [System.Windows.Automation.AutomationElement]::FromHandle($p.MainWindowHandle)
                    $buttons = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, $button)
                    foreach ($item in $buttons) {{
                        if ($item.Current.Name -match '(Ouvrir|Open) STL / 3MF') {{ exit 0 }}
                    }}
                }}
                Start-Sleep -Seconds 1
            }}
            throw "The main window did not expose the file-open button"
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
