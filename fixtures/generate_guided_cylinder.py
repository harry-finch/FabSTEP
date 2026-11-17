"""Two placed instances of the independent M2b mesh, without recognition code."""
import hashlib
import json
from pathlib import Path

from generate_m3 import mesh_object, model, write

ROOT = Path(__file__).resolve().parent


def main():
    source = ROOT / 'm2b_boss_blind.stl'
    points, triangles, indices, pending = [], [], {}, []
    for line in source.read_text().splitlines():
        fields = line.split()
        if fields[:1] != ['vertex']:
            continue
        point = tuple(float(value) for value in fields[1:])
        if point not in indices:
            indices[point] = len(points)
            points.append(point)
        pending.append(indices[point])
        if len(pending) == 3:
            triangles.append(tuple(pending))
            pending = []
    assert not pending and triangles
    transforms = ['1 0 0 0 1 0 0 0 1 0 0 0', '-1 0 0 0 1 0 0 0 1 100 0 0']
    name = 'guided_cylinder_instances'
    write(name, model([mesh_object(1, points, triangles, 'Boss and blind hole')],
                      [(1, transform) for transform in transforms]))
    manifest = {
        'source': source.name,
        'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'output_sha256': hashlib.sha256((ROOT / (name + '.3mf')).read_bytes()).hexdigest(),
        'triangles_per_instance': len(triangles),
        'transforms': transforms,
        'expected': {'solids': 2, 'guided_cylinders': 1,
                     'other_instance': 'planar', 'selected_instance': 'mirrored'},
    }
    (ROOT / (name + '.json')).write_text(json.dumps(manifest, indent=2) + '\n')


if __name__ == '__main__':
    main()
