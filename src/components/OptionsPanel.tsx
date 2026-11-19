import { AlertTriangle } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { SAFE_Z_MAX, SAFE_Z_MIN, SAFE_Z_STEP } from "../lib/state";
import { Card, Notice, Stepper, Switch } from "./ui";

interface Props {
  restore: boolean;
  safeZ: number;
  onRestore: (v: boolean) => void;
  onSafeZ: (v: number) => void;
}

export function OptionsPanel({ restore, safeZ, onRestore, onSafeZ }: Props) {
  const { t, num } = useI18n();
  return (
    <Card title={t.optionsTitle}>
      <div className="flex flex-col gap-4 p-5">
        <div className="flex items-start gap-4">
          <Switch checked={restore} onChange={onRestore} label={t.rapidLabel} />
          <span className="flex cursor-pointer flex-col" onClick={() => onRestore(!restore)}>
            <span className="font-semibold">{t.rapidLabel}</span>
            <span className="text-sm text-muted">{t.rapidHint}</span>
          </span>
        </div>
        <div className={restore ? "" : "opacity-50"}>
          <div className="mb-2 text-sm text-muted">{t.safeZ}</div>
          <Stepper
            value={`${num(safeZ, 1, 1)} mm`}
            onMinus={() => onSafeZ(safeZ - SAFE_Z_STEP)}
            onPlus={() => onSafeZ(safeZ + SAFE_Z_STEP)}
            minusDisabled={safeZ <= SAFE_Z_MIN}
            plusDisabled={safeZ >= SAFE_Z_MAX}
            minusLabel={t.less}
            plusLabel={t.more}
            disabled={!restore}
          />
        </div>
        {restore && (
          <Notice tone="warn" icon={<AlertTriangle className="size-5" />}>
            {t.clampWarning}
          </Notice>
        )}
      </div>
    </Card>
  );
}
