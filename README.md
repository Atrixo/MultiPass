# MultiPass

Fügt mehrere NC-Programme zu **einem Programm mit Werkzeugwechseln** zusammen.

Gedacht für **Fusion 360 Personal** mit dem Postprozessor **`Carvera.cps`** und die **Makera Z1**.
Fusion Personal gibt nur ein Werkzeug pro Datei aus und schreibt alle Eilgänge als langsame
Vorschubbewegungen. MultiPass behebt beides, ohne die Fräsbahnen zu verändern.

- Dateien per Drag & Drop oder Datei-Dialog, natürliche Sortierung
- Reihenfolge per Knopf oder Ziehen, Werkzeugnummern automatisch, per − / + oder Zahlenfeld
- Werkzeug-Inventar mit festen Nummern (z. B. T13 = Spiral O Metal 3.175×12)
- Optional: Leerfahrten über dem Material wieder als Eilgang (G0)
- Laufzeitschätzung ohne/mit Eilgang, 2D-Bahnvorschau
- Deutsch/Englisch, Hell/Dunkel, komplett mit der Maus bedienbar, keine Netzwerkzugriffe

Bedienung: siehe **[KURZANLEITUNG.md](KURZANLEITUNG.md)**.

## Installieren

Installer gibt es unter **Releases** (Windows, macOS, Linux). Sie sind **nicht signiert**.

| System | Datei | Hinweis beim ersten Start |
|---|---|---|
| Windows 10/11 | `MultiPass_<Version>_x64-setup.exe` | „Der Computer wurde durch Windows geschützt“ → **Weitere Informationen** → **Trotzdem ausführen**. Keine Administratorrechte nötig. |
| macOS (Apple Silicon) | `MultiPass_<Version>_aarch64.dmg` | MultiPass in „Programme“ ziehen, beim ersten Start **Rechtsklick → Öffnen**. |
| macOS (Intel) | `MultiPass_<Version>_x64.dmg` | wie oben |
| Linux | `.AppImage` oder `.deb` | AppImage: Rechtsklick → Eigenschaften → „Als Programm ausführen“ erlauben. |

Einstellungen und Inventar liegen unter Windows in `%APPDATA%\de.multipass.desktop\`.

## Sicherheit der Fräsbahnen

- Ohne Eilgang-Option werden nur Werkzeugnummern umgeschrieben und die Teile aneinandergehängt.
- Mit Eilgang-Option wird eine G1-Bewegung nur zu G0, wenn Start **und** Ziel über der Sicherheitshöhe
  liegen oder es ein reiner Rückzug nach oben ist. Bögen und Bewegungen im Material bleiben unverändert.
- Spannpratzen kennt das Programm nicht. Die Option ist deshalb standardmäßig aus.
- Die Ausgabe ist byte-gleich mit der geprüften Python-Version (`reference/`). Bewusste Abweichungen
  (Sicherheitsverbesserungen) stehen in [PHASE1_ANALYSE.md](PHASE1_ANALYSE.md), Abschnitt 7.

## Entwickeln

Voraussetzungen: Rust (stable), Node.js 22, auf Linux zusätzlich
`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

```
npm ci
npx tauri dev                      # App im Entwicklungsmodus
npm run dev                        # nur Oberfläche im Browser (mit Backend-Attrappe)
npx tauri build                    # Installer für das eigene System
```

Kommandozeile ohne Oberfläche:

```
cargo run -p multipass-cli -- a.cnc b.cnc -o GESAMT.cnc --rapid --safe-z 1.0
```

### Aufbau

| Pfad | Inhalt |
|---|---|
| `crates/multipass-core` | Kernlogik in Rust: Einlesen, Werkzeugzuordnung, Zusammenfügen, Eilgang, Laufzeit, Vorschau |
| `crates/multipass-cli` | Kommandozeile |
| `src-tauri` | Tauri-App (Befehle für die Oberfläche) |
| `src` | Oberfläche: React, TypeScript, Tailwind |
| `reference` | Python-Referenz (alte Version), Grundlage der Golden-Tests |
| `tests/fixtures`, `tests/golden` | Testdateien und erwartete Ausgaben der Referenz |

### Tests

```
cargo test --release               # Unit-, Golden-, Sicherheits-, Großdatei- und IPC-Tests
npm test                           # Frontend-Logik
python reference/test_multipass.py # Python-Referenz
```

Weitere Varianten (Zufallstests, Vergleich mit echten Dateien) stehen in [CLAUDE.md](CLAUDE.md).
Jede Änderung an Einlesen, Zusammenfügen oder Eilgang braucht Tests; die Golden-Tests müssen grün bleiben.

### Neue Version veröffentlichen

1. Version in `Cargo.toml`, `package.json` und `src-tauri/tauri.conf.json` erhöhen.
2. Tag setzen und pushen: `git tag v2.0.1 && git push origin v2.0.1`.
3. GitHub Actions baut die Installer und legt einen **Entwurf** unter Releases an. Prüfen, dann veröffentlichen.
