"""Public refusal controls for the measured torus chart and its native joins."""

import argparse
from pathlib import Path
import subprocess
import tempfile


def controls(folder):
    original = (folder / "charts.txt").read_text().splitlines()
    binary = folder / "qualify-flange-residual"
    if int(original[0]) != 1:
        raise ValueError("Expected the independent notched ramp chart")
    with tempfile.TemporaryDirectory() as work:
        out = Path(work)
        for kind, expected in (
            ("duplicate", "duplicate chart owner"),
            ("radius", "boundary vertex differs from source"),
            ("winding", "folded torus source triangle"),
            ("deviation", "torus witness understates source deviation"),
        ):
            data = original.copy()
            header = data[1].split()
            ids = data[2].split()
            if kind == "duplicate":
                ids[0] = ids[1]
            if kind == "radius":
                header[4] = str(float(header[4]) + 0.2)
            if kind == "winding":
                header[5] = str(-int(header[5]))
            if kind == "deviation":
                header[-1] = "0"
            data[1] = " ".join(header)
            data[2] = " ".join(ids)
            witness = out / f"{kind}.txt"
            witness.write_text("\n".join(data) + "\n")
            candidate = out / f"{kind}.step"
            proc = subprocess.run(
                [
                    str(binary),
                    "--source-only",
                    str(folder / "witness.txt"),
                    str(candidate),
                    str(witness),
                ],
                capture_output=True,
                text=True,
                timeout=120,
            )
            if (
                proc.returncode == 0
                or expected not in proc.stderr
                or candidate.exists()
            ):
                raise ValueError(
                    f"{kind} did not produce the expected refusal: {proc.stderr}"
                )
            print(f"{kind}: refused")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("public_output", type=Path)
    controls(parser.parse_args().public_output)
