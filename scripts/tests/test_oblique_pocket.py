"""Compare the STL-only pocket probe to geometry supplied by a separate generator."""

import importlib.util
import json
import math
from pathlib import Path
import runpy
import unittest


@unittest.skipUnless(
    importlib.util.find_spec("numpy"), "requires the NumPy qualification environment"
)
class ObliquePocketTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        scripts = Path(__file__).resolve().parents[1]
        cls.fixture = scripts.parent / "fixtures" / "oblique_pocket"
        cls.manifest = json.loads((cls.fixture / "manifest.json").read_text())
        cls.probe = staticmethod(
            runpy.run_path(str(scripts / "qualify-oblique-pocket.py"))["check"]
        )

    def test_source_only_fit_recovers_the_expected_curves_and_entire_wall(self):
        for name in self.manifest["positives"]:
            with self.subTest(name=name):
                expected = self.manifest["cases"][name]
                measured = self.probe(self.fixture / f"{name}.stl")
                first, end = expected["groups"]["fillet"]
                self.assertEqual(
                    measured["wall_source_triangles"], list(range(first, end))
                )
                self.assertAlmostEqual(
                    measured["radius_mm"], expected["radius_mm"], places=5
                )
                self.assertAlmostEqual(
                    measured["ellipse_major_mm"],
                    expected["ellipse_major_radius_mm"],
                    places=5,
                )
                for point in ("circular_center_mm", "ellipse_center_mm"):
                    self.assertLess(math.dist(measured[point], expected[point]), 1e-5)
                self.assertLess(measured["wall_bidirectional_bound_mm"], 0.05)

    def test_budget_below_the_facet_deviation_refuses_the_wall(self):
        with self.assertRaisesRegex(ValueError, "upper bound"):
            self.probe(self.fixture / "nominal.stl", max_deviation=0.0001)

    def test_dented_elliptical_end_is_outside_the_probe_tolerance(self):
        with self.assertRaises(ValueError):
            self.probe(self.fixture / "warped_stop.stl")
