import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import type { NcError, Warning } from "./api";
import { saveSetting, type Lang } from "./settings";

export type { Lang };

const de = {
  runtime: "Laufzeit",
  about: (t: string) => `ca. ${t}`,
  withRapid: "mit Eilgang",
  withoutRapid: "ohne Eilgang",
  saves: (t: string) => `spart ca. ${t}`,
  rapidBlocked: "Eilgang hier nicht möglich (Z-Nullpunkt oder G91)",
  runtimeHint: "Schätzung aus Strecke und Vorschub, ohne Beschleunigung und Werkzeugwechsel.",
  estimating: "Laufzeit wird geschätzt …",
  rapidSpeed: "Eilgang-Tempo der Maschine (nur für die Schätzung)",
  tabSequence: "Ablauf",
  tabPreview: "Bahnvorschau",
  previewLoading: "Bahnen werden berechnet …",
  previewOnlySelected: "Nur ausgewählte Datei",
  previewRapids: "Eilgänge zeigen",
  previewDashed: "gestrichelt = Eilgang · Mausrad zoomt, Ziehen verschiebt",
  previewEmpty: "Keine Bahnen gefunden.",
  previewOrigin: "Nullpunkt",
  previewExpand: "Groß anzeigen",
  zoomIn: "Hineinzoomen",
  zoomOut: "Herauszoomen",
  zoomFit: "Einpassen",
  previewZoomHint: "Mausrad zoomt, Ziehen verschiebt",
  modePaths: "Bahnen",
  modeSim: "Simulation",
  simLoading: "Simulation wird vorbereitet …",
  simComputing: (p: number) => `Abtrag wird berechnet … ${p} %`,
  simPlay: "Abspielen",
  simPause: "Anhalten",
  simStart: "Zum Anfang",
  simEnd: "Zum Ende",
  simPrevPart: "Vorheriger Teil",
  simNextPart: "Nächster Teil",
  simSpeed: "Tempo",
  simTimeline: "Zeitleiste",
  simStopAtChange: "Bei Werkzeugwechsel anhalten",
  simPart: (i: number, n: number, name: string) => `Teil ${i} von ${n}: ${name}`,
  simToolChange: (tool: string) => `Werkzeugwechsel: jetzt ${tool} einsetzen. Weiter mit „Abspielen“.`,
  simNoMaterial: "Materialabtrag nur möglich, wenn der Z-Nullpunkt auf der Materialoberkante liegt. Die Bahn wird trotzdem abgespielt.",
  simUnknownTool: "Werkzeugform nicht erkannt, als Schaftfräser Ø 3,175 mm angenommen.",
  simHint: "Vereinfachte Draufsicht. Spannpratzen und Halter werden nicht geprüft.",
  simInAir: "über dem Material",
  simCutting: "im Material",
  simEmpty: "Keine Bewegungen gefunden.",
  openFilesTitle: "Dateien geöffnet",
  inventoryButton: "Inventar",
  inventoryTitle: "Werkzeug-Inventar",
  inventoryIntro:
    "Diese Werkzeuge bekommen beim Laden automatisch ihre feste Nummer. Erkannt wird die Werkzeugbeschreibung aus Fusion.",
  inventoryEmpty: "Noch keine Werkzeuge gemerkt. Datei auswählen und „Als Inventar merken“ klicken.",
  rememberTool: "Als Inventar merken",
  rememberToolWas: (n: number) => `Als Inventar merken (bisher T${n})`,
  inInventory: (n: number) => `Im Inventar als T${n}`,
  inventoryConflictTitle: "Nummer schon vergeben",
  inventoryConflict: (n: number, other: string) => `T${n} ist im Inventar schon „${other}“ zugeordnet. Soll die Nummer ab jetzt für dieses Werkzeug gelten?`,
  replace: "Ersetzen",
  deleteEntry: "Löschen",
  reallyDelete: "Wirklich löschen",
  restoreDefaults: "Vorbelegung wiederherstellen",
  openFolder: "Ordner öffnen",
  appTagline: "NC-Programme mit Werkzeugwechseln zusammenfügen",
  language: "Sprache",
  theme: "Darstellung",
  themeSystem: "System",
  themeLight: "Hell",
  themeDark: "Dunkel",
  addFiles: "Dateien hinzufügen",
  dropTitle: "NC-Dateien hier hineinziehen",
  dropOr: "oder",
  dropClick: "Dateien auswählen",
  dropHint: "Fusion 360 Personal · Carvera.cps · .cnc .nc .gcode .ngc .tap .txt",
  dropNow: "Loslassen zum Hinzufügen",
  filesTitle: "Dateien",
  filesCount: (n: number) => (n === 1 ? "1 Datei" : `${n} Dateien`),
  dragHandle: "Ziehen zum Umsortieren",
  deepest: "tiefster Punkt",
  lines: "Zeilen",
  selected: "Ausgewählt",
  fileOf: (i: number, n: number) => `Datei ${i} von ${n}`,
  tool: "Werkzeug",
  toolNumber: "Werkzeug-Nr.",
  manual: "von Hand",
  chooseNumber: "Nummer wählen",
  chooseNumberFor: (d: string) => `Werkzeugnummer für „${d}“`,
  usedElsewhere: "schon vergeben",
  zOrigin: "Z-Nullpunkt",
  unknown: "unbekannt",
  notStockTop: "Nicht Materialoberkante – Eilgang-Option wird für diesen Lauf gesperrt.",
  incremental: "Datei nutzt G91 (inkrementell).",
  moveUp: "Nach oben",
  moveDown: "Nach unten",
  remove: "Entfernen",
  removeAll: "Alle entfernen",
  removeAllQuestion: "Alle Dateien aus der Liste entfernen?",
  removeAllHint: "Die Dateien selbst bleiben auf der Festplatte unverändert.",
  autoNumbers: "Nummern automatisch",
  sequenceTitle: "Ablauf an der Maschine",
  sequenceEmpty: "Noch keine Dateien.",
  changeTo: (t: number) => `Werkzeugwechsel auf T${t}`,
  sameTool: "gleiches Werkzeug – Makera fragt trotzdem",
  optionsTitle: "Optionen",
  fanLabel: "Lüfter beim Werkzeugwechsel aus",
  fanHint: "Schaltet den Spindellüfter vor jedem Wechsel aus (M812) und danach wieder ein (M811 S100).",
  rapidLabel: "Leerfahrten als Eilgang",
  rapidHint: "Fahrten über dem Material wieder als G0 ausgeben. Spart oft viel Zeit.",
  safeZ: "Sicherheitshöhe über Materialoberkante",
  clampWarning: "Spannpratzen, die höher sind als dieser Wert, kennt das Programm nicht.",
  less: "weniger",
  more: "mehr",
  mergeSave: "Zusammenfügen und speichern",
  working: "Bitte warten …",
  loading: (d: number, n: number) => `Lade Datei ${Math.min(d + 1, n)} von ${n}`,
  merging: "Füge zusammen …",
  saving: "Speichere …",
  cancel: "Abbrechen",
  close: "Schließen",
  ok: "OK",
  yesRemoveAll: "Ja, alle entfernen",
  saveAnyway: "Trotzdem speichern",
  warningsTitle: "Bitte prüfen",
  warningsIntro: "Beim Zusammenfügen ist Folgendes aufgefallen:",
  errorTitle: "Das hat nicht geklappt",
  loadErrorsTitle: "Einige Dateien wurden nicht geladen",
  loadErrorsHint: "Die übrigen Dateien wurden trotzdem hinzugefügt.",
  saveDialogTitle: "Zusammengefügtes Programm speichern",
  filterNc: "NC-Programme",
  filterAll: "Alle Dateien",
  doneTitle: "Gespeichert",
  savedTo: "Gespeichert unter",
  order: "Reihenfolge",
  rapidResult: (n: number, m: string, min: string) =>
    `${n} Leerfahrten als Eilgang (${m} m, vorher ca. ${min} min im Vorschub).`,
  makeraTitle: "An der Makera Z1",
  makeraChange: "Makera Studio fragt bei jedem Wechsel „Please change to Tool n“. Passenden Fräser einsetzen und Confirm klicken.",
  makeraProbe: "Auto-Z nur auf unbearbeiteter Materialfläche antasten, nicht im Fräsbereich – sonst stimmt Z nicht.",
  errors: {
    io: (e: { file: string; message: string }) => `${e.file}: Datei konnte nicht gelesen werden (${e.message}).`,
    noToolChange: (e: { file: string }) =>
      `${e.file}: kein Werkzeugwechsel (T.. M6) gefunden. Ist das eine NC-Datei aus Fusion mit Carvera.cps?`,
    noProgramEnd: (e: { file: string }) =>
      `${e.file}: kein Programmende (M30 / M2) gefunden. Die Datei ist vielleicht unvollständig – bitte neu ausgeben.`,
    structure: (e: { file: string }) => `${e.file}: Aufbau nicht erkannt (Programmende vor dem Werkzeugwechsel).`,
    badNumber: (e: { file: string; line: number }) => `${e.file}: Zahl in Zeile ${e.line} nicht lesbar.`,
    alreadyMerged: (e: { file: string }) =>
      `${e.file}: ist schon mit MultiPass zusammengefügt (enthält alle Teile) und wurde nicht übernommen – sonst würde alles doppelt gefräst.`,
    noFiles: () => "Keine Dateien ausgewählt.",
    mixedUnits: () =>
      "Die Dateien verwenden unterschiedliche Einheiten (mm / Zoll). Bitte alle in derselben Einheit ausgeben.",
  },
  warnings: {
    zOriginDiffers: (w: { origins: string[] }) => `Unterschiedlicher Z-Nullpunkt in den Dateien: ${w.origins.join(", ")}.`,
    offsetsDiffer: () => "Unterschiedliche X/Y-Versätze in den Dateiköpfen – stammen die Dateien aus demselben Setup?",
    machinesDiffer: (w: { models: string[] }) => `Dateien für unterschiedliche Maschinen: ${w.models.join(", ")}.`,
    rapidOffIncremental: () => "Eine Datei nutzt G91 (inkrementell) – Eilgang-Umwandlung wurde deshalb abgeschaltet.",
    rapidOffZOrigin: (w: { files: string[] }) =>
      `Z-Nullpunkt ist nicht die Materialoberkante (Stock Top) in: ${w.files.join(", ")} – Eilgang-Umwandlung wurde deshalb abgeschaltet.`,
    rapidOffNonAscii: (w: { file: string; line: number }) =>
      `${w.file}: ungewöhnliche Zeichen in Zeile ${w.line} – Eilgang-Umwandlung für diesen Teil abgeschaltet.`,
  },
};

