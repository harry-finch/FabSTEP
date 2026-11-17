"""Independent two-frustum source corpus. References precede recognition."""
import hashlib
import json
from pathlib import Path
import generate_m2b
from generate_post_v1_chains import build, reference

ROOT = Path(__file__).resolve().parent / 'adjacent_cones'


def main():
    ROOT.mkdir(exist_ok=True)
    generate_m2b.ROOT = ROOT
    cases = {}
    for name, profile, rotate in [
        ('increasing', [(0., 10.), (3., 13.), (9., 16.)], False),
        ('decreasing', [(0., 20.), (3., 17.), (9., 15.)], False),
        ('opposed', [(0., 10.), (3., 15.), (9., 12.)], False),
        ('rotated', [(0., 10.), (3., 13.), (9., 16.)], True),
    ]:
        cases[name] = reference(profile, rotate)
        build(profile).write(name, rotate)
        cases[name]['sha256'] = hashlib.sha256((ROOT / (name + '.stl')).read_bytes()).hexdigest()
    (ROOT / 'manifest.json').write_text(json.dumps(dict(unit='mm', construction='Two full adjacent frusta; source rings at thirds; R5 bore; independent analytical references', cases=cases), indent=2) + '\n')


if __name__ == '__main__':
    main()
