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
- Simulation mit Materialabtrag (Draufsicht), Abspielen mit Zeitleiste und Halt beim Werkzeugwechsel
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

## Kommandozeile

Für Skripte gibt es `multipass-cli`. Es nutzt denselben Kern wie die Oberfläche und
erzeugt bis auf den Zeitstempel identische Dateien. Im Release liegt es als
`multipass-cli-windows-x64.exe`, `-macos-arm64`, `-macos-x64` und `-linux-x64`.

```
multipass-cli *.cnc --sortieren --rapid
multipass-cli 1_Schruppen.cnc 2_Gravur.cnc -t 2=16 -o Teil.cnc --nur-pruefen
```

| Option | Wirkung |
|---|---|
| `-o DATEI` | Zieldatei, Standard `GESAMT.<Endung>` im Ordner der ersten Datei |
| `-f` | vorhandene Zieldatei ersetzen (sonst Abbruch; Eingabedateien werden nie überschrieben) |
| `-s` | nach Namen sortieren wie in der Oberfläche |
| `-n` | nur prüfen: Ablauf, Hinweise und Laufzeit, nichts schreiben |
| `-t N=T`, `-t N:A=T` | Werkzeugnummer für Teil N setzen |
| `--inventar DATEI`, `--ohne-inventar` | anderes oder kein Inventar |
| `--rapid`, `--safe-z MM` | Eilgang-Wiederherstellung (standardmäßig aus) |
| `--eilgang-tempo MM` | Eilgang der Maschine für die Schätzung |

Ohne Angabe gelten Inventar, Sicherheitshöhe und Eilgang-Tempo aus den Einstellungen der
Oberfläche. Dateien, die schon mit MultiPass zusammengefügt wurden, werden übersprungen.
Rückgabewert: 0 fertig, 1 Fehler, 2 falscher Aufruf.

## Entwickeln

Voraussetzungen: Rust (stable), Node.js 22, auf Linux zusätzlich
`libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf`.

```
npm ci
npx tauri dev                      # App im Entwicklungsmodus
npm run dev                        # nur Oberfläche im Browser (mit Backend-Attrappe)
npx tauri build                    # Installer für das eigene System
```

Kommandozeile ohne Oberfläche (`multipass-cli --hilfe` zeigt alle Optionen):

```
cargo run -p multipass-cli -- a.cnc b.cnc -o GESAMT.cnc --rapid --safe-z 1.0
```

### Aufbau

| Pfad | Inhalt |
|---|---|
| `crates/multipass-core` | Kernlogik in Rust: Einlesen, Werkzeugzuordnung, Zusammenfügen, Eilgang, Laufzeit, Vorschau, Simulation |
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
