"""R-122. See fixtures/recognition_corpus/README.md for invocation and metric definitions."""

import argparse
import hashlib
import json
import os
import platform
import struct
import subprocess
import sys
import tempfile
import time
from collections import Counter
from pathlib import Path

import FreeCAD
import MeshPart
import Part

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT / "fixtures/recognition_corpus"
DENSITIES = {"coarse": (0.08, 0.44), "fine": (0.035, 0.30)}
FAMILIES = dict(
    GeomPlane="plane",
    GeomCylinder="cylinder",
    GeomCone="cone",
    GeomSphere="sphere",
    GeomToroid="torus",
    GeomSurfaceOfRevolution="revolution",
)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def reference(part, cache):
    path = cache / (part["id"] + ".step")
    if not path.exists():
        download = subprocess.run(
            [
                "curl",
                "--fail",
                "--silent",
                "--show-error",
                "--location",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--max-time",
                "45",
                "--max-filesize",
                "2000000",
                part["url"],
            ],
            stdout=subprocess.PIPE,
        )
        if download.returncode:
            raise OSError(f"reference download failed (curl {download.returncode})")
        data = download.stdout
        if len(data) > 2_000_000 or digest(data) != part["sha256"]:
            raise ValueError("reference download differs from manifest")
        path.write_bytes(data)
    if digest(path.read_bytes()) != part["sha256"]:
        raise ValueError("cached reference differs from manifest")
    return path


def source(step, density, dest):
    # Read anew at each density to avoid reusing a finer cached OCCT triangulation.
    shape = Part.read(str(step))
    if len(shape.Solids) != 1 or not shape.Solids[0].isClosed() or not shape.isValid():
        raise ValueError("reference must contain one valid closed solid")
    lin, ang = DENSITIES[density]
    mesh = MeshPart.meshFromShape(
        Shape=shape,
        LinearDeflection=lin,
        AngularDeflection=ang,
        Relative=False,
        Segments=True,
    )
    if mesh.countSegments() != len(shape.Faces) or not 0 < mesh.CountFacets <= 100_000:
        raise ValueError("segmentation missing or ordinary budget exceeded")
    families = [None] * mesh.CountFacets
    for i, face in enumerate(shape.Faces):
        name = FAMILIES.get(face.Surface.TypeId.split("::")[-1], "other")
        for idx in mesh.getSegment(i):
            if families[idx] is not None:
                raise ValueError("reference triangle claimed twice")
            families[idx] = name
    if any(f is None for f in families):
        raise ValueError("incomplete reference coverage")
    # Write the actual segmented facet order with a constant header, in f32 mm.
    data = bytearray(80) + struct.pack("<I", mesh.CountFacets)
    areas = []
    for facet in mesh.Facets:
        points = [FreeCAD.Vector(*v) for v in facet.Points]
        a, b, c = points
        areas.append((b - a).cross(c - a).Length / 2)
        data.extend(struct.pack("<12fH", *facet.Normal, *a, *b, *c, 0))
    dest.write_bytes(data)
    return digest(data), families, areas


def measure(binary, stl, timeout):
    with tempfile.TemporaryDirectory() as temp, stl.open("rb") as source_file:
        out = Path(temp) / "result.json"
        start = time.monotonic()
        with subprocess.Popen(
            [binary, out], stdin=source_file, stdout=subprocess.DEVNULL
        ) as proc:
            while True:
                pid, status, usage = os.wait4(proc.pid, os.WNOHANG)
                if pid:
                    proc.returncode = os.waitstatus_to_exitcode(status)
                    if proc.returncode:
                        raise RuntimeError(f"probe exited with {proc.returncode}")
                    break
                if time.monotonic() - start >= timeout:
                    proc.kill()
                    proc.wait()
                    return {"refusal": {"code": "corpus.timeout"}}, {
                        "seconds": time.monotonic() - start
                    }
                time.sleep(0.02)
        rss = usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024)
        return json.loads(out.read_text()), {
            "seconds": time.monotonic() - start,
            "rss_bytes": rss,
        }


