# MultiPass

Merges several NC programs into **one program with tool changes**.

Made for **Fusion 360 Personal** with the **`Carvera.cps`** post processor and the **Makera Z1**.
Fusion Personal only outputs one tool per file and writes all rapids as slow feed moves.
MultiPass fixes both without changing the toolpaths.

- Files via drag & drop or file dialog, natural sorting
- Order by button or dragging, automatic tool numbers, adjustable with − / + or a number pad
- Tool inventory with fixed numbers (e.g. T13 = Spiral O Metal 3.175×12)
- Optional: air moves above the stock become rapids (G0) again
- Optional: spindle fan off during tool changes
- Run time estimate with and without rapids, 2D toolpath preview
- Material removal simulation (top view), playback with timeline and pause at tool changes
- English/German, light/dark, fully usable with the mouse only, no network access

How to use it: see **[GUIDE.md](GUIDE.md)** (German: [KURZANLEITUNG.md](KURZANLEITUNG.md)).

## Install

Installers are available under **Releases** (Windows, macOS, Linux). They are **not signed**.

| System | File | Note on first launch |
|---|---|---|
| Windows 10/11 | `MultiPass_<version>_x64-setup.exe` | "Windows protected your PC" → **More info** → **Run anyway**. No admin rights needed. |
| macOS (Apple Silicon) | `MultiPass_<version>_aarch64.dmg` | Drag MultiPass into "Applications", on first launch **right-click → Open**. |
| macOS (Intel) | `MultiPass_<version>_x64.dmg` | same as above |
| Linux | `.AppImage` or `.deb` | AppImage: right-click → Properties → allow "Run as program". |

On Windows, settings and inventory are stored in `%APPDATA%\de.multipass.desktop\`.

## Toolpath safety

- Without the rapid option, only tool numbers are rewritten and the parts are appended.
- With the rapid option, a G1 move only becomes G0 if start **and** end are above the safe height,
  or if it is a pure retract upwards. Arcs and moves inside the material stay unchanged.
- The program does not know about clamps, so the option is off by default.
- The output matches the verified Python version (`reference/`) byte for byte, except for the
  comment lines MultiPass writes itself, which are now in English. Deliberate differences
  (safety improvements) are listed in [PHASE1_ANALYSIS.md](PHASE1_ANALYSIS.md), section 7.
- The fan option only adds `M812` before and `M811 S100` after each `Tn M6`; nothing else changes.

## Command line

`multipass-cli` is meant for scripts. It uses the same core as the GUI and produces identical
files apart from the time stamp. Releases contain it as `multipass-cli-windows-x64.exe`,
`-macos-arm64`, `-macos-x64` and `-linux-x64`.

```
multipass-cli *.cnc --sort --rapid
multipass-cli 1_Roughing.cnc 2_Engrave.cnc -t 2=16 -o Part.cnc --dry-run
```

| Option | Effect |
|---|---|
| `-o FILE` | output file, default `MERGED.<ext>` in the folder of the first file |
| `-f` | replace an existing output file (otherwise abort; input files are never overwritten) |
| `-s` | sort by name like the GUI |
| `-n` | dry run: order, notes and run time, nothing written |
| `-t N=T`, `-t N:A=T` | set the tool number for part N |
| `--inventory FILE`, `--no-inventory` | other or no inventory |
| `--rapid`, `--safe-z MM` | rapid restoration (off by default) |
| `--rapid-speed MM` | machine rapid speed for the estimate |
| `--fan-pause` | spindle fan off during tool changes |

Without these options, inventory, safe height and rapid speed come from the GUI settings.
Files that were already merged by MultiPass are skipped. The German option names of
earlier versions (`--ausgabe`, `--werkzeug`, ...) still work.
Exit code: 0 done, 1 error, 2 wrong usage.

## Development

Requirements: Rust (stable), Node.js 22, on Linux also
`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

```
npm ci
npx tauri dev                      # app in development mode
npm run dev                        # GUI only, in the browser (with a mock backend)
npx tauri build                    # installer for your own system
```

Command line without GUI (`multipass-cli --help` lists all options):

```
cargo run -p multipass-cli -- a.cnc b.cnc -o MERGED.cnc --rapid --safe-z 1.0
```

### Layout

| Path | Content |
|---|---|
| `crates/multipass-core` | core logic in Rust: parsing, tool assignment, merging, rapids, run time, preview, simulation |
| `crates/multipass-cli` | command line |
| `src-tauri` | Tauri app (commands for the GUI) |
| `src` | GUI: React, TypeScript, Tailwind |
| `reference` | Python reference (old version), basis of the golden tests |
| `tests/fixtures`, `tests/golden` | test files and expected output of the reference |

### Tests

```
cargo test --release               # unit, golden, safety, large file and IPC tests
npm test                           # frontend logic
python reference/test_multipass.py # Python reference
```

More variants (random tests, comparison with real files) are listed in [CLAUDE.md](CLAUDE.md).
Every change to parsing, merging or rapids needs tests; the golden tests must stay green.

### Publishing a new version

1. Bump the version in `Cargo.toml`, `package.json` and `src-tauri/tauri.conf.json`.
2. Create and push a tag: `git tag v2.2.0 && git push origin v2.2.0`.
3. GitHub Actions builds the installers and creates a **draft** under Releases. Check it, then publish.

## License

MIT, see [LICENSE](LICENSE).
