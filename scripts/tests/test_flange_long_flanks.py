"""Source coverage of a public trimmed cylindrical wall with two axial steps."""
import importlib.util
from pathlib import Path
import runpy
import unittest


@unittest.skipUnless(importlib.util.find_spec("numpy") and importlib.util.find_spec("scipy"),
                     "requires the NumPy/SciPy qualification environment")
class LongFlankTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import numpy as np
        code = runpy.run_path(str(Path(__file__).resolve().parents[1] / "qualify-flange-flanks.py"))
        cls.chart = staticmethod(code["cylindrical_chart"])
        cls.mesh_type = code["boundary"]["DumpMesh"]
        cls.dtype = code["partition"]["residual"]["RECORD"]
        # A cylindrical strip with a raised ledge, sloping along its upper edge.
        uv = np.array([[.1,0], [.1,1], [.2,0], [.2,1], [.2,1.5], [.3,0], [.3,1], [.3,1.6], [.4,0], [.4,1]])
        cls.vertices = np.column_stack([uv[:, 1], 2*np.cos(uv[:, 0]), 2*np.sin(uv[:, 0])])
        cls.tri = np.array([[0,2,3], [0,3,1], [2,5,6], [2,6,3], [3,6,7], [3,7,4], [5,8,9], [5,9,6]])
        cls.upper = uv[[1,3,4,7,6,9]]

    def source_mesh(self, reverse=False, omit_neighbor=False):
        import numpy as np
        tri = self.tri[:, ::-1] if reverse else self.tri
        xyz = list(self.vertices[tri])
        # Give each free edge its own opposite source triangle. Interior edges
        # already have both incidences.
        edges = {(int(a), int(b)) for face in tri for a, b in zip(face, np.roll(face, -1))}
        for a, b in sorted(edges):
            if (b, a) not in edges and not (omit_neighbor and (a, b) == (7, 4)):
                xyz.append(np.array([self.vertices[b], self.vertices[a], [-1.,0.,0.]]))
        rows = np.zeros(len(xyz), dtype=self.dtype)
        rows["points"] = xyz
        rows["triangle"] = np.arange(len(xyz))
        rows["owner"] = rows["triangle"]
        return self.mesh_type(rows)

    def test_ordered_trim_retains_vertical_steps_and_sloping_edge(self):
        import numpy as np
        for reverse in (False, True):
            with self.subTest(reverse=reverse):
                params = {}
                mesh = self.source_mesh(reverse)
                report, _, _ = self.chart(mesh, np.arange(len(self.tri)), np.zeros(2), 2., [.1,.4], 0.,
                                          long_domain=params)
                np.testing.assert_allclose(params["top_uv"], self.upper, atol=1e-14)
                self.assertEqual(report["upper_vertical_edges"], 2)
                self.assertAlmostEqual(params["area_mm2"], .71)
                self.assertEqual(params["outward"], not reverse)
                self.assertEqual(params["source_forward"], reverse)
                for a, e, b, f, owner in params["top_source_uses"]:
                    p, q = mesh.triangles[a], mesh.triangles[b]
                    self.assertEqual((p[e],p[(e+1)%3]), (q[(f+1)%3],q[f]))
                    self.assertEqual(owner,b)
                self.assertLess(max(params["edge_bound_mm"]), .003)

    def test_plateau_is_reconciled_before_area_and_distance_proofs(self):
        import numpy as np
        params = {}
        station = 1. - 2e-6
        report, _, _ = self.chart(self.source_mesh(), np.arange(len(self.tri)), np.zeros(2), 2.,
                                   [.1,.4], 0., long_domain=params, plateau=station)
        upper = self.upper.copy()
        upper[upper[:, 1] == 1., 1] = station
        np.testing.assert_allclose(params["top_uv"], upper, atol=1e-14)
        self.assertAlmostEqual(report["vertex_motion_mm"], 2e-6)
        self.assertLess(params["area_mm2"], .71)
        with self.assertRaisesRegex(ValueError, "does not reach the fillet station"):
            self.chart(self.source_mesh(), np.arange(len(self.tri)), np.zeros(2), 2.,
                       [.1,.4], 0., long_domain={}, plateau=.9)

    def test_missing_upper_neighbor_is_refused_before_export(self):
        import numpy as np
        with self.assertRaisesRegex(ValueError,"Missing opposite owner"):
            self.chart(self.source_mesh(omit_neighbor=True), np.arange(len(self.tri)), np.zeros(2),
                       2., [.1,.4], 0., long_domain={})
