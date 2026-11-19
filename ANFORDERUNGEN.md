# MultiPass – Anforderungen für die Neuentwicklung mit Tauri 2 + React

> **Auftrag an Claude Code**
> 1. **Phase 1 – Anforderungsanalyse:** Lies dieses Dokument, `CLAUDE.md` und die Referenz-
>    implementierung `reference/multipass.py` samt Tests. Prüfe die Anforderungen auf Lücken
>    und Widersprüche. Stelle dem Nutzer die **offenen Fragen (Abschnitt 12)** und schlage
>    einen Umsetzungsplan mit Meilensteinen vor. **Erst nach seiner Freigabe bauen.**
> 2. **Phase 2 – Umsetzung** nach freigegebenem Plan (MVP zuerst, dann Ausbaustufen).
> 3. **Phase 3 – Prüfung:** Tests, Installer bauen, kurze Anleitung (Deutsch).

---

## 1. Ziel

MultiPass fügt mehrere NC-Programme (G-Code) zu **einem Programm mit Werkzeugwechseln**
zusammen.

**Hintergrund:** Der Nutzer erzeugt Fräsprogramme mit **Fusion 360 Personal** für eine
**Makera Z1** (Postprozessor `Carvera.cps`). Fusion Personal hat zwei Einschränkungen:

1. **Nur ein Werkzeug pro ausgegebener Datei.** Bei einem Teil mit 4 Fräsern entstehen
   5 Dateien, die einzeln nacheinander in Makera Studio geladen werden müssen.
2. **Eilgänge (G0) werden als Vorschubbewegungen (G1) ausgegeben.** Das verlängert die Laufzeit
   stark. Beim Projekt „Angel-Token“ kostete das rund 1,5 Stunden.

MultiPass behebt beides: eine Datei mit `Tn M6`-Wechseln und optional wiederhergestellten
Eilgängen. Dabei dürfen sich die Fräsbahnen **nie** verändern.

Es gibt bereits eine funktionierende **Python/Tkinter-Version** (`reference/multipass.py`).
Ihre Logik ist mit echten Dateien geprüft und gilt als **fachliche Referenz**. Ihre Oberfläche
gefällt dem Nutzer nicht („altbacken“), deshalb die Neuentwicklung.

## 2. Nutzer und Rahmenbedingungen

- **Sprache:** Deutsch (Oberfläche, Meldungen, Anleitung).
- **Bedienung ausschließlich mit der Maus. Der Nutzer hat keine Tastatur.**
  - Jede Funktion muss per Klick erreichbar sein, ohne Tastenkürzel und ohne Pflicht-Texteingabe.
  - Zahlenwerte über große **− / +**-Knöpfe oder Auswahllisten statt Textfeldern.
  - Dateien über **Datei-Dialog** und **Drag & Drop**, nie über getippte Pfade.
  - Große Klickflächen (mindestens 44 × 44 px), klare Zustände, keine Hover-only-Funktionen.
  - Kein Doppelklick erforderlich. Ein Doppelklick darf zusätzlich angeboten werden.
- **Plattformen:** Windows (Hauptsystem), macOS, Linux.
- **Offline**, keine Telemetrie, keine Cloud.
- Auf dem Windows-PC sind Rust (`.cargo`, `.rustup`) und vermutlich Node.js vorhanden.
  Bitte in Phase 1 prüfen bzw. erfragen.

## 3. Technischer Stack

| Bereich | Vorgabe |
|---|---|
| Desktop-Rahmen | **Tauri 2** |
| Frontend | **React + TypeScript + Vite** |
| UI | modernes Komponenten-System, Vorschlag: **Tailwind CSS + shadcn/ui** (Alternativen in Phase 1 begründen); Icons z. B. lucide-react |
| Kernlogik | **Rust** (im Tauri-Backend): Einlesen, Zusammenfügen und Eilgang-Umwandlung. Grund: Dateien sind bis > 10 MB / ~1 Mio. Zeilen groß |
| Tauri-Plugins | `dialog` (Öffnen/Speichern), `fs`, `store` (Einstellungen), ggf. `opener` (Ordner öffnen) |
| Tests | Rust-Unit-Tests + Golden-Tests gegen die Python-Referenz; Frontend-Tests nach Bedarf |
| Build | Installer für Windows (NSIS `.exe` oder `.msi`), macOS (`.dmg`), Linux (`.AppImage`/`.deb`). Am besten per GitHub Actions (`tauri-action`) |

