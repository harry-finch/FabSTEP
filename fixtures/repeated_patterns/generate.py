"""Square flange with eight bolt positions, one empty and one oversized, in mm."""
import math
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from generate_m2b import Mesh, boundary


def flange(subdivisions):
    mesh = Mesh()
    for row in (-1, 0, 1):
        for col in (-1, 0, 1):
            cx, cy = 16 * col, 16 * row
            edge = boundary(cx - 8, cx + 8, cy - 8, cy + 8, subdivisions)
            base, top = mesh.ring(edge, 0), mesh.ring(edge, 5)
            if (row, col) == (1, 0):
                mesh.fan(top, (cx, cy, 5))
                mesh.fan(base, (cx, cy, 0), True)
            else:
                angle = math.atan2(row, col)
                x, y = (16 * math.cos(angle), 16 * math.sin(angle)) if row or col else (0, 0)
                radius = 5 if (row, col) == (0, 0) else 2.8 if (row, col) == (-1, 0) else 2
                arc = []
                for px, py in edge:
                    length = math.hypot(px - x, py - y)
                    arc.append((x + radius * (px - x) / length, y + radius * (py - y) / length))
                low, high = mesh.ring(arc, 0), mesh.ring(arc, 5)
                mesh.annulus(top, high)
                mesh.annulus(base, low, True)
                mesh.wall(low, high, True)
            for i, (ax, ay) in enumerate(edge):
                j = (i + 1) % len(edge)
                bx, by = edge[j]
                if ax == bx and abs(ax) == 24 or ay == by and abs(ay) == 24:
                    mesh.triangle(base[i], base[j], top[j])
                    mesh.triangle(base[i], top[j], top[i])
    data = bytearray(80) + struct.pack('<I', len(mesh.faces))
    for face in mesh.faces:
        points = [coordinate for index in face for coordinate in mesh.vertices[index]]
        data.extend(struct.pack('<12fH', 0, 0, 0, *points, 0))
    return data


if __name__ == '__main__':
    Path(__file__).with_name('flange.stl').write_bytes(flange(12))