type Dict = typeof de;

const en: Dict = {
  runtime: "Run time",
  about: (t: string) => `approx. ${t}`,
  withRapid: "with rapids",
  withoutRapid: "without rapids",
  saves: (t: string) => `saves approx. ${t}`,
  rapidBlocked: "Rapids not possible here (Z origin or G91)",
  runtimeHint: "Estimated from distance and feed rate, without acceleration and tool changes.",
  estimating: "Estimating run time …",
  rapidSpeed: "Machine rapid speed (for the estimate only)",
  tabSequence: "Sequence",
  tabPreview: "Toolpath preview",
  previewLoading: "Calculating toolpaths …",
  previewOnlySelected: "Selected file only",
  previewRapids: "Show rapids",
  previewDashed: "dashed = rapid · wheel zooms, drag pans",
  previewEmpty: "No toolpaths found.",
  previewOrigin: "Origin",
  previewExpand: "Enlarge",
  zoomIn: "Zoom in",
  zoomOut: "Zoom out",
  zoomFit: "Fit",
  previewZoomHint: "Mouse wheel zooms, drag to pan",
  modePaths: "Toolpaths",
  modeSim: "Simulation",
  simLoading: "Preparing simulation …",
  simComputing: (p: number) => `Calculating material removal … ${p} %`,
  simPlay: "Play",
  simPause: "Pause",
  simStart: "To start",
  simEnd: "To end",
  simPrevPart: "Previous part",
  simNextPart: "Next part",
  simSpeed: "Speed",
  simTimeline: "Timeline",
  simStopAtChange: "Pause at tool change",
  simPart: (i: number, n: number, name: string) => `Part ${i} of ${n}: ${name}`,
  simToolChange: (tool: string) => `Tool change: insert ${tool} now. Continue with "Play".`,
  simNoMaterial: "Material removal needs the Z origin on the stock top. The toolpath is still played back.",
  simUnknownTool: "Tool shape not recognised, assuming a 3.175 mm flat end mill.",
  simHint: "Simplified top view. Clamps and holders are not checked.",
  simInAir: "above the stock",
  simCutting: "in the material",
  simEmpty: "No moves found.",
  openFilesTitle: "Files opened",
  inventoryButton: "Inventory",
  inventoryTitle: "Tool inventory",
  inventoryIntro: "These tools automatically get their fixed number when loading. Matched by the tool description from Fusion.",
  inventoryEmpty: "No tools saved yet. Select a file and click “Save to inventory”.",
  rememberTool: "Save to inventory",
  rememberToolWas: (n: number) => `Save to inventory (was T${n})`,
  inInventory: (n: number) => `In inventory as T${n}`,
  inventoryConflictTitle: "Number already used",
  inventoryConflict: (n: number, other: string) => `T${n} is already assigned to “${other}” in the inventory. Use this number for this tool from now on?`,
  replace: "Replace",
  deleteEntry: "Delete",
  reallyDelete: "Really delete",
  restoreDefaults: "Restore defaults",
  openFolder: "Open folder",
  appTagline: "Merge NC programs with tool changes",
  language: "Language",
  theme: "Appearance",
  themeSystem: "System",
  themeLight: "Light",
  themeDark: "Dark",
  addFiles: "Add files",
  dropTitle: "Drag NC files here",
  dropOr: "or",
  dropClick: "Choose files",
  dropHint: "Fusion 360 Personal · Carvera.cps · .cnc .nc .gcode .ngc .tap .txt",
  dropNow: "Release to add",
  filesTitle: "Files",
  filesCount: (n: number) => (n === 1 ? "1 file" : `${n} files`),
  dragHandle: "Drag to reorder",
  deepest: "lowest point",
  lines: "lines",
  selected: "Selected",
  fileOf: (i: number, n: number) => `File ${i} of ${n}`,
  tool: "Tool",
  toolNumber: "Tool no.",
  manual: "manual",
  chooseNumber: "Choose number",
  chooseNumberFor: (d: string) => `Tool number for “${d}”`,
  usedElsewhere: "already used",
  zOrigin: "Z origin",
  unknown: "unknown",
  notStockTop: "Not stock top – rapid option will be disabled for this run.",
  incremental: "File uses G91 (incremental).",
  moveUp: "Move up",
  moveDown: "Move down",
  remove: "Remove",
  removeAll: "Remove all",
  removeAllQuestion: "Remove all files from the list?",
  removeAllHint: "The files on disk stay unchanged.",
  autoNumbers: "Automatic numbers",
  sequenceTitle: "Sequence on the machine",
  sequenceEmpty: "No files yet.",
  changeTo: (t: number) => `Tool change to T${t}`,
  sameTool: "same tool – Makera still asks",
  optionsTitle: "Options",
  fanLabel: "Spindle fan off during tool changes",
  fanHint: "Turns the spindle fan off before each change (M812) and back on afterwards (M811 S100).",
  rapidLabel: "Air moves as rapids",
  rapidHint: "Output moves above the stock as G0 again. Often saves a lot of time.",
  safeZ: "Safe height above stock top",
  clampWarning: "Clamps higher than this value are unknown to the program.",
  less: "less",
  more: "more",
  mergeSave: "Merge and save",
  working: "Please wait …",
  loading: (d: number, n: number) => `Loading file ${Math.min(d + 1, n)} of ${n}`,
  merging: "Merging …",
  saving: "Saving …",
  cancel: "Cancel",
  close: "Close",
  ok: "OK",
  yesRemoveAll: "Yes, remove all",
  saveAnyway: "Save anyway",
  warningsTitle: "Please check",
  warningsIntro: "The following was noticed while merging:",
  errorTitle: "That did not work",
  loadErrorsTitle: "Some files were not loaded",
  loadErrorsHint: "The other files were added anyway.",
  saveDialogTitle: "Save merged program",
  filterNc: "NC programs",
  filterAll: "All files",
  doneTitle: "Saved",
  savedTo: "Saved to",
  order: "Order",
  rapidResult: (n: number, m: string, min: string) =>
    `${n} air moves as rapids (${m} m, previously approx. ${min} min at feed rate).`,
  makeraTitle: "On the Makera Z1",
  makeraChange: "Makera Studio asks “Please change to Tool n” at every change. Insert the matching cutter and click Confirm.",
  makeraProbe: "Probe Auto-Z only on untouched stock, not inside the machined area – otherwise Z will be wrong.",
  errors: {
    io: (e) => `${e.file}: could not be read (${e.message}).`,
    noToolChange: (e) => `${e.file}: no tool change (T.. M6) found. Is this an NC file from Fusion with Carvera.cps?`,
    noProgramEnd: (e) => `${e.file}: no program end (M30 / M2) found. The file may be incomplete – please post again.`,
    structure: (e) => `${e.file}: structure not recognized (program end before tool change).`,
    badNumber: (e) => `${e.file}: number in line ${e.line} not readable.`,
    alreadyMerged: (e) =>
      `${e.file}: was already merged with MultiPass (contains all parts) and was not added – otherwise everything would be milled twice.`,
    noFiles: () => "No files selected.",
    mixedUnits: () => "The files use different units (mm / inch). Please post all in the same unit.",
  },
  warnings: {
    zOriginDiffers: (w) => `Different Z origin in the files: ${w.origins.join(", ")}.`,
    offsetsDiffer: () => "Different X/Y offsets in the file headers – are the files from the same setup?",
    machinesDiffer: (w) => `Files for different machines: ${w.models.join(", ")}.`,
    rapidOffIncremental: () => "A file uses G91 (incremental) – rapid conversion was therefore disabled.",
    rapidOffZOrigin: (w) =>
      `Z origin is not the stock top in: ${w.files.join(", ")} – rapid conversion was therefore disabled.`,
    rapidOffNonAscii: (w) => `${w.file}: unusual characters in line ${w.line} – rapid conversion disabled for this part.`,
  },
};

