"""Independent standard-library mesh and analytic oracle for cone/open-arc combinations.

No FabSTEP recognizer or native tessellator participates in fixture generation.
Two adjacent planar cells share their exact sampled boundary. The right cell
contains the complete countersink; the left cell contains a semicircular notch.
"""
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parent / "post_v1_combinations"


def cross(a, b, c):
    return (b[0]-a[0])*(c[1]-a[1]) - (b[1]-a[1])*(c[0]-a[0])


def triangulate(points):
    remaining = list(range(len(points)))
    triangles = []
    while len(remaining) > 3:
        for i, b in enumerate(remaining):
            a, c = remaining[i-1], remaining[(i+1) % len(remaining)]
            if cross(points[a], points[b], points[c]) <= 1e-10:
                continue
            if any(all(cross(points[u], points[v], points[k]) >= -1e-10
                       for u, v in ((a,b), (b,c), (c,a)))
                   for k in remaining if k not in (a,b,c)):
                continue
            triangles.append((a,b,c))
            remaining.pop(i)
            break
        else:
            raise ValueError("Cannot triangulate without altering source boundary")
    return triangles + [tuple(remaining)]


def transform(point, variant):
    x, y, z = point
    if variant == "mirrored":
        return (-x+55, y-17, z+3)
    if variant == "rotated":
        axis = (1/math.sqrt(14), 2/math.sqrt(14), 3/math.sqrt(14))
        cosine, sine = math.cos(.63), math.sin(.63)
        dot = sum(a*b for a,b in zip(axis, point))
        vector = (axis[1]*z-axis[2]*y, axis[2]*x-axis[0]*z, axis[0]*y-axis[1]*x)
        return tuple(cosine*p+sine*v+(1-cosine)*dot*a+t
                     for p,v,a,t in zip(point, vector, axis, (13,-7,11)))
    return point


class Mesh:
    def __init__(self):
        self.points, self.triangles = [], []

    def ring(self, points, z):
        indices = list(range(len(self.points), len(self.points)+len(points)))
        self.points.extend((x,y,z) for x,y in points)
        return indices

    def tri(self, a, b, c, reverse=False):
        self.triangles.append((c,b,a) if reverse else (a,b,c))

    def wall(self, low, high, reverse=False, omit=None):
        for i in range(len(low)):
            j = (i+1) % len(low)
            if omit and omit(self.points[low[i]], self.points[low[j]]):
                continue
            self.tri(low[i], low[j], high[j], reverse)
            self.tri(low[i], high[j], high[i], reverse)

    def annulus(self, outside, inside, reverse=False):
        for i in range(len(outside)):
            j = (i+1) % len(outside)
            self.tri(inside[i], outside[i], outside[j], reverse)
            self.tri(inside[i], outside[j], inside[j], reverse)

    def write(self, name, variant):
        points = [transform(p, variant) for p in self.points]
        with (ROOT / f"{name}.stl").open("w") as output:
            output.write(f"solid {name}\n")
            for triangle in self.triangles:
                output.write(" facet normal 0 0 0\n  outer loop\n")
                for index in triangle[::-1] if variant == "mirrored" else triangle:
                    output.write("   vertex " + " ".join(f"{v:.17g}" for v in points[index]) + "\n")
                output.write("  endloop\n endfacet\n")
            output.write(f"endsolid {name}\n")