Die Kernlogik soll als eigenes Rust-Modul bzw. eigene Crate (`multipass-core`) ohne GUI-Abhängigkeit
gebaut werden. Optional gibt es dazu eine kleine CLI für Tests (`multipass-cli a.cnc b.cnc -o gesamt.cnc --rapid`).

## 4. Funktionale Anforderungen

### F1 – Dateien laden
- Über den Knopf „Dateien hinzufügen“ (Mehrfachauswahl; Filter: `.cnc .nc .gcode .ngc .tap .txt`, „Alle Dateien“).
- Per **Drag & Drop** ins Fenster, auch mehrere Dateien auf einmal.
- Optional: Dateien beim Programmstart als Argumente übergeben („Öffnen mit“ / auf das Programmsymbol ziehen).
- Mehrere Dateien werden **natürlich sortiert** (`1_…`, `2_…`, `10_…`).
- Nicht lesbare Dateien führen zu einer verständlichen Meldung. Die übrigen werden trotzdem geladen.

### F2 – Dateiliste anzeigen
Pro Datei als Karte oder Zeile:
- Position (1, 2, 3 …), Dateiname
- **Werkzeug** aus dem Dateikopf, z. B. „Spiral O Metal 3.175*12mm Makera“
- zugewiesene **Werkzeugnummer** (T-Nr.)
- **tiefster Punkt** (`Toolpath Z Min from Stock Top`)
- optional: Anzahl Zeilen bzw. Bewegungen, geschätzte Laufzeit (siehe F8)

### F3 – Reihenfolge ändern
- Große Knöpfe **„Nach oben“ / „Nach unten“** und **„Entfernen“** für die ausgewählte Datei.
- Zusätzlich **Drag & Drop zum Umsortieren** (mit sichtbarem Griff).
- „Alle entfernen“ mit Bestätigung.

### F4 – Werkzeugnummern
- **Automatik:** Gleiche Werkzeugbeschreibung bekommt dieselbe Nummer, neue Werkzeuge die nächste
  freie Nummer (T1, T2, …). Siehe `default_tool_map`.
