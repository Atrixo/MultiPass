# MultiPass – Hinweise für Claude Code

**Lies zuerst `ANFORDERUNGEN.md`.** Dort stehen Auftrag, Anforderungen, Regeln und offene Fragen.
Der Auftrag beginnt mit einer **Anforderungsanalyse**. Gebaut wird erst nach Freigabe durch den Nutzer.

## Kurzfassung
- MultiPass fügt NC-Programme aus Fusion 360 Personal (je ein Werkzeug pro Datei, Postprozessor
  `Carvera.cps`, Maschine Makera Z1) zu einem Programm mit `Tn M6`-Werkzeugwechseln zusammen.
  Optional werden die von Fusion Personal unterdrückten Eilgänge wiederhergestellt.
- **Neuer Stack:** Tauri 2 + React + TypeScript (Vite). Die Kernlogik kommt in Rust.
- **Fachliche Referenz:** `reference/multipass.py` (Python/Tkinter, funktioniert, mit echten Dateien
  geprüft). Die Oberfläche wird ersetzt, die **Logik muss exakt gleich bleiben**.

## Der Nutzer
- Marcel, spricht Deutsch. Oberfläche, Meldungen und Doku bitte auf Deutsch.
- **Bedient den PC ausschließlich mit der Maus, er hat keine Tastatur.** Keine Tastenkürzel, keine
  Pflicht-Texteingaben, große Klickflächen, − / +-Knöpfe statt Zahlenfeldern, Datei-Dialoge und
  Drag & Drop statt Pfadeingabe.
- Hauptsystem Windows. Das Programm muss aber auch auf macOS und Linux laufen.

## Projektstruktur (aktuell)
| Pfad | Inhalt |
|---|---|
| `ANFORDERUNGEN.md` | vollständige Anforderungen für die Neuentwicklung |
| `reference/multipass.py` | Python-Referenz (Kernlogik + alte Tkinter-Oberfläche); CLI: `python reference/multipass.py --cli a.cnc b.cnc -o out.cnc [--rapid]` |
| `reference/test_multipass.py` | Tests der Referenz: `python reference/test_multipass.py` |
| `reference/*.bat/.command/.sh` | Starter der alten Version |
| `tests/fixtures/*.cnc` | echte, kleine NC-Dateien (Fusion + Carvera.cps, Makera Z1) für Tests |
| `tests/fixtures/edge/*.nc` | synthetische Grenzfälle (mehrere Werkzeuge, Umlaute, G28/G53, modale Zeilen) |
| `tests/golden/` | Ausgaben der Python-Referenz; neu erzeugen mit `python tests/golden/erzeugen.py` |
| `PHASE1_ANALYSE.md` | Analyse, Entscheidungen des Nutzers, bewusste Abweichungen von der Referenz |
| `crates/multipass-core` | Rust-Kern: Einlesen, Werkzeugzuordnung, Zusammenfügen, Eilgang, Laufzeit und Bahnvorschau (`motion.rs`) |
| `crates/multipass-cli` | `cargo run -p multipass-cli -- a.cnc b.cnc -o GESAMT.cnc [--rapid] [--safe-z 1.0]` |
| `src-tauri/` | Tauri-App: Befehle `load_files`, `auto_tools`, `prepare_merge`, `save_merge`, `estimate`, `preview` (Logik in `state.rs`, IPC-Tests in `ipc_tests.rs`) |
| `src/` | React-Oberfläche; `lib/state.ts` (Liste, T-Nummern), `lib/i18n.tsx` (DE/EN), `components/` |
| Einstellungen | `%APPDATA%\de.multipass.desktop\settings.json` (Sprache, Darstellung, Eilgang, safe_z, letzter Ordner, Inventar); Fenstergröße in `.window-state.json` |
| `src/dev/mock.ts` | Backend-Attrappe, nur im Browser mit `npm run dev` aktiv (Oberfläche ohne Tauri prüfen) |

## Starten und bauen
- `npx tauri dev` – App im Entwicklungsmodus
- `npx tauri build --bundles nsis` – Windows-Installer unter `target/release/bundle/nsis/`

## Tests
- `npm test` – Frontend-Logik (vitest)
- `cargo test` – alles außer echten Dateien
- `cargo test --release` – zusätzlich Zeitvorgabe 1 Mio. Zeilen < 3 s
- `MULTIPASS_FUZZ=100000 cargo test --release --test safety` – mehr Zufallsprogramme
- `MULTIPASS_REAL_DIR=<Ordner mit .cnc> cargo test --release --test echte_dateien -- --ignored` – Vergleich mit Python an echten Dateien
- `MULTIPASS_PREVIEW_OUT=<Projekt>/target/dev-preview.json` zusätzlich setzen: schreibt Vorschau-Daten der echten Dateien; `npm run dev` zeigt sie dann im Browser

## Unverhandelbare Regeln
1. **Fräsbahnen dürfen sich nie unbemerkt ändern.** Jede Änderung an Parser, Merge oder
   Eilgang-Logik braucht Tests. Golden-Tests gegen die Python-Referenz müssen grün sein.
2. Die Eilgang-Umwandlung folgt exakt Abschnitt 6 der Anforderungen und bleibt standardmäßig **aus**.
3. Von MultiPass erzeugte Kommentare im G-Code sind **nur ASCII**.
4. Keine Netzwerkzugriffe, keine Telemetrie.
5. Alles muss **ohne Tastatur** bedienbar sein.
