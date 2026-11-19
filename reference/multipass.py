#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
MultiPass
=========

Fügt mehrere NC-Programme (z. B. aus Fusion 360 Personal, je ein Werkzeug pro
Datei) zu einem einzigen Programm mit Werkzeugwechseln zusammen.

- läuft unter Windows, macOS und Linux (Python 3 + Tkinter)
- komplett mit der Maus bedienbar
- optional: Leerfahrten über dem Material wieder als Eilgang (G0) ausgeben

Getestet mit dem Postprozessor "Carvera.cps" (Makera Z1 / Carvera).
"""

import os
import re
import sys
import datetime

APP_NAME = "MultiPass"
APP_VERSION = "1.0"

# --------------------------------------------------------------------------
# Kernlogik (ohne Oberfläche, damit sie auch einzeln getestet werden kann)
# --------------------------------------------------------------------------

TOOLCHANGE_RE = re.compile(r"^\s*T\s*(\d+)\s*M0?6\b", re.IGNORECASE)
TOOLCHANGE_SUB_RE = re.compile(r"\bT\s*(\d+)(\s*M0?6\b)", re.IGNORECASE)
END_RE = re.compile(r"^\s*M0?(30|2)\b(?!\d)", re.IGNORECASE)
HEADER_TOOL_RE = re.compile(r"^\(T(\d+)\s+(.*?)\s{2,}D=", re.IGNORECASE)
HEADER_TOOL_FALLBACK_RE = re.compile(r"^\(T(\d+)\s+(.*?)\)\s*$", re.IGNORECASE)
ZMIN_RE = re.compile(r"Z\s*Min\s+from\s+Stock\s+Top\s*:\s*(-?[\d.]+)", re.IGNORECASE)
ZORIGIN_RE = re.compile(r"Z\s*Origin\s+Set\s+To\s*:\s*(.*?)\s*\)", re.IGNORECASE)
XOFF_RE = re.compile(r"^\(\s*X\s+Offset\s*:\s*(-?[\d.]+)", re.IGNORECASE)
YOFF_RE = re.compile(r"^\(\s*Y\s+Offset\s*:\s*(-?[\d.]+)", re.IGNORECASE)
MODEL_RE = re.compile(r"model\s*:\s*(.*?)\s*\)", re.IGNORECASE)
WORD_RE = re.compile(r"([A-Z])\s*([-+]?(?:\d+\.?\d*|\.\d+))", re.IGNORECASE)
COMMENT_RE = re.compile(r"\([^)]*\)|;.*$")


class NCError(Exception):
    pass


class NCFile:
    """Eine eingelesene NC-Datei."""

    def __init__(self, path):
        self.path = path
        self.name = os.path.basename(path)
        with open(path, "r", encoding="utf-8", errors="replace", newline="") as f:
            raw = f.read()
        self.newline = "\r\n" if "\r\n" in raw else "\n"
        self.lines = raw.splitlines()

        # Werkzeugwechsel suchen
        tc = [i for i, l in enumerate(self.lines) if TOOLCHANGE_RE.match(l)]
        if not tc:
            raise NCError(f"{self.name}: kein Werkzeugwechsel (T.. M6) gefunden.")
        start = tc[0]
        # Kommentarzeile mit dem Operationsnamen direkt davor mitnehmen
        if start > 0 and self.lines[start - 1].strip().startswith("("):
            start -= 1
        self.body_start = start

        ends = [i for i, l in enumerate(self.lines) if END_RE.match(l)]
        if not ends:
            raise NCError(f"{self.name}: kein Programmende (M30 / M2) gefunden.")
        self.body_end = ends[-1]          # Index der M30-Zeile (nicht enthalten)
        if self.body_end <= self.body_start:
            raise NCError(f"{self.name}: Aufbau nicht erkannt.")

        self.prolog = self.lines[:self.body_start]
        self.body = self.lines[self.body_start:self.body_end]
        self.epilog = self.lines[self.body_end:]   # M30 und alles danach

        # Infos aus dem Kopf
        self.tools = {}           # Originalnummer -> Beschreibung
        self.zmin = None
        self.zorigin = None
        self.xoff = None
        self.yoff = None
        self.model = None
        for l in self.prolog:
            s = l.strip()
            m = HEADER_TOOL_RE.match(s) or HEADER_TOOL_FALLBACK_RE.match(s)
            if m and m.group(1).isdigit() and int(m.group(1)) not in self.tools:
                desc = re.sub(r"\s+", " ", m.group(2)).strip()
                self.tools[int(m.group(1))] = desc
            m = ZMIN_RE.search(s)
            if m:
                self.zmin = float(m.group(1))
            m = ZORIGIN_RE.search(s)
            if m:
                self.zorigin = m.group(1).strip()
            m = XOFF_RE.match(s)
            if m:
                self.xoff = float(m.group(1))
            m = YOFF_RE.match(s)
            if m:
                self.yoff = float(m.group(1))
            m = MODEL_RE.search(s)
            if m and self.model is None:
                self.model = m.group(1).strip()

        # in der Datei verwendete Werkzeugnummern (Reihenfolge des Auftretens)
        self.tool_numbers = []
        for l in self.body:
            m = TOOLCHANGE_RE.match(l)
            if m:
                n = int(m.group(1))
                if n not in self.tool_numbers:
                    self.tool_numbers.append(n)
        for n in self.tool_numbers:
            self.tools.setdefault(n, f"Werkzeug {n}")

        # Grundeinstellungen (G90/G91, G20/G21) aus dem Vorspann
        pro = " ".join(COMMENT_RE.sub("", l).upper() for l in self.prolog)
        self.units = "inch" if re.search(r"\bG20\b", pro) else "mm"
        self.incremental = bool(re.search(r"\bG91\b", pro)) or any(
            re.search(r"\bG91\b", COMMENT_RE.sub("", l).upper()) for l in self.body)

    
    @property
    def tool_desc(self):
        if self.tool_numbers:
            return self.tools.get(self.tool_numbers[0], "?")
        return next(iter(self.tools.values()), "?")


def _parse_words(code):
    """Zerlegt eine Zeile (ohne Kommentar) in eine Liste (Buchstabe, Wert)."""
    return [(a.upper(), b) for a, b in WORD_RE.findall(code)]


def _fmt(v):
    s = f"{v:.4f}".rstrip("0")
    return "0." if s in ("-0.", "-0", "0") else s


def restore_rapids(lines, safe_z):
    """
    Wandelt Vorschubbewegungen (G1), die vollständig über der Höhe safe_z liegen,
    sowie reine Rückzüge nach oben wieder in Eilgänge (G0) um.
    Bögen, Eintauchbewegungen und Bewegungen im Material bleiben unverändert.
    Gibt (neue_zeilen, anzahl_umgewandelt, strecke_mm, zeit_min_vorher) zurück.
    """
    out = []
    mode_in = None          # aktueller Bewegungsmodus laut Original
    mode_out = None         # zuletzt ausgegebener Bewegungsmodus
    pos = {"X": None, "Y": None, "Z": None}
    feed = None             # aktueller Vorschub laut Original
    feed_dirty = False      # ein F-Wort wurde weggelassen -> beim nächsten G1/G2/G3 wieder setzen
    converted = 0
    dist = 0.0
    time_before = 0.0

    for line in lines:
        code = COMMENT_RE.sub("", line).strip()
        if not code:
            out.append(line)
            continue
        words = _parse_words(code)
        letters = [w[0] for w in words]
        gvals = [float(v) for a, v in words if a == "G"]
        motion_g = [g for g in gvals if g in (0, 1, 2, 3)]

        # Werkzeugwechsel / Spindel / sonstige Befehle -> unverändert
        coords = {a: float(v) for a, v in words if a in ("X", "Y", "Z")}
        only_motion = set(letters) <= {"G", "X", "Y", "Z", "F"} and \
            all(g in (0, 1) for g in gvals) and len(gvals) <= 1

        if motion_g:
            new_mode = int(motion_g[-1])
        else:
            new_mode = mode_in

        fvals = [float(v) for a, v in words if a == "F"]
        if fvals:
            feed = fvals[-1]

        if coords and only_motion and new_mode == 1 and None not in pos.values():
            start = dict(pos)
            end = dict(pos)
            end.update(coords)
            dx = end["X"] - start["X"]
            dy = end["Y"] - start["Y"]
            dz = end["Z"] - start["Z"]
            length = (dx * dx + dy * dy + dz * dz) ** 0.5
            pure_up = (abs(dx) < 1e-9 and abs(dy) < 1e-9 and dz > 0)
            in_air = start["Z"] >= safe_z and end["Z"] >= safe_z
            if (pure_up or in_air) and length > 0:
                parts = ["G0"] + [f"{a}{_fmt(end[a])}" for a in ("X", "Y", "Z") if a in coords]
                out.append(" ".join(parts))
                mode_out = 0
                mode_in = 1
                if fvals:
                    feed_dirty = True
                converted += 1
                dist += length
                if feed:
                    time_before += length / feed
                pos = end
                continue

        # normale Zeile ausgeben – ggf. Modus / Vorschub wiederherstellen
        if coords and new_mode in (1, 2, 3) and mode_out == 0 and not motion_g:
            # Zeile war modal (ohne G-Wort), Ausgabe ist aber noch im G0-Modus
            line = f"G{new_mode} " + line.lstrip()
        if coords and new_mode in (1, 2, 3):
            if feed_dirty and not fvals and feed:
                line = line.rstrip() + f" F{_fmt(feed)}"
            feed_dirty = False
        out.append(line)
        if new_mode is not None and (coords or motion_g):
            mode_out = new_mode
        mode_in = new_mode
        for a in coords:
            pos[a] = coords[a]
        # G28 / G53 o. ä. -> Position unbekannt
        if any(g in (28, 30, 53) for g in gvals):
            pos = {"X": None, "Y": None, "Z": None}

    return out, converted, dist, time_before


def merge(files, tool_map, restore=False, safe_z=1.0):
    """
    files:     Liste von NCFile in Bearbeitungsreihenfolge
    tool_map:  Liste (pro Datei) von dict Originalnummer -> neue Nummer
    Rückgabe:  (text, bericht_zeilen, warnungen)
    """
    if not files:
        raise NCError("Keine Dateien ausgewählt.")
    warnings = []
    first = files[0]

    units = {f.units for f in files}
    if len(units) > 1:
        raise NCError("Die Dateien verwenden unterschiedliche Einheiten (mm / Zoll).")
    zor = {f.zorigin for f in files if f.zorigin}
    if len(zor) > 1:
        warnings.append("Unterschiedlicher Z-Nullpunkt in den Dateien: " + ", ".join(sorted(zor)))
    offs = {(f.xoff, f.yoff) for f in files if f.xoff is not None}
    if len(offs) > 1:
        warnings.append("Unterschiedliche X/Y-Versätze in den Dateikopfen – stammen die Dateien aus demselben Setup?")
    models = {f.model for f in files if f.model}
    if len(models) > 1:
        warnings.append("Dateien für unterschiedliche Maschinen: " + ", ".join(sorted(models)))
    if restore and any(f.incremental for f in files):
        warnings.append("Eine Datei nutzt G91 (inkrementell) – Eilgang-Umwandlung wurde deshalb abgeschaltet.")
        restore = False

    nl = first.newline
    stamp = datetime.datetime.now().strftime("%Y-%m-%d %H:%M")
    out = []
    out.append(f"(Zusammengefuegt mit {ascii_safe(APP_NAME)} {APP_VERSION} am {stamp})")
    out.append("(Reihenfolge:)")
    for i, (f, tm) in enumerate(zip(files, tool_map), 1):
        tl = ", ".join(f"T{tm.get(n, n)} {f.tools.get(n, '')}" for n in f.tool_numbers)
        out.append(f"(  {i}. {ascii_safe(f.name)} -> {ascii_safe(tl)})")
    out.append("")

    # Vorspann der ersten Datei (Kopfkommentare + G90/G17/G21), Werkzeugzeilen angepasst
    for l in first.prolog:
        m = re.match(r"^\(T(\d+)(\s.*)$", l.strip())
        if m and int(m.group(1)) in first.tools:
            continue   # Werkzeug-Kommentar der ersten Datei – wird unten neu geschrieben
        out.append(l)
    # Werkzeugliste
    seen = {}
    for f, tm in zip(files, tool_map):
        for n in f.tool_numbers:
            t = tm.get(n, n)
            seen.setdefault(t, f.tools.get(n, ""))
    tool_block = [f"(T{t}  {ascii_safe(d)})" for t, d in sorted(seen.items())]
    out.extend(["(Werkzeuge:)"] + tool_block + [""])

    report = []
    total_conv = 0
    total_dist = 0.0
    total_time = 0.0
    for i, (f, tm) in enumerate(zip(files, tool_map), 1):
        body = []
        for l in f.body:
            if TOOLCHANGE_RE.match(l):
                l = TOOLCHANGE_SUB_RE.sub(lambda m: f"T{tm.get(int(m.group(1)), int(m.group(1)))}{m.group(2)}", l)
            body.append(l)
        if restore:
            body, conv, dist, tb = restore_rapids(body, safe_z)
            total_conv += conv
            total_dist += dist
            total_time += tb
        out.append(f"(===== Teil {i}: {ascii_safe(f.name)} =====)")
        out.extend(body)
        out.append("")
        report.append(f"{i}. {f.name}  ->  " +
                      ", ".join(f"T{tm.get(n, n)} ({f.tools.get(n, '')})" for n in f.tool_numbers))

    # Programmende der letzten Datei (M30 + evtl. Rest wie '%')
    out.extend(files[-1].epilog)
    if restore:
        report.append("")
        report.append(f"Eilgang: {total_conv} Leerfahrten umgewandelt "
                      f"({total_dist / 1000:.1f} m, vorher ca. {total_time:.0f} min im Vorschub).")
    return nl.join(out) + nl, report, warnings


def ascii_safe(text):
    """Kommentare im NC-Programm nur mit ASCII-Zeichen (manche Steuerungen mögen keine Umlaute)."""
    rep = {"ä": "ae", "ö": "oe", "ü": "ue", "Ä": "Ae", "Ö": "Oe", "Ü": "Ue", "ß": "ss", "(": "[", ")": "]"}
    return "".join(rep.get(c, c if ord(c) < 128 else "_") for c in text)


def default_tool_map(files):
    """Gleiche Werkzeuge (gleiche Beschreibung) bekommen dieselbe Nummer, neue Werkzeuge die nächste freie."""
    by_desc = {}
    maps = []
    nxt = 1
    for f in files:
        m = {}
        for n in f.tool_numbers:
            d = f.tools.get(n, f"#{n}")
            if d not in by_desc:
                by_desc[d] = nxt
                nxt += 1
            m[n] = by_desc[d]
        maps.append(m)
    return maps


def natural_key(s):
    return [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", s)]


# --------------------------------------------------------------------------
# Oberfläche
# --------------------------------------------------------------------------

def run_gui(initial_paths=None):
    import tkinter as tk
    from tkinter import ttk, filedialog, messagebox
    import tkinter.font as tkfont

    root = tk.Tk()
    root.title(f"{APP_NAME} {APP_VERSION}")
    root.minsize(980, 680)
    root.geometry("1100x760")

    # etwas größere Schrift für bessere Lesbarkeit
    for fname in ("TkDefaultFont", "TkTextFont", "TkHeadingFont", "TkMenuFont"):
        try:
            tkfont.nametofont(fname).configure(size=11)
        except tk.TclError:
            pass
    style = ttk.Style(root)
    style.configure("Treeview", rowheight=28)
    style.configure("Big.TButton", padding=(10, 8))
    style.configure("Go.TButton", padding=(12, 12), font=("TkDefaultFont", 12, "bold"))

    state = {"files": [], "maps": [], "manual": False}

    # --- Layout ---------------------------------------------------------------
    main = ttk.Frame(root, padding=12)
    main.pack(fill="both", expand=True)
    main.columnconfigure(0, weight=1)
    main.rowconfigure(1, weight=1)

    hdr = ttk.Frame(main)
    hdr.grid(row=0, column=0, columnspan=3, sticky="ew", pady=(0, 6))
    ttk.Label(hdr, text=APP_NAME, font=("TkDefaultFont", 18, "bold")).pack(side="left")
    ttk.Label(hdr, text="   NC-Programme in Bearbeitungsreihenfolge:",
              font=("TkDefaultFont", 12, "bold")).pack(side="left", pady=(6, 0))

    cols = ("nr", "datei", "werkzeug", "tnr", "tiefe")
    tree = ttk.Treeview(main, columns=cols, show="headings", selectmode="browse", height=10)
    tree.heading("nr", text="#")
    tree.heading("datei", text="Datei")
    tree.heading("werkzeug", text="Werkzeug (aus dem Dateikopf)")
    tree.heading("tnr", text="Werkzeug-Nr.")
    tree.heading("tiefe", text="tiefster Punkt")
    tree.column("nr", width=40, anchor="center", stretch=False)
    tree.column("datei", width=200)
    tree.column("werkzeug", width=300)
    tree.column("tnr", width=110, anchor="center", stretch=False)
    tree.column("tiefe", width=110, anchor="center", stretch=False)
    tree.grid(row=1, column=0, sticky="nsew")
    sb = ttk.Scrollbar(main, orient="vertical", command=tree.yview)
    sb.grid(row=1, column=1, sticky="ns")
    tree.configure(yscrollcommand=sb.set)

    side = ttk.Frame(main, padding=(12, 0, 0, 0))
    side.grid(row=1, column=2, sticky="ns")

    # Optionen
    opt = ttk.LabelFrame(main, text="Optionen", padding=10)
    opt.grid(row=2, column=0, columnspan=3, sticky="ew", pady=(10, 0))
    rapid_var = tk.BooleanVar(value=False)
    safez_var = tk.DoubleVar(value=1.0)
    ttk.Checkbutton(opt, text="Leerfahrten über dem Material wieder als Eilgang (G0) ausgeben  "
                              "– spart viel Zeit bei Dateien aus Fusion Personal",
                    variable=rapid_var).grid(row=0, column=0, columnspan=3, sticky="w")
    zrow = ttk.Frame(opt)
    zrow.grid(row=1, column=0, columnspan=3, sticky="w", padx=(24, 0), pady=(4, 0))
    ttk.Label(zrow, text="nur oberhalb von Z =").pack(side="left")
    ttk.Spinbox(zrow, from_=0.2, to=20.0, increment=0.5, textvariable=safez_var, width=6,
                state="readonly").pack(side="left", padx=4)
    ttk.Label(zrow, text="mm über der Material-Oberkante (Z0)").pack(side="left")
    ttk.Label(opt, text="Achtung: Spannpratzen, die höher sind als dieser Wert, kennt das Programm nicht!",
              foreground="#8a4b00").grid(row=2, column=0, columnspan=3, sticky="w", padx=(24, 0), pady=(2, 0))

    # Statusbereich
    status = tk.Text(main, height=9, wrap="word", relief="flat", background=root.cget("background"))
    status.grid(row=3, column=0, sticky="ew", pady=(10, 0))
    ssb = ttk.Scrollbar(main, orient="vertical", command=status.yview)
    ssb.grid(row=3, column=1, sticky="ns", pady=(10, 0))
    status.configure(yscrollcommand=ssb.set)
    status.configure(state="disabled")

    def set_status(text):
        status.configure(state="normal")
        status.delete("1.0", "end")
        status.insert("1.0", text)
        status.configure(state="disabled")

    # Werkzeugnummer-Bearbeitung
    tbox = ttk.LabelFrame(side, text="Werkzeug-Nr. der Auswahl", padding=8)
    tvar = tk.IntVar(value=1)
    ttk.Spinbox(tbox, from_=1, to=99, textvariable=tvar, width=5, state="readonly").pack(side="left")
    def apply_tool():
        idx = selected()
        if idx is None:
            return
        f = state["files"][idx]
        for n in f.tool_numbers:
            state["maps"][idx][n] = int(tvar.get())
        state["manual"] = True
        refresh(idx)
    ttk.Button(tbox, text="Setzen", command=apply_tool).pack(side="left", padx=(6, 0))

    def selected():
        sel = tree.selection()
        if not sel:
            return None
        return int(sel[0])

    def on_select(_evt=None):
        idx = selected()
        if idx is None:
            return
        f = state["files"][idx]
        if f.tool_numbers:
            tvar.set(state["maps"][idx].get(f.tool_numbers[0], 1))
    tree.bind("<<TreeviewSelect>>", on_select)

    def refresh(select=None):
        if not state["manual"]:
            state["maps"] = default_tool_map(state["files"])
        tree.delete(*tree.get_children())
        for i, f in enumerate(state["files"]):
            tnr = ", ".join(f"T{state['maps'][i].get(n, n)}" for n in f.tool_numbers)
            depth = f"{f.zmin:.2f} mm" if f.zmin is not None else "–"
            tree.insert("", "end", iid=str(i), values=(i + 1, f.name, f.tool_desc, tnr, depth))
        if select is not None and 0 <= select < len(state["files"]):
            tree.selection_set(str(select))
            tree.see(str(select))
        # Zusammenfassung
        if not state["files"]:
            set_status("Noch keine Dateien. Klicke auf „Dateien hinzufügen …“.\n\n"
                       "Tipp: Dateien wie „1_…“, „2_…“ werden automatisch nach Nummer sortiert.")
            return
        lines = ["Ablauf an der Maschine:"]
        last = None
        for i, f in enumerate(state["files"]):
            for n in f.tool_numbers:
                t = state["maps"][i].get(n, n)
                if t != last:
                    lines.append(f"  → Werkzeugwechsel auf T{t}: {f.tools.get(n, '')}")
                    last = t
            lines.append(f"      {i + 1}. {f.name}")
        set_status("\n".join(lines))

    def add_files():
        paths = filedialog.askopenfilenames(
            title="NC-Programme auswählen",
            filetypes=[("NC-Programme", "*.cnc *.nc *.gcode *.ngc *.tap *.txt"), ("Alle Dateien", "*.*")])
        if not paths:
            return
        errors = []
        for p in sorted(paths, key=lambda p: natural_key(os.path.basename(p))):
            try:
                state["files"].append(NCFile(p))
            except Exception as e:  # noqa
                errors.append(str(e))
        state["manual"] = False
        refresh(len(state["files"]) - 1)
        if errors:
            messagebox.showwarning(APP_NAME, "Diese Dateien konnten nicht gelesen werden:\n\n" + "\n".join(errors))

    def move(delta):
        idx = selected()
        if idx is None:
            return
        j = idx + delta
        if 0 <= j < len(state["files"]):
            fl, mp = state["files"], state["maps"]
            fl[idx], fl[j] = fl[j], fl[idx]
            if state["manual"]:
                mp[idx], mp[j] = mp[j], mp[idx]
            refresh(j)

    def remove():
        idx = selected()
        if idx is None:
            return
        del state["files"][idx]
        if state["manual"]:
            del state["maps"][idx]
        refresh(min(idx, len(state["files"]) - 1))

    def clear():
        state["files"].clear()
        state["maps"].clear()
        state["manual"] = False
        refresh()

    def auto_numbers():
        state["manual"] = False
        refresh(selected())

    def do_merge():
        if not state["files"]:
            messagebox.showinfo(APP_NAME, "Bitte zuerst Dateien hinzufügen.")
            return
        try:
            text, report, warns = merge(state["files"], state["maps"],
                                        restore=rapid_var.get(), safe_z=float(safez_var.get()))
        except NCError as e:
            messagebox.showerror(APP_NAME, str(e))
            return
        if warns:
            if not messagebox.askyesno(APP_NAME, "Hinweise:\n\n• " + "\n• ".join(warns) +
                                       "\n\nTrotzdem speichern?"):
                return
        first = state["files"][0]
        ext = os.path.splitext(first.name)[1] or ".cnc"

        path = filedialog.asksaveasfilename(
            title="Zusammengefügtes Programm speichern",
            initialdir=os.path.dirname(first.path),
            initialfile=f"GESAMT{ext}",
            defaultextension=ext,
            filetypes=[("NC-Programm", f"*{ext}"), ("Alle Dateien", "*.*")])
        if not path:
            return
        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write(text)
        messagebox.showinfo(APP_NAME, "Gespeichert:\n" + path + "\n\n" + "\n".join(report))

    for txt, cmd in (("Dateien hinzufügen …", add_files),
                     ("▲  Nach oben", lambda: move(-1)),
                     ("▼  Nach unten", lambda: move(1)),
                     ("Entfernen", remove),
                     ("Alle entfernen", clear)):
        ttk.Button(side, text=txt, command=cmd, style="Big.TButton").pack(fill="x", pady=3)
    ttk.Separator(side).pack(fill="x", pady=8)
    tbox.pack(fill="x", pady=3)
    ttk.Button(side, text="Nummern automatisch", command=auto_numbers).pack(fill="x", pady=3)
    ttk.Button(main, text="Zusammenfügen und\nspeichern …", command=do_merge,
               style="Go.TButton").grid(row=3, column=2, sticky="nsew", padx=(12, 0), pady=(10, 0))

    refresh()
    if initial_paths:
        # Dateien, die z. B. auf die Startdatei gezogen wurden, direkt laden
        errs = []
        for p in sorted(initial_paths, key=lambda p: natural_key(os.path.basename(p))):
            try:
                state["files"].append(NCFile(p))
            except Exception as e:  # noqa
                errs.append(str(e))
        refresh(0 if state["files"] else None)
        if errs:
            root.after(300, lambda: messagebox.showwarning(APP_NAME, "\n".join(errs)))
    root.mainloop()


if __name__ == "__main__":
    # Kommandozeile: multipass.py --cli a.cnc b.cnc ... -o gesamt.cnc [--rapid]
    if len(sys.argv) > 1 and sys.argv[1] == "--cli":
        args = sys.argv[2:]
        outp = "GESAMT.cnc"
        rapid = False
        if "--rapid" in args:
            rapid = True
            args.remove("--rapid")
        if "-o" in args:
            i = args.index("-o")
            outp = args[i + 1]
            del args[i:i + 2]
        fl = [NCFile(p) for p in args]
        txt, rep, warn = merge(fl, default_tool_map(fl), restore=rapid)
        with open(outp, "w", encoding="utf-8", newline="") as fh:
            fh.write(txt)
        print("\n".join(rep))
        for w in warn:
            print("HINWEIS:", w)
    else:
        run_gui([p for p in sys.argv[1:] if os.path.isfile(p)])