def score(result, truth, areas):
    if "refusal" in result:
        return {"status": "refused", "error": result["refusal"]["code"]}
    report = result["ok"]
    mapping = report["mapping"]
    families = report["families"]
    score = dict(
        status="converted",
        cad=report["cad"],
        fidelity=report["fidelity"],
        settings=report["settings"],
        provenance=mapping["status"],
        collars=sum(f == "other" for f in families) if report["atlas"] else 0,
    )
    if mapping["status"] != "exact":
        return score
    seen = set()
    matrix = {}
    for record in mapping["faces"]:
        fitted = families[record["face_id"]]
        for idx in record["source_triangle_ids"]:
            if idx in seen or not 0 <= idx < len(truth):
                raise ValueError("invalid source correspondence")
            seen.add(idx)
            row = matrix.setdefault(truth[idx], Counter())
            row[fitted] += areas[idx]
    if len(seen) != len(truth):
        raise ValueError("missing source triangles")
    score["source_area_mm2_by_reference_and_prediction"] = matrix
    score["families"] = {}
    for family in sorted(set(matrix) | set(families)):
        expected = sum(matrix.get(family, {}).values())
        matched = matrix.get(family, {}).get(family, 0)
        extra = sum(row.get(family, 0) for key, row in matrix.items() if key != family)
        score["families"][family] = dict(
            coverage=matched / expected if expected else None,
            missed_mm2=expected - matched,
            false_positive_mm2=extra,
        )
    return score


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=Path)
    parser.add_argument("--probe", type=Path)
    parser.add_argument("--part", action="append", default=[])
    parser.add_argument("--repeat", type=int, choices=(1, 2), default=2)
    parser.add_argument("--timeout", type=int, default=120)
    args = parser.parse_args()
    parts = json.loads((CORPUS / "manifest.json").read_text())["parts"]
    unknown = set(args.part) - {p["id"] for p in parts}
    if unknown or args.timeout < 1:
        parser.error("unknown part or nonpositive timeout")
    cache = args.out.resolve() / "reference"
    meshes = args.out.resolve() / "meshes"
    for d in (cache, meshes):
        d.mkdir(parents=True, exist_ok=True)
    results = {}
    lock = json.loads((CORPUS / "meshes.json").read_text())
    env = dict(
        freecad=FreeCAD.Version()[:3],
        occ=Part.OCC_VERSION,
        machine=platform.machine(),
        platform=platform.platform(),
        densities=DENSITIES,
        unit="mm",
        timeout_s=args.timeout,
        probe_sha256=digest(args.probe.read_bytes()) if args.probe else None,
    )
    ok = True
    for part in parts:
        if args.part and part["id"] not in args.part:
            continue
        for density in DENSITIES:
            key = part["id"] + "-" + density
            try:
                path = reference(part, cache)
                stl = meshes / (key + ".stl")
                sha, labels, areas = source(path, density, stl)
                if sha != lock[key]:
                    raise ValueError("STL hash differs from pinned mesher")
                row = dict(input_sha256=sha, triangles=len(labels), split=part["split"])
                if args.probe:
                    runs = []
                    for _ in range(args.repeat):
                        raw, usage = measure(args.probe.resolve(), stl, args.timeout)
                        runs.append(score(raw, labels, areas))
                        row.setdefault("resources", []).append(usage)
                    if any(r != runs[0] for r in runs):
                        raise ValueError(
                            "geometric results differ across repeated conversions"
                        )
                    row.update(runs[0])
                results[key] = row
            except (OSError, KeyError, ValueError, RuntimeError) as err:
                results[key] = {"status": "unavailable", "reason": str(err)}
                ok = False
            print(key, results[key].get("status", "prepared"), flush=True)
            (args.out / "report.json").write_text(
                json.dumps({"environment": env, "parts": results}, indent=2) + "\n"
            )
    if args.probe and not args.part:
        for name, missing in [("m5d_torus_dent", "torus"), ("m5d_open_torus", "solid")]:
            stl = ROOT / "fixtures" / (name + ".stl")
            raw, usage = measure(args.probe.resolve(), stl, args.timeout)
            refusal = raw.get("refusal", {}).get("code")
            expected = (
                refusal == "mesh.open"
                if missing == "solid"
                else ("ok" in raw and missing not in raw["ok"]["families"])
            )
            results[name] = {
                "expected_absent": missing,
                "refusal": refusal,
                "pass": expected,
                "source_sha256": digest(stl.read_bytes()),
                "resources": [usage],
            }
            ok &= expected
        (args.out / "report.json").write_text(
            json.dumps({"environment": env, "parts": results}, indent=2) + "\n"
        )
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
