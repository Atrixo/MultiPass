# MultiPass – Kurzanleitung

MultiPass fügt mehrere NC-Programme aus Fusion 360 Personal zu **einem** Programm mit Werkzeugwechseln
zusammen. Alles geht mit der Maus.

## In 5 Schritten

1. **Dateien hinzufügen:** NC-Dateien ins Fenster ziehen oder auf „Dateien auswählen“ klicken.
   Mehrere Dateien werden nach Nummer sortiert (1_…, 2_…, 10_…).
2. **Reihenfolge prüfen:** Datei anklicken, dann rechts „Nach oben“ / „Nach unten“. Oder am Griff ⋮⋮ ziehen.
3. **Werkzeugnummern prüfen:** Rechts steht pro Datei die T-Nummer. Ändern mit **−** / **+** oder
   Klick auf die Nummer (Zahlenfeld 1–99). Bekannte Fräser aus dem **Inventar** bekommen ihre Nummer automatisch.
4. **Optional „Leerfahrten als Eilgang“** einschalten (siehe unten).
5. **„Zusammenfügen und speichern“** klicken, Speicherort bestätigen (Vorschlag: `GESAMT.cnc`
   im Ordner der ersten Datei). Mit **„Ordner öffnen“** zeigt MultiPass die fertige Datei.

## Leerfahrten als Eilgang

Fusion Personal gibt alle Eilgänge als langsame Vorschubbewegungen aus. MultiPass kann Fahrten
**über dem Material** wieder zu Eilgängen machen. Das spart oft über eine Stunde.

- Umgewandelt wird nur, was **vollständig über der Sicherheitshöhe** liegt, oder ein reiner Rückzug nach oben.
- **Achtung:** Spannpratzen, die höher sind als die Sicherheitshöhe, kennt das Programm nicht.
  Sicherheitshöhe mit − / + passend einstellen (0,5–20 mm).
- Nur möglich, wenn in Fusion der **Z-Nullpunkt auf „Stock Top“** (Materialoberkante) steht.
  Sonst schaltet MultiPass die Option für diesen Lauf ab und sagt Bescheid.

## Inventar

Oben auf **„Inventar“** klicken. Dort stehen die Fräser mit fester Nummer, z. B. T13 = Spiral O Metal 3.175×12.
Neuen Fräser merken: Datei auswählen, Nummer einstellen, **„Als Inventar merken“**.

## Laufzeit und Vorschau

- Unten steht die geschätzte Laufzeit, ohne und mit Eilgang. Das Eilgang-Tempo der Maschine unter
  „Optionen“ einstellen, dann wird die Schätzung genauer.
- Reiter **„Bahnvorschau“**: Draufsicht aller Bahnen, farbig je Werkzeug, Eilgänge gestrichelt.
  „Groß anzeigen“ öffnet sie bildschirmfüllend.

## An der Makera Z1

- Makera Studio fragt bei jedem Wechsel **„Please change to Tool n“**. Passenden Fräser einsetzen,
  **Confirm** klicken. Das Werkzeug wird danach automatisch vermessen.
- **Auto-Z nur auf unbearbeiteter Materialfläche** antasten, nicht im schon gefrästen Bereich.

## Wenn etwas nicht klappt

| Meldung | Was tun |
|---|---|
| „kein Werkzeugwechsel gefunden“ | Datei ist kein Fusion/Carvera-Programm. Mit Postprozessor `Carvera.cps` neu ausgeben. |
| „kein Programmende gefunden“ | Datei unvollständig. In Fusion neu ausgeben. |
| „unterschiedliche Einheiten“ | Alle Dateien in mm ausgeben. |
| Warnung „Z-Nullpunkt“ / „X/Y-Versätze“ | Prüfen, ob alle Dateien aus demselben Setup stammen. |
