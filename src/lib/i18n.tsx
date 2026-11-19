import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import type { NcError, Warning } from "./api";

export type Lang = "de" | "en";

const de = {
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

function initialLang(): Lang {
  try {
    const saved = localStorage.getItem("multipass.lang");
    if (saved === "de" || saved === "en") return saved;
  } catch {
    return "de";
  }
  return navigator.language.toLowerCase().startsWith("de") ? "de" : "en";
}

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLang] = useState<Lang>(initialLang);
  useEffect(() => {
    document.documentElement.lang = lang;
    try {
      localStorage.setItem("multipass.lang", lang);
    } catch {
      return;
    }
  }, [lang]);
  const value = useMemo<I18n>(() => {
    const locale = lang === "de" ? "de-DE" : "en-US";
    return {
      lang,
      t: dicts[lang],
      setLang,
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
