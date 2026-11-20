"""Oracle for the four clipped-band solids. Run with the FreeCAD Python.

Pass the destination directory. Solids are exported and reimported before
meshing. The independent face labels are written with each pair of inputs.
"""
import argparse
import json
import math
import subprocess
from pathlib import Path

import FreeCAD as App
import MeshPart
import Part

V = App.Vector
CASES = [
    ("cone1", 12, 8, 8, -3, 0.23),
    ("cone2", 15, 10, 11, -4, -0.13),
    ("sphere1", 12, None, 7, -3, 0.24),
    ("sphere3", 15, None, 9, -4, 0.24),
]


def shape(r, top, height, offset, slope):
    if top is None:
        support = Part.makeSphere(r, V(0, 0, height * 0.38))
        slab = Part.makeBox(4 * r, 4 * r, height, V(-2 * r, -2 * r, 0))
        blank = support.common(slab)
    else:
        blank = Part.makeCone(r, top, height)
    reach = 8 * max(r, height)
    block = Part.makeBox(reach, 2 * reach, 2 * reach, V(0, -reach, -reach))
    block.rotate(V(), V(0, 1, 0), math.degrees(math.atan(slope)))
    block.translate(V(offset, 0, 0))
    return blank.cut(block).removeSplitter()


def verify_export(cli, source, reference, owners):
    result = source.with_name(source.stem + "-out.step")
    report = subprocess.run(
        [str(cli), "convert", str(source), "--mode", "auto", "--json", "--force", "-o", str(result)],
        capture_output=True, text=True, check=True,
    )
    body = json.loads(report.stdout)["bodies"][0]
    fit = body["oblique_band_recognition"]
    assert fit["triangle_faces"] == owners and fit["report"]["max_deviation_bound_mm"] <= .05
    solid = Part.read(str(result))
    assert solid.isValid() and solid.isClosed() and len(solid.Solids) == 1
    solid.check(True)
    assert len(solid.Vertexes) == 4 and abs(reference.Volume - solid.Volume) < .001
    assert sorted(f.Surface.TypeId for f in solid.Faces) == sorted(f.Surface.TypeId for f in reference.Faces)
    assert sorted(e.Curve.TypeId for e in solid.Edges) == sorted(e.Curve.TypeId for e in reference.Edges)


def run(dest, cli):
    dest.mkdir(parents=True, exist_ok=True)
    Part.setStaticValue("write.surfacecurve.mode", 1)
    records = {}
    for name, r, top, h, c, k in CASES:
        path = dest / (name + ".step")
        shape(r, top, h, c, k).exportStep(str(path))
        solid = Part.read(str(path))
        assert solid.isValid() and solid.isClosed() and len(solid.Solids) == 1
        solid.check(True)
        densities = [("coarse", .035, .2), ("fine", .019, .14)]
        if name == "cone2":
            densities = [("sparse", .035, .2), ("coarse", .026, .16), densities[1]]
        for density, linear, angular in densities:
            mesh = MeshPart.meshFromShape(
                Shape=solid, LinearDeflection=linear, AngularDeflection=angular,
                Relative=False, Segments=True
            )
            key = name + "-" + density
            mesh.write(str(dest / (key + ".stl")))
            owner = [-1] * mesh.CountFacets
            maximum_z = max(face.CenterOfMass.z for face in solid.Faces)
            for face_id, face in enumerate(solid.Faces):
                normal = face.normalAt(0, 0)
                if face.Surface.TypeId != "Part::GeomPlane":
                    label = 3
                elif abs(normal.z) < .95:
                    label = 2
                else:
                    label = int(abs(face.CenterOfMass.z - maximum_z) < 1e-6)
                for idx in mesh.getSegment(face_id):
                    assert owner[idx] == -1
                    owner[idx] = label
            assert set(owner) == {0, 1, 2, 3}
            (dest / (key + ".owners")).write_bytes(bytes(owner))
            if cli and density != "sparse":
                verify_export(cli, dest / (key + ".stl"), solid, owner)
            records[key] = {"volume_mm3": solid.Volume, "triangles": len(owner)}
    (dest / "reference.json").write_text(json.dumps(records, indent=2) + "\n")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--cli", type=Path, help="Export the measured STL and check it against the independent solid")
    args = parser.parse_args()
    run(args.directory, args.cli.resolve() if args.cli else None)