- **Manuell:** Nummer pro Datei über − / + (1–99) ändern. Knopf „Nummern automatisch“ setzt zurück.
- **Inventar-Zuordnung (wichtig für den Nutzer):** Zuordnung „Werkzeugbeschreibung → Nummer“
  dauerhaft speichern, damit z. B. der 3,175er Metallfräser immer **T13** heißt (Inventarnummer #13).
  Gespeicherte Zuordnungen werden beim Laden automatisch angewendet. Eine kleine Verwaltungsansicht
  zeigt die Zuordnungen und erlaubt das Löschen.
  Bekanntes Inventar des Nutzers: #13 = Spiral O Metal 3.175×12, #10 = 1.5×6, #9 = 1×3,
  #16 = Gravierstichel Metal 30°×0.3, #21 = Fase 90°.

### F5 – Ablaufvorschau
Gut sichtbare Übersicht „Ablauf an der Maschine“:
`→ Werkzeugwechsel auf T13 (Spiral O Metal 3.175×12) → 1. 1_T13_Schruppen.cnc → Werkzeugwechsel auf T10 …`

### F6 – Option „Leerfahrten als Eilgang“
- Schalter, **standardmäßig aus**.
- Einstellbare Sicherheitshöhe `safe_z` über − / + (0,5–20 mm, Standard 1,0 mm, Schritt 0,5).
- Deutlicher Warnhinweis: „Spannpratzen, die höher sind als dieser Wert, kennt das Programm nicht.“
- Regeln exakt wie in Abschnitt 6.

### F7 – Zusammenfügen und speichern
- Großer Haupt-Knopf „Zusammenfügen und speichern“.
- Speichern-Dialog, vorgeschlagen werden der Ordner der ersten Datei und der Name `GESAMT.cnc`
  (bzw. die Endung der Eingabedateien).
- Vorher Prüfungen (siehe 5.6). Fehler blockieren, Warnungen werden angezeigt und müssen bestätigt werden.
- Ergebnis-Dialog: Speicherort, Reihenfolge mit T-Nummern und Anzahl der umgewandelten
  Leerfahrten samt Strecke und gesparter Zeit. Dazu ein Knopf „Ordner öffnen“.
- **Hinweis-Box im Ergebnis** zur Makera Z1 (kurz):
  - Makera Studio fragt bei jedem Wechsel „Please change to Tool n“, dann den passenden Fräser
    einsetzen und **Confirm** klicken.
  - **Auto-Z-Antasten nur auf unbearbeiteter Materialfläche**, nicht im Fräsbereich, denn sonst
    stimmt Z nicht.

### F8 – Laufzeitschätzung (Ausbaustufe)
- Pro Datei und gesamt: Strecke / Vorschub. Für Eilgänge gibt es eine einstellbare Eilgang-Geschwindigkeit.
- Vergleich „ohne / mit Eilgang-Option“.

### F9 – Einstellungen merken
Letzter Ordner, Eilgang-Option, `safe_z`, Inventar-Zuordnungen und Fenstergröße. Gespeichert wird mit `tauri-plugin-store`.

### F10 – Bahnvorschau (Ausbaustufe, optional)
2D-Draufsicht (XY) der Bahnen, farbig je Werkzeug. Eilgänge gestrichelt. Gedacht zur Kontrolle,
nicht als Simulator.

## 5. Zusammenfüge-Regeln (fachlich verbindlich)

Referenz: `merge()` in `reference/multipass.py`.

### 5.1 Aufbau einer Eingabedatei (Carvera.cps, Makera Z1)
```
(Programmname)
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

(Operationsname)
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

### 5.2 Erkennung
- **Werkzeugwechsel:** Zeile `^\s*T\s*(\d+)\s*M0?6\b` (Groß/Klein egal).
- **Körperbeginn:** erster Werkzeugwechsel. Steht direkt davor eine Kommentarzeile (Operationsname), gehört sie dazu.
- **Programmende:** letzte Zeile `^\s*M0?(30|2)\b`. Der Körper endet **vor** dieser Zeile.
- **Vorspann** = alles vor dem Körper, **Epilog** = ab `M30` bis Dateiende.
- **Kopfinfos:** Werkzeugzeile `(T<n>  <Beschreibung>  D=…)`, Z-Min, Z-Nullpunkt, X/Y-Offset,
  Maschinenmodell, Einheiten (`G20` = Zoll, sonst mm), inkrementell (`G91`).

### 5.3 Ausgabe
1. Kommentarblock: „Zusammengefuegt mit MultiPass <Version> am <Datum>“ und die Reihenfolge mit T-Nummern.
2. **Vorspann der ersten Datei.** Deren eigene Werkzeug-Kommentarzeile entfällt.
3. Kommentar-Werkzeugliste `(T<n>  <Beschreibung>)`, sortiert nach Nummer.
4. Pro Datei: `(===== Teil i: <Dateiname> =====)`, dann der Körper. Dabei wird `T<alt>M6` durch `T<neu>M6` ersetzt.
   `M5 / M400 / M852 / G28` am Teilende bleiben erhalten.
5. Am Schluss der **Epilog der letzten Datei** (`M30` …). Es gibt genau ein `M30`.
6. **Zeilenenden** wie in der ersten Datei (CRLF/LF beibehalten).
7. **Alle von MultiPass erzeugten Kommentare nur ASCII.** Umlaute werden umgeschrieben (ä→ae …),
   `( )` in Texten wird zu `[ ]`, sonstiges Nicht-ASCII zu `_`.

### 5.4 Werkzeugnummern
Siehe F4. Enthält eine Datei mehrere Werkzeuge (z. B. aus einer Vollversion), wird jede
Originalnummer einzeln zugeordnet.

### 5.5 Robustheit
- Datei ohne Werkzeugwechsel oder ohne `M30`/`M2` wird abgelehnt, mit klarer Meldung.
- Große Dateien (≥ 10 MB) müssen flüssig verarbeitet werden. Kein Einfrieren der Oberfläche,
  stattdessen ein Fortschrittsbalken.
- Datei-Kodierung: UTF-8, ungültige Bytes tolerieren.

### 5.6 Prüfungen vor dem Speichern
| Fall | Verhalten |
|---|---|
| unterschiedliche Einheiten (mm/Zoll) | **Fehler**, Abbruch |
| unterschiedlicher Z-Nullpunkt | Warnung |
| unterschiedliche X/Y-Offsets | Warnung („aus demselben Setup?“) |
| unterschiedliche Maschinenmodelle | Warnung |
| `G91` in einer Datei bei aktivierter Eilgang-Option | Warnung, Eilgang-Option wird für den Lauf deaktiviert |

## 6. Eilgang-Wiederherstellung – sicherheitskritisch

Referenz: `restore_rapids()` in `reference/multipass.py`.

- Modale Simulation pro Teil (absolute Koordinaten, G90): aktueller Modus (G0/G1/G2/G3), Position X/Y/Z, Vorschub F.
- Eine **G1-Linearbewegung** wird **nur** zu `G0`, wenn
  - **Start-Z und End-Z ≥ `safe_z`** sind (vollständig über dem Material), **oder**
  - es ein **reiner Rückzug nach oben** ist (nur Z ändert sich, Z steigt).
- Erst umwandeln, wenn X, Y und Z bekannt sind. `G28`/`G53` setzen die Position auf „unbekannt“.
- **Nie** umgewandelt werden Bögen (G2/G3), Eintauchbewegungen, Bewegungen unterhalb `safe_z` und
  Zeilen mit anderen G-/M-Wörtern.
- Ausgabe der umgewandelten Zeile: `G0` + nur die im Original vorhandenen Achsen. Das F-Wort entfällt.
- Folgt danach eine **modale** G1/G2/G3-Zeile ohne G-Wort, wird das passende G-Wort davor ergänzt.
- Wurde ein F-Wort weggelassen, bekommt die nächste G1/G2/G3-Zeile ohne F den aktuellen Vorschub angehängt.
- Statistik: Anzahl, Strecke und Zeit, die vorher im Vorschub gefahren wurde.

**Garantie (muss durch Tests abgesichert sein):** Für jede Bewegungszeile ist die Zielposition
identisch zum Original, jede Schnittbewegung hat denselben Vorschub, und jede Umwandlung erfüllt die
obige Regel.

## 7. Nichtfunktionale Anforderungen

- **Korrektheit vor allem anderen.** Fräsbahnen dürfen sich nie unbemerkt ändern.
- **Leistung:** 10 MB / 1 Mio. Zeilen in unter ca. 3 s einlesen und zusammenfügen, bei flüssiger Oberfläche.
- **Barrierearm:** reine Mausbedienung (siehe 2), gut lesbare Schrift (≥ 14 px), hoher Kontrast,
  **Dark und Light Mode** (System folgen, umschaltbar).
- **Keine Netzwerkzugriffe.**
- **Größe:** Installer möglichst klein (Tauri-typisch < 15 MB).
- **Fehlermeldungen** auf Deutsch, konkret und mit Handlungsempfehlung.

## 8. Oberfläche (Entwurf)

```
┌──────────────────────────────────────────────────────────────────────────┐
│  ◆ MultiPass                                           [☀/☾]  [⚙]        │
├──────────────────────────────────────────────┬───────────────────────────┤
│  ┌ Dateien hier ablegen oder [+ Hinzufügen] ┐│  Ausgewählt: 3_T9_…       │
│  │ ⋮⋮ 1  1_T13_Schruppen.cnc   T13  −2,55 mm ││  Werkzeug: Spiral O 1×3   │
│  │ ⋮⋮ 2  2_T10_Rest.cnc        T10  −2,55 mm ││  Werkzeug-Nr.  [−] 9 [+]  │
│  │ ⋮⋮ 3  3_T9_Rest_Boeden.cnc  T9   −2,60 mm ││  [Als Inventar merken]    │
│  │ ⋮⋮ 4  4_T16_Gravur.cnc      T16  −2,10 mm ││  [▲ Nach oben][▼ Nach unten]│
│  │ ⋮⋮ 5  5_T13_Kontur.cnc      T13  −5,00 mm ││  [🗑 Entfernen]            │
│  └───────────────────────────────────────────┘├───────────────────────────┤
│  Ablauf an der Maschine                       │  Optionen                 │
│  → T13 Spiral O 3.175×12 → 1. Schruppen        │  [●] Leerfahrten als      │
│  → T10 Spiral O 1.5×6   → 2. Rest              │      Eilgang              │
│  → T9  …                                       │  Sicherheitshöhe          │
│                                                │  [−] 1,0 mm [+]           │
│                                                │  ⚠ Pratzen-Hinweis        │
├──────────────────────────────────────────────┴───────────────────────────┤
│  Laufzeit ca. 5 h 40 min → mit Eilgang ca. 4 h 10 min                     │
│                                      [  Zusammenfügen und speichern  ]    │
└──────────────────────────────────────────────────────────────────────────┘
```
- **Leerer Zustand:** große Ablagefläche „NC-Dateien hier hineinziehen oder klicken“.
- Gestaltung modern und ruhig: Karten, abgerundete Ecken, klare Typografie, dezente Animationen.
- Es gibt noch kein Logo. In Phase 1 einen Vorschlag machen (schlicht, z. B. Fräser plus Kette bzw. Pfeil).

## 9. Tests und Abnahme

1. **Rust-Unit-Tests** für Parser, Werkzeugzuordnung, Zusammenfügen und Eilgang-Regeln.
2. **Golden-Tests:** Für die Fixtures (`tests/fixtures/*.cnc`) muss die Rust-Ausgabe der
   Python-Referenz entsprechen (`reference/multipass.py --cli … [--rapid]`). Abweichen darf nur die Zeitstempel-Zeile.
   Die Referenz-Ausgaben einmal erzeugen und als Dateien einchecken.
3. **Sicherheitstest** wie `test_rapid_restore_is_safe`: Positionen und Vorschübe identisch,
   Umwandlungsregel eingehalten.
4. **Großdatei-Test:** Ein synthetisch erzeugtes ~1-Mio.-Zeilen-Programm innerhalb der Zeitvorgabe.
5. **Manuelle Abnahme** durch den Nutzer: echte Angel-Token-Dateien zusammenfügen, in Makera Studio
   laden, Werkzeugwechsel-Dialoge prüfen.

Echte Vergleichswerte der Referenz (Angel-Token, 5 Dateien, ~950.000 Bewegungen): alle Positionen
identisch, 2.487 Leerfahrten umgewandelt (16,0 m, vorher ca. 94 min im Vorschub).

## 10. Lieferumfang

- Quellcode (Git), `README.md` (Deutsch) mit „Installieren“, „Bedienen“ und „Entwickeln“
- Installer für Windows, macOS und Linux (Signierung siehe offene Fragen)
- Kurzanleitung für den Nutzer (1 Seite)
- `reference/` bleibt erhalten (Python-Version und Tests)

## 11. Meilensteine (Vorschlag)

1. **M1 Kern:** Rust-Crate mit Parser, Merge und Eilgang, inkl. Tests und Golden-Abgleich
2. **M2 MVP-Oberfläche:** Laden (Dialog + Drag & Drop), Liste, Sortieren, T-Nummern, Option, Speichern
3. **M3 Komfort:** Inventar-Zuordnung, Einstellungen, Ablaufvorschau, Ergebnis-Dialog mit Makera-Hinweisen, Dark Mode
4. **M4 Ausbau:** Laufzeitschätzung, ggf. 2D-Vorschau
5. **M5 Auslieferung:** Installer per GitHub Actions, README, Kurzanleitung

## 12. Offene Fragen an den Nutzer (in Phase 1 klären)

1. Ist **Node.js** installiert, und welche Version? Soll Claude Code fehlende Werkzeuge installieren?
2. Soll das Projekt auf **GitHub** liegen (für automatische Builds aller drei Systeme)? Öffentlich oder privat?
3. **Installer signieren?** Unsigniert zeigt Windows/macOS eine Warnung. Signieren kostet ggf. Geld.
4. Soll die **Inventar-Zuordnung** mit den bekannten Fräsern vorbelegt werden (#13, #10, #9, #16, #21)?
5. **Automatische Updates** gewünscht (Tauri-Updater), oder reicht ein neuer Installer?
6. Nur **Deutsch**, oder auch Englisch vorsehen?
7. Ist die **2D-Bahnvorschau** gewünscht, und in welcher Ausbaustufe?
8. Gibt es Wünsche zu **Farben/Logo**?
9. Soll die Python-Version weiter ausgeliefert werden, oder nur noch als Referenz im Repo bleiben?
