"""Draw two independent rolling-sphere corner profiles and verify their closed STL."""

import json
import math
from pathlib import Path
import runpy

lib = runpy.run_path(str(Path(__file__).with_name("generate-local-blend-fixtures.py")))
L, W, H = lib["LENGTHS"]
root = Path(__file__).resolve().parents[1] / "fixtures/two_linear_blends"


def draw(laws, segments, layers, bad):
    m = lib["Mesh"]()
    walls = []
    for i, (a, b) in enumerate(laws):
        rings = []
        for level in range(layers + 1):
            z = H * level / layers
            curve = [
                lib["envelope_point"](
                    j / segments, z, a, b, "nonlinear" if bad and i else None
                )
                for j in range(segments + 1)
            ]
            if i:
                curve = [(L - x, W - y, z) for x, y, z in curve]
            rings.append(curve)
        outward = (1.0, 1.0, 0.0) if i else (-1.0, -1.0, 0.0)
        for a, b in zip(rings, rings[1:]):
            m.strip(a, b, outward, f"cone_{i}")
        walls.append(rings)
    for end, normal in ((0, -1.0), (-1, 1.0)):
        a, b = [wall[end] for wall in walls]
        z = a[0][2]
        m.fan([*a, (0.0, W, z), *b, (L, 0.0, z)], (0.0, 0.0, normal), f"cap_{end}")
    for i, wall in enumerate(walls):
        for j, (k, far) in enumerate(((0, (L, 0)), (-1, (0, W)))):
            rail = [ring[k] for ring in wall]
            if i:
                far = (L - far[0], W - far[1])
            opposite = [(*far, p[2]) for p in rail]
            m.fan(
                rail + opposite[::-1],
                lib["direction"](1 - j, 1.0 if i else -1.0),
                f"side_{i}_{j}",
            )
    return m


def main():
    root.mkdir(exist_ok=True)
    result = {}
    for name, laws, n, layers, placement, bad in [
        ("opposed", ((3.0, 0.2), (4.0, -0.15)), 28, 5, "plain", False),
        ("offset", ((2.0, 0.24), (2.5, 0.12)), 64, 3, "permuted", False),
        ("nonlinear_second", ((3.0, 0.2), (4.0, -0.15)), 28, 5, "plain", True),
    ]:
        vol = sum(lib["volume_g"](*law) for law in laws) - L * W * H
        result[name] = lib["write"](
            root,
            name,
            draw(laws, n, layers, bad),
            placement,
            not bad,
            {"laws": laws, "volume_mm3": vol},
            defect="nonlinear" if bad else None,
        )
    (root / "manifest.json").write_text(json.dumps(result, indent=2) + "\n")


if __name__ == "__main__":
    main()
