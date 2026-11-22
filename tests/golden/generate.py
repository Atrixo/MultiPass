"""
Generates the golden files from the Python reference.

Run from the project folder:
    python tests/golden/generate.py
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "reference"))

import multipass as mp  # noqa: E402

FIX = os.path.join(ROOT, "tests", "fixtures")
G = os.path.join(FIX, "4_T16_Gravur.cnc")
K = os.path.join(FIX, "5_T13_Kontur.cnc")
A = os.path.join(FIX, "edge", "a_multi_tool.nc")
B = os.path.join(FIX, "edge", "b_Übung (Test).nc")

CASES = [
    ("g01_engrave_contour.cnc", [G, K], False, 1.0),
    ("g02_engrave_contour_rapid.cnc", [G, K], True, 1.0),
    ("g03_contour_engrave_contour_rapid.cnc", [K, G, K], True, 1.0),
    ("g04_edge_ab.nc", [A, B], False, 1.0),
    ("g05_edge_ab_rapid.nc", [A, B], True, 1.0),
    ("g06_edge_ba_rapid.nc", [B, A], True, 1.0),
    ("g07_edge_a_engrave_rapid.nc", [A, G], True, 1.0),
    ("g08_edge_ab_rapid_safez5.nc", [A, B], True, 5.0),
]

if __name__ == "__main__":
    for name, paths, rapid, safe_z in CASES:
        files = [mp.NCFile(p) for p in paths]
        text, _, _ = mp.merge(files, mp.default_tool_map(files), restore=rapid, safe_z=safe_z)
        with open(os.path.join(HERE, name), "w", encoding="utf-8", newline="") as fh:
            fh.write(text)
        print("OK", name)
