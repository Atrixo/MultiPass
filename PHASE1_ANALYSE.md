# MultiPass – Phase 1: Anforderungsanalyse

Stand: 19.11.2026. Grundlage: `ANFORDERUNGEN.md`, `CLAUDE.md`, `reference/multipass.py`, `reference/test_multipass.py`.

## 1. Umgebung (geprüft)

| Werkzeug | Befund |
|---|---|
| Node.js | v22.22.1, npm 11.6.2 – ausreichend für Vite/Tauri |
| Rust | rustc/cargo 1.94.1, Ziel `x86_64-pc-windows-msvc` |
| MSVC | Visual Studio Build Tools 2019 vorhanden |
| WebView2 | 157 installiert |
| git / gh | vorhanden (2.45 / 2.102) |
| Python | 3.14 (C:\Python314) – Referenztests laufen grün (5/5) |

Es muss voraussichtlich nichts nachinstalliert werden. Der Ordner `Desktop\Angel-Token NC\MultiPass` ist noch **kein Git-Repository**.

## 2. Lücken und Widersprüche

### Sicherheit (Eilgang)
1. **Z-Nullpunkt nicht geprüft.** `safe_z` gilt nur, wenn Z0 = Materialoberkante ist. Steht der Nullpunkt z. B. auf „Model Bottom“, wäre `safe_z = 1 mm` mitten im Material. Die Referenz prüft das nicht.
   **Vorschlag:** Eilgang-Option nur erlauben, wenn alle Dateien `Z Origin Set To : Stock Top` melden; sonst Warnung und Option für den Lauf aus (wie bei G91). Das weicht bewusst von der Referenz ab und bekommt einen eigenen Test; die Golden-Tests bleiben unberührt, weil die Fixtures „Stock Top“ haben.
2. **G30** setzt in der Referenz die Position ebenfalls zurück, in Abschnitt 6 steht nur G28/G53. Referenzverhalten (G28/G30/G53) wird übernommen.
3. Abschnitt 5.2 nennt `M0?(30|2)\b`, die Referenz nutzt zusätzlich `(?!\d)`. Referenz gilt.

### Golden-Test-Fallen (Rust muss Python exakt nachbilden)
4. Python `splitlines()` trennt auch an `\v`, `\f`, `\x1c–\x1e`, `\x85`, `\u2028/29`. Wird in Rust nachgebaut.
5. Zahlenformat `_fmt` (`{:.4f}`, Nullen weg, `-0.` → `0.`) wird nachgebaut und gezielt getestet.
6. Ungültige UTF-8-Bytes werden zu `U+FFFD` und so auch geschrieben (wie Referenz).
7. Zeitstempel-Zeile wird im Golden-Vergleich ausgeblendet; Rust-Kern bekommt den Zeitstempel als Parameter.

### Fachliche Lücken
8. **„Gesparte Zeit“ (F7)** braucht eine Eilgang-Geschwindigkeit; die Referenz liefert nur „Zeit vorher im Vorschub“. Bis M4 zeigt das Ergebnis „vorher ca. X min im Vorschub“, ab M4 die echte Ersparnis.
9. **Inventar vs. Automatik (F4):** Was ist „nächste freie Nummer“, wenn das Inventar T13 belegt? Vorschlag: Inventar zuerst anwenden, unbekannte Werkzeuge bekommen die kleinste Nummer, die weder im Inventar noch im Job belegt ist. Abgleich über die Beschreibung (Leerraum normalisiert, Groß/Klein egal).
10. **Mehrere Werkzeuge pro Datei (5.4)** passen nicht zu „eine T-Nr. pro Datei“ in der Oberfläche. Vorschlag: rechts eine Zeile mit − / + je Originalwerkzeug.
11. **− / + für 1–99** heißt bis zu 98 Klicks. Vorschlag: zusätzlich ein Zahlenraster (Klick auf Nummer öffnet ein 10×10-Feld), ein Klick genügt.
12. **Gleiches Werkzeug direkt hintereinander:** Die Referenz schreibt trotzdem `Tn M6` (Makera fragt erneut und misst neu). Wird so beibehalten.
13. **Natürliche Sortierung (F1):** gilt für die neu hinzugefügte Gruppe; sie wird ans Ende der bestehenden Liste angehängt.
14. **Fenstergröße (F9)** besser mit `tauri-plugin-window-state` statt `store`.

### Technische Risiken
15. **Drag & Drop unter Windows:** Ist Tauris Datei-Drop aktiv, funktioniert HTML5-Drag & Drop im Fenster nicht. Umsortieren daher mit **dnd-kit** (Pointer-Events), das funktioniert parallel.
16. **Große Dateien:** Laden/Zusammenfügen läuft in einem Rust-Thread, Fortschritt per Tauri-Event; das Frontend bekommt nur Kopfdaten, nie die Million Zeilen.
17. **Signierung:** Selbstsignierte Zertifikate kommen nicht in Frage. Bleibt: unsigniert (Warnhinweis bei Installation) oder kostenpflichtiges Zertifikat.

