"""Construct an independent pair from two cone cuts; compare the exported solids."""

import argparse
import json
import math
from pathlib import Path

import FreeCAD as App
import Part
import MeshPart

V = App.Vector


def corner(radius, slope):
    h = 10.0
    rings = []
    for z in [0, h]:
        r = radius + slope * z
        points = [V(0, 0, z), V(r, 0, z), V(0, r, z), V(0, 0, z)]
        rings.append(Part.makePolygon(points))
    wedge = Part.makeLoft(rings, True, True)
    q = math.sqrt(1 + 2 * slope * slope)
    direction = V(slope, slope, 1) * (math.copysign(1.0, slope) / q)
    angle = math.asin(abs(slope) / q)
    reach = abs(radius / slope) + 100
    cone = Part.makeCone(
        0, reach * math.tan(angle), reach, V(0, 0, -radius / slope), direction
    )
    cone.rotate(V(0, 0, -radius / slope), direction, 135)
    return wedge.cut(cone)


def check(name, case, folder, mesh_dir):
    shape = Part.makeBox(20, 16, 10)
    for i, (radius, slope) in enumerate(case["parameters"]["laws"]):
        cut = corner(radius, slope)
        if i:
            cut.rotate(V(10, 8, 0), V(0, 0, 1), 180)
        shape = shape.cut(cut)
    shape = shape.removeSplitter()
    if case["placement"] == "permuted":
        shape.rotate(V(0, 0, 0), V(1, 1, 1), 120)
        shape.translate(V(3, -8, 5))
    shape.exportStep(str(folder / f"{name}-reference.step"))
    if mesh_dir:
        mesh_dir.mkdir(parents=True, exist_ok=True)
        for density, angle in [("coarse", 0.18), ("fine", 0.07)]:
            # Reimport for a native topology and meshing independent of FabSTEP.
            ref = Part.read(str(folder / f"{name}-reference.step"))
            mesh = MeshPart.meshFromShape(
                Shape=ref,
                LinearDeflection=0.02,
                AngularDeflection=angle,
                Relative=False,
            )
            mesh.write(str(mesh_dir / f"{name}-{density}.stl"))
    # Comparison across different constructors and independently trimmed faces.
    model = Part.read(str(folder / f"{name}.step"))
    assert len(model.Solids) == len(shape.Solids) == 1
    assert model.isValid() and shape.isValid() and model.Solids[0].isClosed()
    for body in (model, shape):
        assert len(body.Faces) == 8
        assert sum(isinstance(f.Surface, Part.Cone) for f in body.Faces) == 2
        assert sum(isinstance(e.Curve, Part.Ellipse) for e in body.Edges) == 4
        assert math.isclose(body.Volume, case["parameters"]["volume_mm3"], rel_tol=2e-7)
    # STEP positions and materials at interior, cleared corner and air points.
    probes = 0
    for z in (1.0, 5.0, 9.0):
        for x, y in [
            (10.0, 8.0),
            (0.05, 0.05),
            (19.95, 15.95),
            (-1.0, 8.0),
            (1.6, 1.6),
            (18.5, 14.5),
        ]:
            p = V(x, y, z) if case["placement"] == "plain" else V(z + 3, x - 8, y + 5)
            assert model.isInside(p, 1e-7, False) == shape.isInside(p, 1e-7, False)
            probes += 1
    return {
        "independent_volume_mm3": shape.Volume,
        "exported_volume_mm3": model.Volume,
        "edges": len(model.Edges),
        "material_probes": probes,
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--mesh-dir", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    manifest = json.loads(
        (root / "fixtures/two_linear_blends/manifest.json").read_text()
    )
    print(
        json.dumps(
            {
                name: check(name, case, args.directory, args.mesh_dir)
                for name, case in manifest.items()
                if case["positive"]
            }
        )
    )
