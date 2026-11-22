# MultiPass – Notes for Claude Code

**Read `REQUIREMENTS.md` first.** It contains the brief, requirements, rules and open questions.
Decisions and deliberate differences from the reference are in `PHASE1_ANALYSIS.md`.

## Summary
- MultiPass merges NC programs from Fusion 360 Personal (one tool per file, post processor
  `Carvera.cps`, machine Makera Z1) into one program with `Tn M6` tool changes.
  Optionally it restores the rapids that Fusion Personal suppresses.
- **Stack:** Tauri 2 + React + TypeScript (Vite). The core logic is in Rust.
- **Functional reference:** `reference/multipass.py` (old Python/Tkinter version, verified with real
  files). The **logic must stay exactly the same**.

## The user
- Speaks German. **The repository is in English** (code, identifiers, tests, docs, commit messages,
  CLI output, G-code comments). The GUI is bilingual and starts in English; German stays available.
- **Uses the PC with the mouse only, there is no keyboard.** No shortcuts, no mandatory text input,
  large click targets, − / + buttons instead of number fields, file dialogs and drag & drop instead
  of typed paths.
- Main system Windows. The program must also run on macOS and Linux.

## Project layout
| Path | Content |
|---|---|
| `REQUIREMENTS.md` | full requirements of the rewrite |
| `PHASE1_ANALYSIS.md` | analysis, user decisions, deliberate differences from the reference |
| `reference/multipass.py` | Python reference (core logic + old Tkinter GUI, German); CLI: `python reference/multipass.py --cli a.cnc b.cnc -o out.cnc [--rapid]` |
| `reference/test_multipass.py` | reference tests: `python reference/test_multipass.py` |
| `tests/fixtures/*.cnc` | real, small NC files (Fusion + Carvera.cps, Makera Z1, German Fusion install) |
| `tests/fixtures/edge/*.nc` | synthetic edge cases (several tools, umlauts, G28/G53, modal lines) |
| `tests/golden/` | output of the Python reference; regenerate with `python tests/golden/generate.py`. The reference writes German comment lines; the tests translate them with `english_comments` before comparing |
| `crates/multipass-core` | Rust core: parsing, tool assignment, merging (incl. fan option), rapids, run time, toolpath preview (`motion.rs`) and simulation timeline (`sim.rs`: vertices, duration, tool shape from D/TD/CR/TAPER) |
| `crates/multipass-cli` | `cargo run -p multipass-cli -- --help`; reads the app settings (`MULTIPASS_SETTINGS` overrides the path), tests in `tests/cli.rs`; German option names stay as aliases |
| `src-tauri/` | Tauri app: commands `load_files`, `auto_tools`, `prepare_merge`, `save_merge`, `estimate`, `preview`, `timeline` (binary response: JSON header + f32 data; logic in `state.rs`, IPC tests in `ipc_tests.rs`) |
| `src/` | React GUI; `lib/state.ts` (list, T numbers), `lib/i18n.tsx` (EN/DE), `lib/sim.ts` + `lib/sim.worker.ts` (material removal as a height map in a web worker), `components/` |
| Settings | `%APPDATA%\de.multipass.desktop\settings.json` (language, theme, rapids, safe_z, fan option, last folder, inventory); window size in `.window-state.json` |
| `src/dev/mock.ts` | mock backend, only active in the browser with `npm run dev` (check the GUI without Tauri) |

## Run and build
- `npx tauri dev` – app in development mode
- `npx tauri build --bundles nsis` – Windows installer in `target/release/bundle/nsis/`

## Tests
- `npm test` – frontend logic (vitest)
- `cargo test` – everything except real files
- `cargo test --release` – additionally the time limit of 1 M lines < 3 s
- `MULTIPASS_FUZZ=100000 cargo test --release --test safety` – more random programs
- `MULTIPASS_REAL_DIR=<folder with .cnc> cargo test --release --test real_files -- --ignored` – comparison with Python on real files
- additionally set `MULTIPASS_PREVIEW_OUT=<project>/target/dev-preview.json`: writes preview data of the real files; `npm run dev` then shows them in the browser
- `MULTIPASS_TIMELINE_OUT=<project>/target/dev-timeline.bin` with `cargo test --release --test real_files simulation -- --ignored`: simulation data for `npm run dev`

## Non-negotiable rules
1. **Toolpaths must never change unnoticed.** Every change to parser, merge or rapid logic needs
   tests. Golden tests against the Python reference must be green.
2. Rapid conversion follows section 6 of the requirements exactly and stays **off** by default.
   The fan option is also off by default and only adds `M812` / `M811 S100` around `Tn M6`.
3. Comments that MultiPass writes into G-code are **ASCII only**; control characters become `_`.
4. No network access, no telemetry.
5. Everything must be usable **without a keyboard**.