### Organisation
18. `ANFORDERUNGEN.md` liegt in `Desktop\Angel-Token NC\MultiPass` (kein Git), das Git-Repository mit der alten Version in `Downloads\MultiPass\MultiPass`. Welcher Ordner wird das neue Projekt?

## 3. UI-Bibliothek

**Tailwind CSS 4 + shadcn/ui (Radix) + lucide-react + dnd-kit.** Radix liefert zugängliche Dialoge, Schalter und Menüs; shadcn-Komponenten liegen als Quellcode im Projekt (keine schwere Abhängigkeit, kleiner Installer). Alternativen: Mantine (mehr fertig, aber größer und eigener Stil), Fluent UI (Windows-Look, auf macOS/Linux fremd).

## 4. Logo-Vorschlag

Abgerundetes Quadrat, darin drei kleine Kreise (Werkzeuge) nebeneinander, verbunden durch einen Pfeil zu einem großen Kreis rechts – „mehrere zu einem“. Einfarbig, funktioniert als 16-px-Symbol.

## 5. Umsetzungsplan

| Meilenstein | Inhalt | Abnahme |
|---|---|---|
| **M1 Kern** | Cargo-Workspace: `multipass-core` (Parser, Werkzeugzuordnung, Merge, Eilgang), `multipass-cli`. Golden-Dateien aus der Referenz erzeugen und einchecken. Sicherheitstest, Großdatei-Test (1 Mio. Zeilen < 3 s) | `cargo test` grün, Golden identisch |
| **M2 MVP** | Tauri 2 + React/TS/Vite, Laden per Dialog und Drag & Drop, Liste, Umsortieren (Knöpfe + Ziehen), T-Nummern, Eilgang-Option mit Warnung, Prüfungen, Speichern mit Fortschritt | Fixtures per Maus zusammenfügen |
| **M3 Komfort** | Inventar, Einstellungen, Ablaufvorschau, Ergebnis-Dialog mit Makera-Hinweisen und „Ordner öffnen“, Dark/Light | Angel-Token-Dateien, Test in Makera Studio |
| **M4 Ausbau** | Laufzeitschätzung, Vergleich mit/ohne Eilgang; optional 2D-Vorschau | Werte plausibel zu Referenz (94 min) |
| **M5 Auslieferung** | GitHub Actions mit `tauri-action`, Installer Win/macOS/Linux, README, Kurzanleitung | Installer starten auf allen drei Systemen |

Projektstruktur:
```
MultiPass/
  crates/multipass-core/   Kernlogik + Tests + Golden
  crates/multipass-cli/
  src-tauri/               Tauri-App (nutzt multipass-core)
  src/                     React-Frontend
  reference/               Python-Version (bleibt)
  tests/fixtures/
```

## 6. Entscheidungen des Nutzers (19.11.2026)

| Frage | Entscheidung |
|---|---|
| Projektordner | `Desktop\Angel-Token NC\MultiPass` (neues Git-Repo) |
| GitHub | ja, **privat**, Builds per GitHub Actions |
| Signierung | **unsigniert** |
| Inventar | mit #13, #10, #9, #16, #21 **vorbelegen** |
| Updates | **kein** Auto-Updater, neuer Installer genügt |
| Sprache | **Deutsch + Englisch** mit Umschalter |
| 2D-Vorschau | ja, in **M4** |
| Python-Version | nur noch **Referenz** in `reference/` |
| Z-Nullpunkt ≠ Stock Top | Warnung, **Eilgang für den Lauf gesperrt** |
| T-Nummer | − / + **und Zahlenraster** 1–99 |
| Farben | **Blau/Petrol**, ruhig |
| Logo | drei kleine Kreise → Pfeil → großer Kreis |

Offen: Freigabe des Plans (Abschnitt 5) für Phase 2.

## 7. Bewusste Abweichungen von der Python-Referenz (M1)

Gefunden durch Zufallstests (Fuzzing) des Rust-Kerns. Für die echten Fusion-Dateien ändert sich nichts:
Die Angel-Token-Dateien ergeben byte-gleich dieselbe Ausgabe wie die Referenz.

| Fall | Referenz | Rust-Kern |
|---|---|---|
| Nachgetragenes F-Wort bei Zeile mit `;`-Kommentar | F landet **im Kommentar**, Maschine fräst mit falschem Vorschub | F wird vor dem `;` eingefügt |
| Umgewandelte Zeile hatte `F0` | F geht verloren | F wird nachgetragen |
| Koordinate mit mehr als 4 Nachkommastellen in umgewandelter Zeile | auf 4 Stellen gerundet, Position ändert sich minimal | exakter Wert bleibt erhalten |
| Z-Nullpunkt nicht „Stock Top“ oder unbekannt | Eilgang wird trotzdem umgewandelt | Warnung, Eilgang für den Lauf aus |
| Nicht-ASCII-Zeichen im G-Code (außerhalb von Kommentaren) | wird verarbeitet | Eilgang für diesen Teil aus, Warnung |
| Unlesbare Zahl im Dateikopf / Ziffern außerhalb ASCII in T-Nummern | Absturz bzw. verarbeitet | Datei wird mit Meldung abgelehnt |
