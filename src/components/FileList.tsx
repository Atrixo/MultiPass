import { DndContext, PointerSensor, closestCenter, useSensor, useSensors, type DragEndEvent } from "@dnd-kit/core";
import { SortableContext, useSortable, verticalListSortingStrategy } from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
import { restrictToVerticalAxis } from "./dndModifiers";
import { AlertTriangle, FilePlus2, GripVertical, UploadCloud } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { toolNumber, type Entry } from "../lib/state";
import { Button, cx } from "./ui";

interface Props {
  entries: Entry[];
  selected: number | null;
  dragOver: boolean;
  onSelect: (id: number) => void;
  onReorder: (activeId: number, overId: number) => void;
  onAdd: () => void;
  runtimes: Record<number, string>;
}

export function FileList({ entries, selected, dragOver, onSelect, onReorder, onAdd, runtimes }: Props) {
  const { t } = useI18n();
  const sensors = useSensors(useSensor(PointerSensor, { activationConstraint: { distance: 4 } }));

  if (entries.length === 0) {
    return (
      <button
        type="button"
        onClick={onAdd}
        className={cx(
          "m-5 flex min-h-0 flex-1 flex-col items-center justify-center gap-4 rounded-2xl border-[3px] border-dashed p-6 text-center transition-colors",
          dragOver ? "border-primary bg-primary-soft" : "border-line bg-surface-2 hover:border-primary hover:bg-primary-soft",
        )}
      >
        <UploadCloud className="size-16 text-primary" strokeWidth={1.5} />
        <span className="text-2xl font-semibold">{dragOver ? t.dropNow : t.dropTitle}</span>
        {!dragOver && (
          <>
            <span className="text-muted">{t.dropOr}</span>
            <span className="inline-flex min-h-12 items-center gap-2 rounded-xl bg-primary px-6 text-lg font-medium text-primary-fg">
              <FilePlus2 className="size-5" />
              {t.dropClick}
            </span>
            <span className="text-sm text-muted">{t.dropHint}</span>
          </>
        )}
      </button>
    );
  }

  const end = (e: DragEndEvent) => {
    if (e.over && e.active.id !== e.over.id) onReorder(Number(e.active.id), Number(e.over.id));
  };

  return (
    <div className={cx("relative flex min-h-0 flex-1 flex-col", dragOver && "bg-primary-soft")}>
      <div className="scroll-thin min-h-0 flex-1 overflow-y-auto p-3">
        <DndContext sensors={sensors} collisionDetection={closestCenter} modifiers={[restrictToVerticalAxis]} onDragEnd={end}>
          <SortableContext items={entries.map((e) => e.id)} strategy={verticalListSortingStrategy}>
            <ol className="flex flex-col gap-2">
              {entries.map((e, i) => (
                <Row
                  key={e.id}
                  entry={e}
                  position={i + 1}
                  active={e.id === selected}
                  runtime={runtimes[e.id]}
                  onSelect={() => onSelect(e.id)}
                />
              ))}
            </ol>
          </SortableContext>
        </DndContext>
      </div>
      <div className="border-t border-line p-3">
        <Button variant={dragOver ? "primary" : "secondary"} icon={<FilePlus2 className="size-5" />} onClick={onAdd} className="w-full">
          {dragOver ? t.dropNow : t.addFiles}
        </Button>
      </div>
    </div>
  );
}

interface RowProps {
  entry: Entry;
  position: number;
  active: boolean;
  runtime?: string;
  onSelect: () => void;
}

function Row({ entry, position, active, runtime, onSelect }: RowProps) {
  const { t, num } = useI18n();
  const { attributes, listeners, setNodeRef, setActivatorNodeRef, transform, transition, isDragging } = useSortable({ id: entry.id });
  const info = entry.info;
  const warn = !info.stockTop || info.incremental;
  return (
    <li
      ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition }}
      className={cx(
        "flex items-stretch rounded-xl border-2 bg-surface transition-colors",
        active ? "border-primary bg-primary-soft" : "border-line hover:border-primary/60",
        isDragging && "relative z-10 shadow-xl",
      )}
    >
      <button
        type="button"
        ref={setActivatorNodeRef}
        {...attributes}
        {...listeners}
        aria-label={t.dragHandle}
        title={t.dragHandle}
        className="flex w-11 shrink-0 cursor-grab touch-none items-center justify-center rounded-l-xl text-muted hover:bg-surface-2 active:cursor-grabbing"
      >
        <GripVertical className="size-6" />
      </button>
      <button type="button" onClick={onSelect} className="flex min-h-16 min-w-0 flex-1 items-center gap-4 py-2 pr-4 text-left">
        <span className="flex size-9 shrink-0 items-center justify-center rounded-full bg-surface-2 text-base font-semibold tabular-nums">
          {position}
        </span>
        <span className="min-w-0 flex-1">
          <span className="block truncate font-semibold">{info.name}</span>
          <span className="block truncate text-sm text-muted">
            {info.tools.map((x) => x.description).join(" · ")}
            {runtime && <span className="ml-2 whitespace-nowrap">· {t.about(runtime)}</span>}
          </span>
        </span>
        {warn && <AlertTriangle className="size-5 shrink-0 text-warn" aria-label={t.notStockTop} />}
        <span className="flex shrink-0 flex-col items-end gap-1">
          <span className="flex gap-1">
            {info.tools.map((x) => (
              <span key={x.number} className="rounded-lg bg-primary px-2.5 py-0.5 text-sm font-bold text-primary-fg tabular-nums">
                T{toolNumber(entry, x.number)}
              </span>
            ))}
          </span>
          {info.zmin !== null && <span className="text-sm text-muted tabular-nums">{num(info.zmin)} mm</span>}
        </span>
      </button>
    </li>
  );
}