const dicts: Record<Lang, Dict> = { de, en };

export function errorText(t: Dict, e: NcError): string {
  switch (e.code) {
    case "io":
      return t.errors.io(e);
    case "noToolChange":
      return t.errors.noToolChange(e);
    case "noProgramEnd":
      return t.errors.noProgramEnd(e);
    case "structure":
      return t.errors.structure(e);
    case "badNumber":
      return t.errors.badNumber(e);
    case "alreadyMerged":
      return t.errors.alreadyMerged(e);
    case "noFiles":
      return t.errors.noFiles();
    case "mixedUnits":
      return t.errors.mixedUnits();
  }
}

export function warningText(t: Dict, w: Warning): string {
  switch (w.code) {
    case "zOriginDiffers":
      return t.warnings.zOriginDiffers(w);
    case "offsetsDiffer":
      return t.warnings.offsetsDiffer();
    case "machinesDiffer":
      return t.warnings.machinesDiffer(w);
    case "rapidOffIncremental":
      return t.warnings.rapidOffIncremental();
    case "rapidOffZOrigin":
      return t.warnings.rapidOffZOrigin(w);
    case "rapidOffNonAscii":
      return t.warnings.rapidOffNonAscii(w);
  }
}

interface I18n {
  lang: Lang;
  t: Dict;
  setLang: (l: Lang) => void;
  num: (v: number, digits?: number, minDigits?: number) => string;
}

const Ctx = createContext<I18n | null>(null);

export function I18nProvider({ children, initial }: { children: ReactNode; initial?: Lang }) {
  const [lang, setLangState] = useState<Lang>(initial ?? "en");
  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);
  const value = useMemo<I18n>(() => {
    const locale = lang === "de" ? "de-DE" : "en-US";
    return {
      lang,
      t: dicts[lang],
      setLang: (l) => {
        setLangState(l);
        saveSetting("lang", l);
      },
      num: (v, digits = 2, minDigits = 0) =>
        new Intl.NumberFormat(locale, { minimumFractionDigits: minDigits, maximumFractionDigits: digits }).format(v),
    };
  }, [lang]);
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useI18n(): I18n {
  const v = useContext(Ctx);
  if (!v) throw new Error("I18nProvider fehlt");
  return v;
}
