import { listen } from "@tauri-apps/api/event";
import { join } from "@tauri-apps/api/path";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  Info,
  Merge,
  Monitor,
  Moon,
  RotateCcw,
  Sun,
  Trash2,
  Wrench,
  XCircle,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import logo from "./assets/logo.svg";
import { DetailPanel } from "./components/DetailPanel";
import { FileList } from "./components/FileList";
import { InventoryDialog } from "./components/InventoryDialog";
import { NumberGrid } from "./components/NumberGrid";
import { OptionsPanel } from "./components/OptionsPanel";
import { Sequence } from "./components/Sequence";
import { Button, Card, Modal, Notice, cx } from "./components/ui";
import {
  NC_EXTENSIONS,
  api,
  isCommandError,
  stamp,
  type InventoryEntry,
  type LoadError,
  type LoadProgress,
  type Prepared,
  type Saved,
} from "./lib/api";
import { errorText, useI18n, warningText } from "./lib/i18n";
import { dirOf, saveSetting, type Settings } from "./lib/settings";
import {
  clampSafeZ,
  effectiveMap,
  forgetTool,
  hasManual,
  initialState,
  inventoryConflict,
  mergeDefaults,
  reducer,
  rememberTool,
  toolNumber,
} from "./lib/state";
import { useTheme } from "./lib/theme";

type Busy = { kind: "load"; done: number; total: number } | { kind: "merge" } | { kind: "save" } | null;

interface OrderLine {
  position: number;
  name: string;
  tools: string;
}

type DialogState =
  | { kind: "none" }
  | { kind: "loadErrors"; errors: LoadError[] }
  | { kind: "error"; message: string }
  | { kind: "warnings"; prepared: Prepared; order: OrderLine[] }
  | { kind: "done"; prepared: Prepared; saved: Saved; order: OrderLine[] }
  | { kind: "confirmClear" }
  | { kind: "grid"; id: number; orig: number; description: string }
  | { kind: "inventory" }
  | { kind: "inventoryConflict"; description: string; number: number; other: InventoryEntry };

