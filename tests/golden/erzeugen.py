"""
Erzeugt die Golden-Dateien aus der Python-Referenz.

Ausfuehren (aus dem Projektordner):
    python tests/golden/erzeugen.py
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
A = os.path.join(FIX, "edge", "a_mehrwerkzeug.nc")
B = os.path.join(FIX, "edge", "b_Übung (Test).nc")

CASES = [
    ("g01_gravur_kontur.cnc", [G, K], False, 1.0),
    ("g02_gravur_kontur_rapid.cnc", [G, K], True, 1.0),
    ("g03_kontur_gravur_kontur_rapid.cnc", [K, G, K], True, 1.0),
    ("g04_edge_ab.nc", [A, B], False, 1.0),
    ("g05_edge_ab_rapid.nc", [A, B], True, 1.0),
    ("g06_edge_ba_rapid.nc", [B, A], True, 1.0),
    ("g07_edge_a_gravur_rapid.nc", [A, G], True, 1.0),
    ("g08_edge_ab_rapid_safez5.nc", [A, B], True, 5.0),
]

if __name__ == "__main__":
    for name, paths, rapid, safe_z in CASES:
        files = [mp.NCFile(p) for p in paths]
        text, _, _ = mp.merge(files, mp.default_tool_map(files), restore=rapid, safe_z=safe_z)
        with open(os.path.join(HERE, name), "w", encoding="utf-8", newline="") as fh:
            fh.write(text)
        print("OK", name)
