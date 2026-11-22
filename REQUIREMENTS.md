# MultiPass – Requirements for the rewrite with Tauri 2 + React

> **Brief for Claude Code**
> 1. **Phase 1 – Requirements analysis:** Read this document, `CLAUDE.md` and the reference
>    implementation `reference/multipass.py` including its tests. Check the requirements for gaps
>    and contradictions. Ask the user the **open questions (section 12)** and propose an
>    implementation plan with milestones. **Only build after approval.**
> 2. **Phase 2 – Implementation** according to the approved plan (MVP first, then extensions).
> 3. **Phase 3 – Verification:** tests, build installers, short user guide.

---

## 1. Goal

MultiPass merges several NC programs (G-code) into **one program with tool changes**.

**Background:** The user creates milling programs with **Fusion 360 Personal** for a
**Makera Z1** (post processor `Carvera.cps`). Fusion Personal has two limitations:

1. **Only one tool per output file.** A part with 4 cutters produces 5 files that have to be
   loaded into Makera Studio one after another.
2. **Rapids (G0) are written as feed moves (G1).** This greatly increases the run time. For the
   "Angel Token" project this cost about 1.5 hours.

MultiPass fixes both: one file with `Tn M6` changes and optionally restored rapids.
The toolpaths must **never** change.

A working **Python/Tkinter version** already exists (`reference/multipass.py`).
Its logic has been verified with real files and is the **functional reference**. Its GUI looks
dated to the user, hence the rewrite.

## 2. User and constraints

- **Language:** the GUI is bilingual (English/German); the repository is in English.
- **Mouse-only operation. The user has no keyboard.**
  - Every function must be reachable by clicking, without shortcuts and without mandatory text input.
  - Numbers via large **− / +** buttons or selection lists instead of text fields.
  - Files via **file dialog** and **drag & drop**, never via typed paths.
  - Large click targets (at least 44 × 44 px), clear states, no hover-only functions.
  - No double click required. A double click may be offered in addition.
- **Platforms:** Windows (main system), macOS, Linux.
- **Offline**, no telemetry, no cloud.

## 3. Technical stack

| Area | Requirement |
|---|---|
| Desktop shell | **Tauri 2** |
| Frontend | **React + TypeScript + Vite** |
| UI | modern component system: **Tailwind CSS + shadcn/ui**; icons lucide-react |
| Core logic | **Rust**: parsing, merging and rapid conversion. Reason: files are up to > 10 MB / ~1 M lines |
| Tauri plugins | `dialog` (open/save), `store` (settings), `opener` (open folder) |
| Tests | Rust unit tests + golden tests against the Python reference; frontend tests as needed |
| Build | installers for Windows (NSIS `.exe` or `.msi`), macOS (`.dmg`), Linux (`.AppImage`/`.deb`) via GitHub Actions (`tauri-action`) |

The core logic lives in its own crate (`multipass-core`) without GUI dependencies.
There is also a CLI (`multipass-cli a.cnc b.cnc -o merged.cnc --rapid`).

## 4. Functional requirements

### F1 – Loading files
- Via the "Choose files" button (multi-select; filter: `.cnc .nc .gcode .ngc .tap .txt`, "All files").
- Via **drag & drop** into the window, several files at once.
- Files passed as arguments at program start ("Open with" / drop onto the program icon).
- Several files are **sorted naturally** (`1_…`, `2_…`, `10_…`).
- Unreadable files lead to an understandable message. The others are loaded anyway.

### F2 – File list
Per file as a card or row:
- position (1, 2, 3 …), file name
- **tool** from the file header, e.g. "Spiral O Metal 3.175*12mm Makera"
- assigned **tool number** (T no.)
- **lowest point** (`Toolpath Z Min from Stock Top`)
- optional: number of lines or moves, estimated run time (see F8)

### F3 – Changing the order
- Large buttons **"Move up" / "Move down"** and **"Remove"** for the selected file.
- Additionally **drag & drop for reordering** (with a visible handle).
- "Remove all" with confirmation.

### F4 – Tool numbers
- **Automatic:** the same tool description gets the same number, new tools the next free number
  (T1, T2, …). See `default_tool_map`.
