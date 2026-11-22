# MultiPass – Phase 1: Requirements analysis

As of 2026-11-19. Based on `REQUIREMENTS.md`, `CLAUDE.md`, `reference/multipass.py`, `reference/test_multipass.py`.

## 1. Environment (checked)

| Tool | Finding |
|---|---|
| Node.js | v22.22.1, npm 11.6.2 – sufficient for Vite/Tauri |
| Rust | rustc/cargo 1.94.1, target `x86_64-pc-windows-msvc` |
| MSVC | Visual Studio Build Tools 2019 present |
| WebView2 | 157 installed |
| git / gh | present (2.45 / 2.102) |
| Python | 3.14 – reference tests pass (5/5) |

Most likely nothing needs to be installed. The project folder was not yet a Git repository.

## 2. Gaps and contradictions

### Safety (rapids)
1. **Z origin not checked.** `safe_z` is only valid if Z0 = stock top. If the origin is e.g. "Model Bottom", `safe_z = 1 mm` would be inside the material. The reference does not check this.
   **Proposal:** only allow the rapid option if all files report `Z Origin Set To : Stock Top`; otherwise warn and turn the option off for this run (like with G91). This deliberately differs from the reference and gets its own test; the golden tests are unaffected because the fixtures use "Stock Top".
2. **G30** also resets the position in the reference, while section 6 only mentions G28/G53. The reference behaviour (G28/G30/G53) is kept.
3. Section 5.2 uses `M0?(30|2)\b`, the reference additionally uses `(?!\d)`. The reference wins.

### Golden test pitfalls (Rust must match Python exactly)
4. Python `splitlines()` also splits at `\v`, `\f`, `\x1c–\x1e`, `\x85`, ` /29`. Rebuilt in Rust.
5. Number format `_fmt` (`{:.4f}`, trailing zeros removed, `-0.` → `0.`) is rebuilt and tested specifically.
6. Invalid UTF-8 bytes become `U+FFFD` and are written that way (like the reference).
7. The time stamp line is ignored in the golden comparison; the Rust core gets the time stamp as a parameter.

### Functional gaps
8. **"Time saved" (F7)** needs a rapid speed; the reference only reports "time previously at feed rate". Until M4 the result shows "previously approx. X min at feed rate", from M4 on the real saving.
9. **Inventory vs. automatic numbers (F4):** what is the "next free number" if the inventory uses T13? Proposal: apply the inventory first, unknown tools get the smallest number used neither in the inventory nor in the job. Matching by description (whitespace normalised, case-insensitive).
10. **Several tools per file (5.4)** do not fit "one T number per file" in the GUI. Proposal: one row with − / + per original tool.
11. **− / + for 1–99** means up to 98 clicks. Proposal: additionally a number grid (clicking the number opens a 10×10 grid), one click is enough.
12. **Same tool twice in a row:** the reference still writes `Tn M6` (Makera asks again and measures again). Kept that way.
13. **Natural sorting (F1):** applies to the newly added group; it is appended to the end of the existing list.
14. **Window size (F9)** is better handled with `tauri-plugin-window-state` instead of `store`.

### Technical risks
15. **Drag & drop on Windows:** when Tauri's file drop is active, HTML5 drag & drop inside the window does not work. Reordering therefore uses **dnd-kit** (pointer events), which works alongside it.
16. **Large files:** loading/merging runs in a Rust thread with progress via Tauri events; the frontend only gets header data, never the million lines.
17. **Signing:** self-signed certificates are out of the question. Options: unsigned (warning during installation) or a paid certificate.

### Organisation
18. The requirements and the Git repository of the old version were in different folders. Which folder becomes the new project?

## 3. UI library

**Tailwind CSS 4 + shadcn/ui (Radix) + lucide-react + dnd-kit.** Radix provides accessible dialogs, switches and menus; shadcn components live as source code in the project (no heavy dependency, small installer). Alternatives: Mantine (more ready-made, but larger and with its own style), Fluent UI (Windows look, foreign on macOS/Linux).

## 4. Logo proposal

Rounded square with three small circles (tools) side by side, connected by an arrow to one large circle on the right – "many into one". Single colour, works as a 16 px icon.

## 5. Implementation plan

| Milestone | Content | Acceptance |
|---|---|---|
| **M1 Core** | Cargo workspace: `multipass-core` (parser, tool assignment, merge, rapids), `multipass-cli`. Generate and commit golden files from the reference. Safety test, large file test (1 M lines < 3 s) | `cargo test` green, golden identical |
| **M2 MVP** | Tauri 2 + React/TS/Vite, loading via dialog and drag & drop, list, reordering (buttons + dragging), T numbers, rapid option with warning, checks, saving with progress | merge the fixtures with the mouse |
| **M3 Comfort** | Inventory, settings, sequence preview, result dialog with Makera notes and "Open folder", dark/light | Angel Token files, test in Makera Studio |
| **M4 Extension** | Run time estimate, comparison with/without rapids; optional 2D preview | values plausible against the reference (94 min) |
| **M5 Delivery** | GitHub Actions with `tauri-action`, installers for Win/macOS/Linux, README, quick guide | installers start on all three systems |

Project layout:
```
MultiPass/
  crates/multipass-core/   core logic + tests + golden
  crates/multipass-cli/
  src-tauri/               Tauri app (uses multipass-core)
  src/                     React frontend
  reference/               Python version (kept)
  tests/fixtures/
```

## 6. Decisions by the user (2026-11-19)

| Question | Decision |
|---|---|
| Project folder | new Git repository |
| GitHub | yes, builds via GitHub Actions (made public on 2026-11-20) |
| Signing | **unsigned** |
| Inventory | **pre-filled** with #13, #10, #9, #16, #21 |
| Updates | **no** auto-updater, a new installer is enough |
| Language | **German + English** with a switch (English by default since 2026-11-22) |
| 2D preview | yes, in **M4** |
| Python version | only kept as a **reference** in `reference/` |
| Z origin ≠ stock top | warning, **rapids blocked for this run** |
| T number | − / + **and number grid** 1–99 |
| Colours | **blue/teal**, calm |
| Logo | three small circles → arrow → large circle |

## 7. Deliberate differences from the Python reference

Found by random testing (fuzzing) of the Rust core and later reviews. Nothing changes for the
real Fusion files: the Angel Token files produce the same output as the reference.

| Case | Reference | Rust core |
|---|---|---|
| Added F word on a line with a `;` comment | F ends up **inside the comment**, the machine cuts with the wrong feed | F is inserted before the `;` |
| Converted line had `F0` | F is lost | F is added again |
| Coordinate with more than 4 decimals in a converted line | rounded to 4 decimals, position changes minimally | exact value is kept |
| Z origin not "Stock Top" or unknown | rapids are converted anyway | warning, rapids off for this run |
| Non-ASCII characters in G-code (outside comments) | processed | rapids off for this part, warning |
| Unreadable number in the file header / non-ASCII digits in T numbers | crash or processed | file is rejected with a message |
| Control characters (line breaks) in file names or time stamp | written into the comment line | replaced by `_`, so no extra G-code lines can appear |
| Comment lines written by MultiPass | German (`Zusammengefuegt mit`, `Reihenfolge`, `Werkzeuge`, `Teil`) | English (`Merged with`, `Order`, `Tools`, `Part`); old German files are still recognised as merged |
