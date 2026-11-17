#!/usr/bin/env python3
"""Read an opt-in native DXF corpus with independent ezdxf (test-only tool).

Usage: python validate-dxf-profiles.py artifacts/dxf1/native-corpus
Requires ezdxf in the selected Python environment; no application dependency.
"""
import argparse
import json
import math
from pathlib import Path

import ezdxf
from ezdxf import bbox


def close(actual, expected):
    assert math.isclose(actual, expected, rel_tol=1e-12, abs_tol=1e-9), (actual, expected)


def point(actual, expected):
    for a, b in zip(actual, [*expected, 0.0]):
        close(a, b)


def named_loops(profile):
    if 'bodies' in profile:
        for body in profile['bodies']:
            holes = {}
            for loop in body['loops']:
                component = loop['component_id']
                if loop['is_outer']:
                    suffix = 'OUTER'
                else:
                    holes[component] = holes.get(component, 0) + 1
                    suffix = f'HOLE_{holes[component]}'
                yield f"BODY_{body['solid_id']}_COMPONENT_{component}_{suffix}", loop
    else:
        hole = 0
        for loop in profile['loops']:
            if not loop['is_outer']:
                hole += 1
            yield 'OUTER' if loop['is_outer'] else f'HOLE_{hole}', loop


def validate(directory):
    manifest = json.loads((directory / 'manifest.json').read_text())
    reports = []
    for record in manifest:
        document = ezdxf.readfile(directory / record['file'])
        audit = document.audit()
        assert not audit.errors and not audit.fixes, record['file']
        assert document.dxfversion == 'AC1015' and document.units == 4
        entities = list(document.modelspace())
        profile = record['profile']
        loops = list(named_loops(profile))
        if 'selected_solid_ids' in record:
            assert [body['solid_id'] for body in profile['bodies']] == record['selected_solid_ids']
        assert len(entities) == sum(len(loop['segments']) for _, loop in loops)
        offset = 0
        for layer, loop in loops:
            endpoints = []
            for segment in loop['segments']:
                entity = entities[offset]
                offset += 1
                assert entity.dxftype() == segment['kind'].upper()
                assert entity.dxf.layer == layer
                if segment['kind'] == 'line':
                    point(entity.dxf.start, segment['start_mm'])
                    point(entity.dxf.end, segment['end_mm'])
                    endpoints.append((entity.dxf.start, entity.dxf.end))
                elif segment['kind'] == 'ellipse':
                    point(entity.dxf.center, segment['center_mm'])
                    point(entity.dxf.major_axis, segment['major_axis_mm'])
                    close(entity.dxf.ratio, segment['ratio'])
                    point(entity.dxf.extrusion, [0., 0., 1.])
                    sweep = segment['sweep_param_rad']
                    full = abs(sweep) == math.tau
                    imported_sweep = entity.dxf.end_param - entity.dxf.start_param
                    if imported_sweep < 0:
                        imported_sweep %= math.tau
                    close(imported_sweep, abs(sweep))
                    major = segment['major_axis_mm']
                    minor = [-major[1] * segment['ratio'], major[0] * segment['ratio']]
                    def ellipse_point(parameter):
                        return [segment['center_mm'][i] + major[i] * math.cos(parameter)
                                + minor[i] * math.sin(parameter) for i in range(2)]
                    # Read points through the independent reader, including the
                    # interior of trimmed ellipses (endpoint checks alone miss the arc).
                    for fraction in [0., 0.25, 0.5, 0.75, 1.]:
                        imported_param = entity.dxf.start_param + imported_sweep * fraction
                        actual = next(entity.vertices([imported_param]))
                        expected_param = imported_param if full else (
                            segment['start_param_rad'] + sweep * (fraction if sweep > 0 else 1. - fraction))
                        point(actual, ellipse_point(expected_param))
                    if full:
                        assert len(loop['segments']) == 1
                    else:
                        points = list(entity.vertices([entity.dxf.start_param, entity.dxf.end_param]))
                        if sweep < 0:
                            points.reverse()
                        endpoints.append(points)
                else:
                    point(entity.dxf.center, segment['center_mm'])
                    close(entity.dxf.radius, segment['radius_mm'])
                    if segment['kind'] == 'arc':
                        sweep = segment['sweep_angle_rad']
                        close(math.radians((entity.dxf.end_angle - entity.dxf.start_angle) % 360), abs(sweep))
                        points = list(entity.vertices([entity.dxf.start_angle, entity.dxf.end_angle]))
                        if sweep < 0:
                            points.reverse()
                        start = segment['start_angle_rad']
                        expected = [segment['center_mm'][i] + segment['radius_mm'] * f(start) for i, f in enumerate((math.cos, math.sin))]
                        point(points[0], expected)
                        endpoints.append(points)
                    else:
                        assert len(loop['segments']) == 1
            if endpoints:
                for (_, end), (start, _) in zip(endpoints, endpoints[1:] + endpoints[:1]):
                    assert end.distance(start) <= 1e-7, 'Imported loop is open'
        bounds = bbox.extents(entities)
        assert bounds.has_data
        dimensions = tuple(bounds.size)
        if record['file'].startswith('cube-') or record['file'] == 'box-xy.dxf':
            close(dimensions[0], 1.0)
            close(dimensions[1], 1.0)
        close(dimensions[2], 0.0)
        reports.append({'file': record['file'], 'entities': len(entities), 'loops': len(loops), 'dimensions_mm': dimensions, 'audit_errors': 0, 'audit_fixes': 0})
    assert reports, 'No profiles checked'
    return {'reader': 'ezdxf', 'version': ezdxf.__version__, 'profiles': reports, 'status': 'PASS'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    result = validate(args.directory)
    (args.directory / 'independent-validation.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f"PASS: {len(result['profiles'])} DXF profiles, ezdxf {ezdxf.__version__}, no audit error or repair")
