"""Public source/STEP wire correspondence cases. NumPy is a laboratory dependency."""
import importlib.util
from pathlib import Path
import runpy
import unittest


@unittest.skipUnless(importlib.util.find_spec("numpy"), "requires the NumPy qualification environment")
class FlangeInterfaceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import numpy as np
        script_dir = Path(__file__).resolve().parents[1]
        cls.match = staticmethod(runpy.run_path(str(script_dir / "qualify-flange-interface.py"))["match"])
        points, edges = [], []
        for quadrant in range(4):
            a = quadrant * np.pi / 2
            normal = np.array([np.cos(a), np.sin(a), 0.])
            tangent = np.array([-np.sin(a), np.cos(a), 0.])
            center = 2 * (normal + tangent)
            # Four segments along one side, followed by a unit quarter circle.
            for k in range(4):
                start = 3*normal + (k-2)*tangent
                edges.append(dict(kind="line", start_mm=start.tolist(), end_mm=(start+tangent).tolist()))
                points.extend([start.tolist(), (start+.45*tangent).tolist()])
            arc = dict(kind="circle", center_mm=center.tolist(), radius_mm=1., axis=[0.,0.,1.],
                       start_mm=(center+normal).tolist(), end_mm=(center+tangent).tolist(), sweep_rad=np.pi/2)
            edges.append(arc)
            for t in np.arange(8) * np.pi/16:
                points.append((center+np.cos(t)*normal+np.sin(t)*tangent).tolist())
        cls.source = dict(points_mm=points, neighbors=[["Plane",i//2] for i in range(len(points))])
        cls.edges = edges

    def test_reordered_native_edges_still_cover_the_whole_source(self):
        report = self.match(self.source, dict(edges=self.edges[::-1]))
        self.assertEqual((report["native_edges"], report["source_edges"]), (20, 64))
        self.assertLess(report["max_distance_bound_mm"], .005)

    def test_same_count_with_a_duplicate_segment_is_refused(self):
        with self.assertRaisesRegex(ValueError, "shared twice or unassigned"):
            self.match(self.source, dict(edges=[self.edges[1], *self.edges[1:]]))
