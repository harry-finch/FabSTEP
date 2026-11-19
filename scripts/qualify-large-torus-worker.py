"""Exercise the real helper on the public dense, notched toroidal ramp."""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def qualify(worker, version=2):
    with tempfile.TemporaryDirectory(prefix="fabstep-atlas-") as scratch:
        job = Path(scratch)
        subprocess.run(
            [
                sys.executable,
                Path(__file__).with_name("qualify-flange-public.py"),
                "ramp",
                job / "source.stl",
                "--u",
                "54",
                "--v",
                "32",
                "--sparse-floor",
                "--subdivisions",
                "3",
            ],
            check=True,
        )
        with (job / "source.stl").open("rb") as stream:
            stream.seek(80)
            assert int.from_bytes(stream.read(4), "little") == 194_694
        request = {
            "version": version,
            "source_id": 1,
            "request_id": 1,
            "options": {
                "input_unit": "mm",
                "reconstruction": "auto",
                "weld_tolerance_mm": 1e-6,
                "plane_tolerance_mm": 1e-6,
                "cylinder_fit_tolerance_mm": 1e-5,
                "max_deviation_mm": 0.05,
            },
        }
        (job / "request.json").write_text(json.dumps(request))
        start = time.monotonic()
        subprocess.run([worker.resolve(), "--job-dir", job], timeout=1800, check=True)
        elapsed = time.monotonic() - start
        completion = json.loads((job / "completion.json").read_bytes())
        assert completion["version"] == version and completion["status"] == "success", completion
        report = json.loads((job / "report.json").read_bytes())
        if version == 2:
            assert report["version"] == 2
            assert (job / "report.json").stat().st_size < 1024**2
            report = report["report"]
            assert set(report["bodies"][0]["analysis"]) == {"report"}
        cad = report["cad"]
        assert (cad["solids"], cad["faces"], cad["toroidal_faces"]) == (1, 659, 1)
        assert 93.52 < cad["volume_mm3"] < 93.55
        fidelity = report["bodies"][0]["fidelity"]
        assert (
            max(fidelity["source_to_result_mm"], fidelity["result_to_source_mm"])
            < 0.005
        )
        assert 0 < (job / "result.step").stat().st_size < 128 * 1024**2
        print(f"{cad['faces']} faces, {cad['volume_mm3']:.6f} mm³", fidelity)
        print(json.dumps({"protocol": version, "helper_seconds": round(elapsed, 3),
                          "report_bytes": (job / "report.json").stat().st_size,
                          "step_bytes": (job / "result.step").stat().st_size}))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("worker", type=Path)
    parser.add_argument("--protocol", type=int, choices=[1, 2], default=2)
    args = parser.parse_args()
    qualify(args.worker, args.protocol)
