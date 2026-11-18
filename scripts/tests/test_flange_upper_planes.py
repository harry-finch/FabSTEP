"""Directed source triangles and complete rail coverage for the upper planes."""

import importlib.util
from pathlib import Path
import runpy
import unittest


@unittest.skipUnless(
    importlib.util.find_spec("numpy") and importlib.util.find_spec("scipy"),
    "requires NumPy/SciPy",
)
class UpperPlaneTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.code = runpy.run_path(
            str(Path(__file__).resolve().parents[1] / "qualify-flange-shoulders.py")
        )

    def test_invalid_interior_triangle_incidence(self):
        import numpy as np

        check = self.code["collar"]["check_triangle_incidence"]
        triangles = np.array([[0, 1, 2], [2, 1, 3]])
        check(triangles, 4)
        for bad in (triangles[[0, 1, 1]], np.array([triangles[0], triangles[1, ::-1]])):
            with self.assertRaisesRegex(ValueError, "triangle incidence"):
                check(bad, 4)

    def test_incomplete_reversed_rail(self):
        validate = self.code["validate_map"]
        rail = [dict(kind="line", start_mm=[0, 0, 0], end_mm=[0, 3, 0])]
        chain = [dict(start_mm=i + 1, end_mm=i) for i in (2, 1, 0)]
        validate([dict(native_edge=0, edges=chain)], rail)
        for partial in (chain[:-1], chain[1:], chain[::2]):
            with self.assertRaisesRegex(ValueError, "rail covering"):
                validate([dict(native_edge=0, edges=partial)], rail)

    def test_ruled_triangle_requires_an_unfolded_projection(self):
        import numpy as np

        fan = runpy.run_path(
            str(
                Path(__file__).resolve().parents[1] / "qualify-flange-trim-neighbors.py"
            )
        )["fan"]
        uv = np.array([[0.0, 0.0], [0.1, 0.02]])
        ends = np.c_[uv[:, 1], np.cos(uv[:, 0]), np.sin(uv[:, 0])]
        apex = np.array([0.5, 1.0, 0.1])
        bound, jacobian, _ = fan(np.vstack([ends, apex]), np.zeros(2), 1.0, uv, apex)
        self.assertAlmostEqual(bound, 0.00125)
        self.assertGreater(jacobian, 0)
        apex = np.array([0.01, 1.0, 0.05])
        with self.assertRaisesRegex(ValueError, "Folded projected trim"):
            fan(np.vstack([ends, apex]), np.zeros(2), 1.0, uv, apex)