export default function App({ settings }: { settings: Settings }) {
  const { t, lang, setLang, num } = useI18n();
  const [theme, cycleTheme] = useTheme(settings.theme);
  const [state, dispatch] = useReducer(reducer, {
    ...initialState,
    restore: settings.restore ?? false,
    safeZ: clampSafeZ(settings.safeZ ?? initialState.safeZ),
  });
  const [inventory, setInventory] = useState<InventoryEntry[]>(settings.inventory ?? []);
  const [lastDir, setLastDir] = useState<string | undefined>(settings.lastDir);
  const [busy, setBusy] = useState<Busy>(null);
  const [dialog, setDialog] = useState<DialogState>({ kind: "none" });
  const [dragOver, setDragOver] = useState(false);
  const busyRef = useRef(false);

  const errorMessage = useCallback(
    (e: unknown) => {
      if (isCommandError(e)) return e.error ? errorText(t, e.error) : e.message;
      return String(e);
    },
    [t],
  );

  useEffect(() => saveSetting("restore", state.restore), [state.restore]);
  useEffect(() => saveSetting("safeZ", state.safeZ), [state.safeZ]);

  const updateInventory = (inv: InventoryEntry[]) => {
    setInventory(inv);
    saveSetting("inventory", inv);
  };

  const remember = (description: string, number: number) => {
    const other = inventoryConflict(inventory, description, number);
    if (other) setDialog({ kind: "inventoryConflict", description, number, other });
    else updateInventory(rememberTool(inventory, description, number));
  };

  const loadPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0 || busyRef.current) return;
      busyRef.current = true;
      setBusy({ kind: "load", done: 0, total: paths.length });
      try {
        const out = await api.loadFiles(paths);
        dispatch({ type: "add", infos: out.files });
        if (out.files.length > 0) {
          const d = dirOf(out.files[0].path);
          setLastDir(d);
          saveSetting("lastDir", d);
        }
        if (out.errors.length > 0) setDialog({ kind: "loadErrors", errors: out.errors });
      } catch (e) {
        setDialog({ kind: "error", message: errorMessage(e) });
      } finally {
        busyRef.current = false;
        setBusy(null);
      }
    },
    [errorMessage],
  );

  useEffect(() => {
    const un = listen<LoadProgress>("load-progress", (e) => {
      setBusy((b) => (b && b.kind === "load" ? { kind: "load", done: e.payload.done, total: e.payload.total } : b));
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const un = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") setDragOver(true);
      else if (p.type === "leave") setDragOver(false);
      else if (p.type === "drop") {
        setDragOver(false);
        loadPaths(p.paths);
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, [loadPaths]);

  useEffect(() => {
    api.initialFiles().then(loadPaths).catch(() => undefined);
  }, [loadPaths]);

  const idKey = state.entries.map((e) => e.id).join(",");
  const invKey = JSON.stringify(inventory);
  useEffect(() => {
    const ids = idKey ? idKey.split(",").map(Number) : [];
    if (ids.length === 0) return;
    let stale = false;
    api
      .autoTools(ids, JSON.parse(invKey) as InventoryEntry[])
      .then((maps) => !stale && dispatch({ type: "setAuto", ids, maps }))
      .catch((e) => setDialog({ kind: "error", message: errorMessage(e) }));
    return () => {
      stale = true;
    };
  }, [idKey, invKey, errorMessage]);

  const restoreDefaults = async () => {
    try {
      updateInventory(mergeDefaults(inventory, await api.defaultInventory()));
    } catch (e) {
      setDialog({ kind: "error", message: errorMessage(e) });
    }
  };

  const chooseFiles = async () => {
    const picked = await open({
      multiple: true,
      directory: false,
      defaultPath: lastDir,
      filters: [
        { name: t.filterNc, extensions: NC_EXTENSIONS },
        { name: t.filterAll, extensions: ["*"] },
      ],
    });
    if (picked) loadPaths(Array.isArray(picked) ? picked : [picked]);
  };

  const remove = (id: number) => {
    dispatch({ type: "remove", id });
    api.forgetFiles([id]).catch(() => undefined);
  };

  const clearAll = () => {
    api.forgetFiles(state.entries.map((e) => e.id)).catch(() => undefined);
    dispatch({ type: "clear" });
    setDialog({ kind: "none" });
  };

  const orderLines = (): OrderLine[] =>
    state.entries.map((e, i) => ({
      position: i + 1,
      name: e.info.name,
      tools: e.info.tools.map((x) => `T${toolNumber(e, x.number)} ${x.description}`).join(", "),
    }));

  const saveStep = async (prepared: Prepared, order: OrderLine[]) => {
    setDialog({ kind: "none" });
    const defaultPath = prepared.suggestedDir ? await join(prepared.suggestedDir, prepared.suggestedName) : prepared.suggestedName;
    const ext = prepared.suggestedName.split(".").pop() ?? "cnc";
    const path = await save({
      title: t.saveDialogTitle,
      defaultPath,
      filters: [
        { name: t.filterNc, extensions: [ext, ...NC_EXTENSIONS.filter((x) => x !== ext)] },
        { name: t.filterAll, extensions: ["*"] },
      ],
    });
    if (!path) return;
    setBusy({ kind: "save" });
    try {
      const saved = await api.saveMerge(path);
      const d = dirOf(saved.path);
      setLastDir(d);
      saveSetting("lastDir", d);
      setDialog({ kind: "done", prepared, saved, order });
    } catch (e) {
      setDialog({ kind: "error", message: errorMessage(e) });
    } finally {
      setBusy(null);
    }
  };

  const mergeAndSave = async () => {
    if (state.entries.length === 0 || busyRef.current) return;
    busyRef.current = true;
    setBusy({ kind: "merge" });
    const order = orderLines();
    let prepared: Prepared;
    try {
      prepared = await api.prepareMerge(
        state.entries.map((e) => e.id),
        state.entries.map(effectiveMap),
        state.restore,
        state.safeZ,
        stamp(),
      );
    } catch (e) {
      setDialog({ kind: "error", message: errorMessage(e) });
      return;
    } finally {
      busyRef.current = false;
      setBusy(null);
    }
    if (prepared.warnings.length > 0) setDialog({ kind: "warnings", prepared, order });
    else await saveStep(prepared, order);
  };

  const selIndex = state.entries.findIndex((e) => e.id === state.selected);
  const selEntry = selIndex >= 0 ? state.entries[selIndex] : null;

  const gridUsed = useMemo(() => {
    if (dialog.kind !== "grid") return new Set<number>();
    const s = new Set<number>();
    for (const e of state.entries)
      for (const x of e.info.tools) if (!(e.id === dialog.id && x.number === dialog.orig)) s.add(toolNumber(e, x.number));
    return s;
  }, [dialog, state.entries]);

  const ThemeIcon = theme === "dark" ? Moon : theme === "light" ? Sun : Monitor;
  const themeLabel = theme === "dark" ? t.themeDark : theme === "light" ? t.themeLight : t.themeSystem;

  return (
    <div className="flex h-full flex-col">
      <header className="flex min-h-16 items-center gap-4 border-b border-line bg-surface px-5">
        <img src={logo} alt="" className="size-10" draggable={false} />
        <div className="flex flex-col leading-tight">
          <span className="text-xl font-bold">MultiPass</span>
          <span className="text-sm text-muted">{t.appTagline}</span>
        </div>
        <div className="flex-1" />
        <div className="flex rounded-xl border border-line p-1" role="group" aria-label={t.language}>
          {(["de", "en"] as const).map((l) => (
            <button
              key={l}
              type="button"
              onClick={() => setLang(l)}
              aria-pressed={lang === l}
              className={cx(
                "min-h-9 min-w-12 rounded-lg px-3 text-sm font-semibold uppercase",
                lang === l ? "bg-primary text-primary-fg" : "text-muted hover:bg-surface-2",
              )}
            >
              {l}
            </button>
          ))}
        </div>
        <Button variant="ghost" icon={<Wrench className="size-5" />} onClick={() => setDialog({ kind: "inventory" })}>
          {t.inventoryButton}
        </Button>
        <Button variant="ghost" icon={<ThemeIcon className="size-5" />} onClick={cycleTheme} title={t.theme}>
          {themeLabel}
        </Button>
      </header>

      <main className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_420px] gap-4 p-4">
        <div className="flex min-h-0 flex-col gap-4">
          <Card
            title={t.filesTitle}
            aside={state.entries.length > 0 && <span className="text-sm text-muted">{t.filesCount(state.entries.length)}</span>}
            className="min-h-0 flex-[3] overflow-hidden"
          >
            <FileList
              entries={state.entries}
              selected={state.selected}
              dragOver={dragOver}
              onSelect={(id) => dispatch({ type: "select", id })}
              onReorder={(activeId, overId) => dispatch({ type: "reorder", activeId, overId })}
              onAdd={chooseFiles}
            />
          </Card>
          {state.entries.length > 0 && (
            <div className="flex min-h-0 flex-[2] flex-col">
              <Sequence entries={state.entries} />
            </div>
          )}
        </div>
        <div className="scroll-thin flex min-h-0 flex-col gap-4 overflow-y-auto pr-1 [&>*]:shrink-0">
          <DetailPanel
            entry={selEntry}
            position={selIndex + 1}
            count={state.entries.length}
            onTool={(orig, value) => selEntry && dispatch({ type: "setTool", id: selEntry.id, orig, value })}
            onPickTool={(orig, description) => selEntry && setDialog({ kind: "grid", id: selEntry.id, orig, description })}
            onMove={(delta) => selEntry && dispatch({ type: "move", id: selEntry.id, delta })}
            onRemove={() => selEntry && remove(selEntry.id)}
            inventory={inventory}
            onRemember={remember}
          />
          <OptionsPanel
            restore={state.restore}
            safeZ={state.safeZ}
            onRestore={(value) => dispatch({ type: "setRestore", value })}
            onSafeZ={(value) => dispatch({ type: "setSafeZ", value })}
          />
        </div>
      </main>

      <footer className="flex min-h-20 items-center gap-3 border-t border-line bg-surface px-5">
        <Button
          variant="danger"
          icon={<Trash2 className="size-5" />}
          onClick={() => setDialog({ kind: "confirmClear" })}
          disabled={state.entries.length === 0}
        >
          {t.removeAll}
        </Button>
        <Button icon={<RotateCcw className="size-5" />} onClick={() => dispatch({ type: "resetTools" })} disabled={!hasManual(state)}>
          {t.autoNumbers}
        </Button>
        <div className="flex-1" />
        <Button
          variant="primary"
          big
          icon={<Merge className="size-6" />}
          onClick={mergeAndSave}
          disabled={state.entries.length === 0 || busy !== null}
          className="min-w-80"
        >
          {t.mergeSave}
        </Button>
      </footer>

      {busy && (
        <div className="anim-fade fixed inset-0 z-[60] flex items-center justify-center bg-black/35">
          <div className="w-[420px] rounded-2xl border border-line bg-surface p-6 shadow-2xl">
            <div className="mb-4 text-lg font-semibold">
              {busy.kind === "load" ? t.loading(busy.done, busy.total) : busy.kind === "merge" ? t.merging : t.saving}
            </div>
            <div className="h-3 overflow-hidden rounded-full bg-surface-2">
              {busy.kind === "load" ? (
                <div
                  className="h-full rounded-full bg-primary transition-[width]"
                  style={{ width: `${Math.max(6, (busy.done / Math.max(1, busy.total)) * 100)}%` }}
                />
              ) : (
                <div className="h-full w-1/3 animate-pulse rounded-full bg-primary" />
              )}
            </div>
          </div>
        </div>
      )}

      <Modal
        open={dialog.kind === "confirmClear"}
        onClose={() => setDialog({ kind: "none" })}
        title={t.removeAllQuestion}
        icon={<Trash2 className="size-6 text-danger" />}
        closeLabel={t.close}
        actions={
          <>
            <Button onClick={() => setDialog({ kind: "none" })}>{t.cancel}</Button>
            <Button variant="danger" icon={<Trash2 className="size-5" />} onClick={clearAll}>
              {t.yesRemoveAll}
            </Button>
          </>
        }
      >
        <p className="text-muted">{t.removeAllHint}</p>
      </Modal>

      <Modal
        open={dialog.kind === "loadErrors"}
        onClose={() => setDialog({ kind: "none" })}
        title={t.loadErrorsTitle}
        icon={<XCircle className="size-6 text-danger" />}
        closeLabel={t.close}
        actions={<Button variant="primary" onClick={() => setDialog({ kind: "none" })}>{t.ok}</Button>}
      >
        {dialog.kind === "loadErrors" && (
          <div className="flex flex-col gap-3">
            {dialog.errors.map((e, i) => (
              <Notice key={i} tone="danger" icon={<XCircle className="size-5" />}>
                {errorText(t, e.error)}
              </Notice>
            ))}
            <p className="text-sm text-muted">{t.loadErrorsHint}</p>
          </div>
        )}
      </Modal>

      <Modal
        open={dialog.kind === "error"}
        onClose={() => setDialog({ kind: "none" })}
        title={t.errorTitle}
        icon={<XCircle className="size-6 text-danger" />}
        closeLabel={t.close}
        actions={<Button variant="primary" onClick={() => setDialog({ kind: "none" })}>{t.ok}</Button>}
      >
        {dialog.kind === "error" && <p>{dialog.message}</p>}
      </Modal>

      <Modal
        open={dialog.kind === "warnings"}
        onClose={() => setDialog({ kind: "none" })}
        title={t.warningsTitle}
        icon={<AlertTriangle className="size-6 text-warn" />}
        closeLabel={t.close}
        actions={
          <>
            <Button onClick={() => setDialog({ kind: "none" })}>{t.cancel}</Button>
            <Button
              variant="primary"
              onClick={() => dialog.kind === "warnings" && saveStep(dialog.prepared, dialog.order)}
            >
              {t.saveAnyway}
            </Button>
          </>
        }
      >
        {dialog.kind === "warnings" && (
          <div className="flex flex-col gap-3">
            <p>{t.warningsIntro}</p>
            {dialog.prepared.warnings.map((w, i) => (
              <Notice key={i} tone="warn" icon={<AlertTriangle className="size-5" />}>
                {warningText(t, w)}
              </Notice>
            ))}
          </div>
        )}
      </Modal>

      <Modal
        open={dialog.kind === "done"}
        onClose={() => setDialog({ kind: "none" })}
        title={t.doneTitle}
        icon={<CheckCircle2 className="size-6 text-ok" />}
        closeLabel={t.close}
        wide
        actions={
          <>
            <Button
              icon={<FolderOpen className="size-5" />}
              onClick={() =>
                dialog.kind === "done" &&
                revealItemInDir(dialog.saved.path).catch((e) => setDialog({ kind: "error", message: errorMessage(e) }))
              }
            >
              {t.openFolder}
            </Button>
            <Button variant="primary" onClick={() => setDialog({ kind: "none" })}>
              {t.ok}
            </Button>
          </>
        }
      >
        {dialog.kind === "done" && (
          <div className="flex flex-col gap-4">
            <div>
              <div className="text-sm text-muted">{t.savedTo}</div>
              <div className="font-medium break-all select-text">{dialog.saved.path}</div>
            </div>
            <div>
              <div className="mb-1 text-sm text-muted">{t.order}</div>
              <ol className="flex flex-col gap-1">
                {dialog.order.map((o) => (
                  <li key={o.position}>
                    <span className="font-medium">
                      {o.position}. {o.name}
                    </span>{" "}
                    <span className="text-muted">→ {o.tools}</span>
                  </li>
                ))}
              </ol>
            </div>
            {dialog.prepared.rapid && (
              <Notice tone="ok" icon={<CheckCircle2 className="size-5" />}>
                {t.rapidResult(
                  dialog.prepared.rapid.converted,
                  num(dialog.prepared.rapid.distanceMm / 1000, 1),
                  num(dialog.prepared.rapid.feedTimeMin, 0),
                )}
              </Notice>
            )}
            <Notice tone="info" icon={<Info className="size-5 text-primary" />}>
              <div className="mb-1 font-semibold">{t.makeraTitle}</div>
              <ul className="list-disc pl-5">
                <li>{t.makeraChange}</li>
                <li>{t.makeraProbe}</li>
              </ul>
            </Notice>
          </div>
        )}
      </Modal>

      <InventoryDialog
        open={dialog.kind === "inventory"}
        inventory={inventory}
        onDelete={(d) => updateInventory(forgetTool(inventory, d))}
        onRestoreDefaults={restoreDefaults}
        onClose={() => setDialog({ kind: "none" })}
      />

      <Modal
        open={dialog.kind === "inventoryConflict"}
        onClose={() => setDialog({ kind: "none" })}
        title={t.inventoryConflictTitle}
        icon={<AlertTriangle className="size-6 text-warn" />}
        closeLabel={t.close}
        actions={
          <>
            <Button onClick={() => setDialog({ kind: "none" })}>{t.cancel}</Button>
            <Button
              variant="primary"
              onClick={() => {
                if (dialog.kind === "inventoryConflict")
                  updateInventory(rememberTool(inventory, dialog.description, dialog.number));
                setDialog({ kind: "none" });
              }}
            >
              {t.replace}
            </Button>
          </>
        }
      >
        {dialog.kind === "inventoryConflict" && <p>{t.inventoryConflict(dialog.number, dialog.other.description)}</p>}
      </Modal>

      <NumberGrid
        open={dialog.kind === "grid"}
        description={dialog.kind === "grid" ? dialog.description : ""}
        current={
          dialog.kind === "grid"
            ? toolNumber(state.entries.find((e) => e.id === dialog.id) ?? state.entries[0], dialog.orig)
            : 0
        }
        used={gridUsed}
        onPick={(n) => {
          if (dialog.kind === "grid") dispatch({ type: "setTool", id: dialog.id, orig: dialog.orig, value: n });
          setDialog({ kind: "none" });
        }}
        onClose={() => setDialog({ kind: "none" })}
      />
    </div>
  );
}
