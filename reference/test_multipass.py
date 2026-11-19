"""
Tests für MultiPass.

Ausführen (aus dem Projektordner):
    python reference/test_multipass.py
"""
import os
import re
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)

import multipass as mp  # noqa: E402

FIX = os.path.join(os.path.dirname(HERE), "tests", "fixtures")
FILES = [os.path.join(FIX, n) for n in ("4_T16_Gravur.cnc", "5_T13_Kontur.cnc")]

W = re.compile(r"([A-Z])\s*([-+]?(?:\d+\.?\d*|\.\d+))")
C = re.compile(r"\([^)]*\)|;.*$")


def simulate(text):
    """Einfache Modal-Simulation: liefert pro Bewegungszeile (modus, vorschub, x, y, z, start_z)."""
    mode = None
    feed = None
    pos = {"X": None, "Y": None, "Z": None}
    res = []
    for line in text.splitlines():
        code = C.sub("", line).strip().upper()
        if not code:
            continue
        w = [(a, float(b)) for a, b in W.findall(code)]
        g = [v for a, v in w if a == "G"]
        for v in g:
            if v in (0, 1, 2, 3):
                mode = int(v)
        for a, v in w:
            if a == "F":
                feed = v
        co = {a: v for a, v in w if a in "XYZ"}
        reset = any(x in (28, 53) for x in g)
        if co and not reset:
            start_z = pos["Z"]
            pos.update(co)
            res.append((mode, feed if mode in (1, 2, 3) else None, pos["X"], pos["Y"], pos["Z"], start_z))
        if reset:
            pos = {"X": None, "Y": None, "Z": None}
    return res


def _merge(restore):
    files = [mp.NCFile(p) for p in FILES]
    text, report, warns = mp.merge(files, mp.default_tool_map(files), restore=restore, safe_z=1.0)
    return files, text, report, warns


def test_parse_header():
    f = mp.NCFile(FILES[0])
    assert f.tool_numbers == [1]
    assert "Engraving" in f.tool_desc
    assert abs(f.zmin - (-2.1)) < 1e-9
    assert f.zorigin == "Stock Top"
    assert f.units == "mm"


def test_merge_structure():
    files, text, _, warns = _merge(False)
    assert not warns
    lines = text.splitlines()
    # genau ein Programmende, ganz am Schluss
    ends = [l for l in lines if re.match(r"^\s*M0?(30|2)\b", l)]
    assert len(ends) == 1 and lines[-1].strip() == "M30"
    # Werkzeugwechsel in der richtigen Reihenfolge und neu nummeriert
    tcs = [l.strip() for l in lines if mp.TOOLCHANGE_RE.match(l)]
    assert tcs == ["T1M6", "T2M6"]
    # Bahnen identisch zum Original (ohne Eilgang-Option)
    orig = []
    for f in files:
        orig += simulate("\n".join(f.body))
    assert [r[2:5] for r in simulate(text)] == [r[2:5] for r in orig]
    # nur ASCII in der Ausgabe
    assert all(ord(c) < 128 for c in text)


def test_same_tool_gets_same_number():
    files = [mp.NCFile(p) for p in FILES + FILES[:1]]
    maps = mp.default_tool_map(files)
    assert maps[0][1] == maps[2][1] == 1 and maps[1][1] == 2


def test_rapid_restore_is_safe():
    _, plain, _, _ = _merge(False)
    _, fast, report, _ = _merge(True)
    a, b = simulate(plain), simulate(fast)
    assert len(a) == len(b)
    converted = 0
    for x, y in zip(a, b):
        assert x[2:5] == y[2:5], "Position verändert"
        if x[0] != y[0]:
            converted += 1
            assert x[0] == 1 and y[0] == 0
            start_z, end_z = x[5], x[4]
            assert (start_z >= 1.0 and end_z >= 1.0) or end_z > start_z, "Eilgang zu tief"
        elif x[0] in (1, 2, 3):
            assert x[1] == y[1], "Vorschub verändert"
    assert converted > 0
    assert any("Eilgang" in r for r in report)


def test_write_file_roundtrip():
    _, text, _, _ = _merge(True)
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "GESAMT.cnc")
        with open(p, "w", encoding="utf-8", newline="") as fh:
            fh.write(text)
        # zusammengefügte Datei lässt sich wieder einlesen
        again = mp.NCFile(p)
        assert again.tool_numbers == [1, 2]


if __name__ == "__main__":
    for name, fn in list(globals().items()):
        if name.startswith("test_") and callable(fn):
            fn()
            print("OK ", name)
    print("Alle Tests bestanden.")