- **Manual:** change the number per file with − / + (1–99). The "Automatic numbers" button resets.
- **Inventory mapping:** store "tool description → number" permanently, so that e.g. the 3.175 mm
  metal cutter is always **T13** (inventory number #13). Stored mappings are applied automatically
  when loading. A small management view shows the mappings and allows deleting them.
  Known inventory: #13 = Spiral O Metal 3.175×12, #10 = 1.5×6, #9 = 1×3,
  #16 = engraving bit metal 30°×0.3, #21 = chamfer 90°.

### F5 – Sequence preview
A clearly visible overview "sequence at the machine":
`→ tool change to T13 (Spiral O Metal 3.175×12) → 1. 1_T13_Schruppen.cnc → tool change to T10 …`

### F6 – Option "Air moves as rapids"
- Switch, **off by default**.
- Adjustable safe height `safe_z` via − / + (0.5–20 mm, default 1.0 mm, step 0.5).
- Clear warning: "Clamps higher than this value are unknown to the program."
- Rules exactly as in section 6.

### F7 – Merge and save
- Large main button "Merge and save".
- Save dialog, suggesting the folder of the first file and the name `MERGED.cnc`
  (or the extension of the input files).
- Checks beforehand (see 5.6). Errors block, warnings are shown and must be confirmed.
- Result dialog: location, order with T numbers and the number of converted air moves with
  distance and time saved. Plus an "Open folder" button.
- **Note box in the result** about the Makera Z1:
  - Makera Studio asks "Please change to Tool n" at every change; insert the matching cutter and
    click **Confirm**.
  - **Probe Auto-Z only on untouched stock**, not in the milled area, otherwise Z is wrong.

### F8 – Run time estimate
- Per file and in total: distance / feed. Rapids use an adjustable rapid speed.
- Comparison "without / with rapid option".

### F9 – Remembering settings
Last folder, rapid option, `safe_z`, fan option, inventory mappings and window size.

### F10 – Toolpath preview and simulation
2D top view (XY) of the toolpaths, coloured per tool, rapids dashed. Since 2.1.0 also a
material removal simulation (top view height map) with playback.

### F11 – Spindle fan off during tool changes (since 2.2.0)
Optional switch, **off by default**. Writes `M812` before and `M811 S100` after every `Tn M6`.

## 5. Merge rules (binding)

Reference: `merge()` in `reference/multipass.py`.

### 5.1 Structure of an input file (Carvera.cps, Makera Z1)
```
(Program name)
(Maschine) ... (  model: Z1 3 axis)
(T1  Spiral O Metal 3.175*12mm  Makera    D=3.175 SD=3.175 ... - ZMIN=-2.55 - Schaftfraser)
(   Z Origin Set To   : Stock Top )
(   Stock Height                    : 5. mm )
(   Toolpath Z Min from Stock Top: -2.55 mm )
(   X Offset   : 0. mm )
(   Y Offset   : 0. mm )
G90 G94
G17
G21
(When using Fusion for Personal Use ... rapid ...)

(Operation name)
T1M6
(5)
M400
M851 S100
S12000 M3
G54
G0 X.. Y..
Z15.
G1 Z5. F100
...
M5
M400
M852
G28
M30
```
The German words come from a German Fusion installation and are part of the real file format.
`M851`/`M852` switch the extension port (external vacuum), not the spindle fan (`M811`/`M812`).

### 5.2 Detection
- **Tool change:** line `^\s*T\s*(\d+)\s*M0?6\b` (case-insensitive).
- **Body start:** first tool change. If a comment line (operation name) directly precedes it, it belongs to the body.
- **Program end:** last line `^\s*M0?(30|2)\b`. The body ends **before** this line.
- **Prologue** = everything before the body, **epilogue** = from `M30` to the end of the file.
- **Header info:** tool line `(T<n>  <description>  D=…)`, Z min, Z origin, X/Y offset,
  machine model, units (`G20` = inch, otherwise mm), incremental (`G91`).

### 5.3 Output
1. Comment block: "Merged with MultiPass <version> on <date>" and the order with T numbers.
2. **Prologue of the first file.** Its own tool comment line is dropped.
3. Comment tool list `(T<n>  <description>)`, sorted by number.
4. Per file: `(===== Part i: <file name> =====)`, then the body. `T<old>M6` is replaced by `T<new>M6`.
   `M5 / M400 / M852 / G28` at the end of each part are kept.
5. At the end the **epilogue of the last file** (`M30` …). There is exactly one `M30`.
6. **Line endings** as in the first file (CRLF/LF kept).
7. **All comments written by MultiPass are ASCII only.** Umlauts are transliterated (ä→ae …),
   `( )` in texts becomes `[ ]`, control characters and other non-ASCII become `_`.

### 5.4 Tool numbers
See F4. If a file contains several tools (e.g. from a full Fusion licence), each original number
is assigned separately.

### 5.5 Robustness
- A file without a tool change or without `M30`/`M2` is rejected with a clear message.
- Large files (≥ 10 MB) must be processed smoothly. No frozen GUI, a progress bar instead.
- Encoding: UTF-8, tolerate invalid bytes.
- Files already merged by MultiPass are rejected, otherwise everything would be milled twice.

### 5.6 Checks before saving
| Case | Behaviour |
|---|---|
| different units (mm/inch) | **error**, abort |
| different Z origin | warning |
| different X/Y offsets | warning ("from the same setup?") |
| different machine models | warning |
| `G91` in a file with the rapid option on | warning, rapid option turned off for this run |

## 6. Rapid restoration – safety critical

Reference: `restore_rapids()` in `reference/multipass.py`.

- Modal simulation per part (absolute coordinates, G90): current mode (G0/G1/G2/G3), position X/Y/Z, feed F.
- A **G1 linear move** only becomes `G0` if
  - **start Z and end Z ≥ `safe_z`** (entirely above the material), **or**
  - it is a **pure retract upwards** (only Z changes, Z rises).
- Only convert once X, Y and Z are known. `G28`/`G53` set the position to "unknown".
- **Never** converted: arcs (G2/G3), plunges, moves below `safe_z` and lines with other G/M words.
- Output of a converted line: `G0` + only the axes present in the original. The F word is dropped.
- If a **modal** G1/G2/G3 line without a G word follows, the matching G word is added in front.
- If an F word was dropped, the next G1/G2/G3 line without F gets the current feed appended.
- Statistics: count, distance and time previously spent at feed rate.

**Guarantee (must be covered by tests):** for every motion line the target position is identical
to the original, every cutting move has the same feed, and every conversion follows the rule above.

## 7. Non-functional requirements

- **Correctness above all.** Toolpaths must never change unnoticed.
- **Performance:** read and merge 10 MB / 1 M lines in under about 3 s, with a responsive GUI.
- **Accessibility:** mouse-only operation (see 2), readable fonts (≥ 14 px), high contrast,
  **dark and light mode** (follow the system, switchable).
- **No network access.**
- **Size:** installer as small as possible (Tauri-typical < 15 MB).
- **Error messages** concrete and with a recommended action.

## 8. GUI (original draft)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ◆ MultiPass                                           [☀/☾]  [⚙]        │
├──────────────────────────────────────────────┬───────────────────────────┤
│  ┌ Drop files here or [+ Add]               ┐│  Selected: 3_T9_…         │
│  │ ⋮⋮ 1  1_T13_Schruppen.cnc   T13  −2.55 mm ││  Tool: Spiral O 1×3       │
│  │ ⋮⋮ 2  2_T10_Rest.cnc        T10  −2.55 mm ││  Tool no.     [−] 9 [+]   │
│  │ ⋮⋮ 3  3_T9_Rest_Boeden.cnc  T9   −2.60 mm ││  [Save to inventory]      │
│  │ ⋮⋮ 4  4_T16_Gravur.cnc      T16  −2.10 mm ││  [▲ Move up][▼ Move down] │
│  │ ⋮⋮ 5  5_T13_Kontur.cnc      T13  −5.00 mm ││  [🗑 Remove]               │
│  └───────────────────────────────────────────┘├───────────────────────────┤
│  Sequence at the machine                      │  Options                  │
│  → T13 Spiral O 3.175×12 → 1. Roughing         │  [●] Air moves as rapids  │
│  → T10 Spiral O 1.5×6   → 2. Rest              │  Safe height              │
│  → T9  …                                       │  [−] 1.0 mm [+]           │
│                                                │  ⚠ clamp warning          │
├──────────────────────────────────────────────┴───────────────────────────┤
│  Run time approx. 5 h 40 min → with rapids approx. 4 h 10 min             │
│                                      [      Merge and save      ]         │
└──────────────────────────────────────────────────────────────────────────┘
```
- **Empty state:** large drop area "Drag NC files here or click".
- Modern and calm design: cards, rounded corners, clear typography, subtle animations.

## 9. Tests and acceptance

1. **Rust unit tests** for parser, tool assignment, merging and rapid rules.
2. **Golden tests:** for the fixtures (`tests/fixtures/*.cnc`) the Rust output must match the
   Python reference (`reference/multipass.py --cli … [--rapid]`). Only the time stamp line and the
   comment lines MultiPass writes itself (now English) may differ.
3. **Safety test** like `test_rapid_restore_is_safe`: positions and feeds identical,
   conversion rule followed.
4. **Large file test:** a synthetic ~1 M line program within the time limit.
5. **Manual acceptance** by the user: merge the real Angel Token files, load them into Makera Studio,
   check the tool change dialogs.

Real reference values (Angel Token, 5 files, ~950,000 moves): all positions identical,
2,487 air moves converted (16.0 m, previously approx. 94 min at feed rate).

## 10. Deliverables

- Source code (Git), `README.md` with "Install", "Usage" and "Development"
- Installers for Windows, macOS and Linux (unsigned)
- Short user guide (`GUIDE.md`, German `KURZANLEITUNG.md`)
- `reference/` is kept (Python version and tests)

## 11. Milestones

1. **M1 Core:** Rust crate with parser, merge and rapids, incl. tests and golden comparison
2. **M2 MVP GUI:** loading (dialog + drag & drop), list, sorting, T numbers, option, saving
3. **M3 Comfort:** inventory mapping, settings, sequence preview, result dialog with Makera notes, dark mode
4. **M4 Extension:** run time estimate, 2D preview
5. **M5 Delivery:** installers via GitHub Actions, README, quick guide

## 12. Open questions (resolved in phase 1, see PHASE1_ANALYSIS.md)

1. Is **Node.js** installed, and which version? Should missing tools be installed?
2. Should the project live on **GitHub** (for automatic builds on all three systems)? Public or private?
3. **Sign the installers?** Unsigned installers show a warning on Windows/macOS. Signing may cost money.
4. Should the **inventory mapping** be pre-filled with the known cutters (#13, #10, #9, #16, #21)?
5. **Automatic updates** (Tauri updater), or is a new installer enough?
6. Only **German**, or English as well?
7. Is the **2D toolpath preview** wanted, and in which milestone?
8. Any wishes regarding **colours/logo**?
9. Should the Python version still be shipped, or only stay in the repo as a reference?