def make(name, variant="plain", defect=None, arc_segments=24):
    mesh = Mesh()
    zs = (0, 5, 10)
    profiles = []
    for z in zs:
        radius = 4.3 if defect == "arc_bulge" and z == 5 else 4
        arc = [(radius*math.cos(math.pi/2-i*math.pi/arc_segments),
                12+radius*math.sin(math.pi/2-i*math.pi/arc_segments))
               for i in range(arc_segments+1)]
        arc[0], arc[-1] = (0,12+radius), (0,12-radius)
        profiles.append([(0,0),(20,0)] + [(20,24*i/16) for i in range(1,17)]
                        + [(0,24)] + ([] if defect == "no_arc" else arc))
    rings = [mesh.ring(p,z) for p,z in zip(profiles,zs)]
    for low,high in zip(rings,rings[1:]):
        mesh.wall(low,high,omit=lambda a,b: a[0] == b[0] == 20)
    for profile,ring,reverse in ((profiles[0],rings[0],True),(profiles[-1],rings[-1],False)):
        for a,b,c in triangulate(profile):
            mesh.tri(ring[a],ring[b],ring[c],reverse)

    square = ([(44,24*i/16) for i in range(16)]
              + [(44-24*i/16,24) for i in range(16)]
              + [(20,24-24*i/16) for i in range(16)]
              + [(20+24*i/16,0) for i in range(16)])
    outside = [mesh.ring(square,z) for z in zs]
    for low,high in zip(outside,outside[1:]):
        mesh.wall(low,high,omit=lambda a,b: a[0] == b[0] == 20)
    angles = [math.atan2(y-12,x-32) for x,y in square]
    inside = []
    for z,radius in ((0,3),(6,3),(8,4.5),(10,6)):
        points = []
        for angle in angles:
            r = radius + (.25*math.sin(3*angle) if defect == "cone_warp" and z == 8 else 0)
            points.append(((6 if defect == "intersection" else 32)+r*math.cos(angle),12+r*math.sin(angle)))
        inside.append(mesh.ring(points,z))
    mesh.annulus(outside[0],inside[0],True)
    mesh.annulus(outside[-1],inside[-1])
    for low,high in zip(inside,inside[1:]):
        mesh.wall(low,high,True)
    mesh.write(name,variant)
    return len(mesh.triangles)


def main():
    ROOT.mkdir(exist_ok=True)
    # Box minus an open half-cylinder, through bore and countersink frustum.
    volume = 44*24*10 - math.pi*4**2*10/2 - math.pi*3**2*6 - math.pi*4*(3**2+3*6+6**2)/3
    cases = {}
    for name,variant,defect,segments in (
        ("countersink_notch","plain",None,24),
        ("countersink_notch_rotated","rotated",None,24),
        ("countersink_notch_mirrored","mirrored",None,24),
        ("cone_warp","plain","cone_warp",24),
        ("arc_bulge","plain","arc_bulge",24),
        ("arc_sparse","plain",None,4),
        ("arc_dense_budget","plain",None,400),
        ("intersecting_features","plain","intersection",24),
        ("countersink_only","plain","no_arc",24),
    ):
        item = {"triangles":make(name,variant,defect,segments),"transform":variant}
        if segments == 400:
            item["expected_refusal"] = "recognition.too_complex"
        elif segments == 4:
            item["expected_non_promotion"] = "arc_underresolved: historical faceted notch remains"
        elif defect not in (None,"no_arc"):
            item["expected_refusal"] = defect
        else:
            corners = [transform((x,y,z),variant) for x in (0,44) for y in (0,24) for z in (0,10)]
            item["bounds_mm"] = [list(map(fn,zip(*corners))) for fn in (min,max)]
            item.update(volume_mm3=volume+(math.pi*80 if defect == "no_arc" else 0),
                        planes=6 if defect == "no_arc" else 7,
                        cylinders=1 if defect == "no_arc" else 2, cones=1,
                        partial_cylinders=0 if defect == "no_arc" else 1)
        cases[name] = item
    (ROOT / "manifest.json").write_text(json.dumps({
        "unit":"mm", "dimensions_mm":[44,24,10], "bore_center_mm":[32,12],
        "bore_radius_mm":3, "bore_height_mm":6, "countersink_end_radii_mm":[3,6],
        "countersink_height_mm":4,"notch_radius_mm":4,"notch_sweep_rad":math.pi,
        "cases":cases},indent=2)+"\n")


if __name__ == "__main__":
    main()
