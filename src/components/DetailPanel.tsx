import { AlertTriangle, ArrowDown, ArrowUp, BookmarkCheck, BookmarkPlus, Trash2 } from "lucide-react";
import type { InventoryEntry } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { TOOL_MAX, TOOL_MIN, findInventory, toolNumber, type Entry } from "../lib/state";
import { Button, Card, Notice, Stepper } from "./ui";

interface Props {
  entry: Entry | null;
  position: number;
  count: number;
  onTool: (orig: number, value: number) => void;
  onPickTool: (orig: number, description: string) => void;
  onMove: (delta: number) => void;
  onRemove: () => void;
  inventory: InventoryEntry[];
  onRemember: (description: string, number: number) => void;
}

export function DetailPanel({ entry, position, count, onTool, onPickTool, onMove, onRemove, inventory, onRemember }: Props) {
  const { t, num } = useI18n();
  if (!entry) {
    return (
      <Card title={t.selected}>
        <p className="p-5 text-muted">{t.sequenceEmpty}</p>
      </Card>
    );
  }
  const info = entry.info;
  return (
    <Card title={t.selected} aside={<span className="text-sm text-muted">{t.fileOf(position, count)}</span>}>
      <div className="flex flex-col gap-4 p-5">
        <div className="truncate text-lg font-semibold" title={info.path}>
          {info.name}
        </div>
        {info.tools.map((tool) => {
          const n = toolNumber(entry, tool.number);
          const manual = entry.manual?.[String(tool.number)] !== undefined;
          const inv = findInventory(inventory, tool.description);
          return (
            <div key={tool.number} className="flex flex-col gap-2 rounded-xl bg-surface-2 p-4">
              <div className="text-sm text-muted">{t.tool}</div>
              <div className="font-medium">{tool.description}</div>
              <div className="mt-1 flex items-center justify-between gap-3">
                <span className="text-sm whitespace-nowrap text-muted">
                  {t.toolNumber}
                  {manual && <span className="ml-2 rounded-md bg-primary-soft px-1.5 py-0.5 text-xs font-semibold text-primary">{t.manual}</span>}
                </span>
                <Stepper
                  value={`T${n}`}
                  onMinus={() => onTool(tool.number, n - 1)}
                  onPlus={() => onTool(tool.number, n + 1)}
                  minusDisabled={n <= TOOL_MIN}
                  plusDisabled={n >= TOOL_MAX}
                  minusLabel={t.less}
                  plusLabel={t.more}
                  onValueClick={() => onPickTool(tool.number, tool.description)}
                  valueLabel={t.chooseNumber}
                />
              </div>
              {inv && inv.number === n ? (
                <div className="mt-1 flex min-h-11 items-center gap-2 text-sm font-medium text-ok">
                  <BookmarkCheck className="size-5" />
                  {t.inInventory(n)}
                </div>
              ) : (
                <Button
                  icon={<BookmarkPlus className="size-5" />}
                  onClick={() => onRemember(tool.description, n)}
                  className="mt-1"
                >
                  {inv ? t.rememberToolWas(inv.number) : t.rememberTool}
                </Button>
              )}
            </div>
          );
        })}
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-[15px]">
          <dt className="text-muted">{t.deepest}</dt>
          <dd className="tabular-nums">{info.zmin === null ? t.unknown : `${num(info.zmin)} mm`}</dd>
          <dt className="text-muted">{t.zOrigin}</dt>
          <dd>{info.zorigin ?? t.unknown}</dd>
          <dt className="text-muted">{t.lines}</dt>
          <dd className="tabular-nums">{num(info.lines, 0)}</dd>
        </dl>
        {!info.stockTop && (
          <Notice tone="warn" icon={<AlertTriangle className="size-5" />}>
            {t.notStockTop}
          </Notice>
        )}
        {info.incremental && (
          <Notice tone="warn" icon={<AlertTriangle className="size-5" />}>
            {t.incremental}
          </Notice>
        )}
        <div className="grid grid-cols-2 gap-2">
          <Button icon={<ArrowUp className="size-5" />} onClick={() => onMove(-1)} disabled={position <= 1}>
            {t.moveUp}
          </Button>
          <Button icon={<ArrowDown className="size-5" />} onClick={() => onMove(1)} disabled={position >= count}>
            {t.moveDown}
          </Button>
          <Button variant="danger" icon={<Trash2 className="size-5" />} onClick={onRemove} className="col-span-2">
            {t.remove}
          </Button>
        </div>
      </div>
    </Card>
  );
}
