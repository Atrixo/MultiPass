import { AlertTriangle } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { RAPID_SPEED_MAX, RAPID_SPEED_MIN, RAPID_SPEED_STEP } from "../lib/format";
import { SAFE_Z_MAX, SAFE_Z_MIN, SAFE_Z_STEP } from "../lib/state";
import { Card, Notice, Stepper, Switch } from "./ui";

interface Props {
  restore: boolean;
  safeZ: number;
  onRestore: (v: boolean) => void;
  onSafeZ: (v: number) => void;
  rapidSpeed: number;
  onRapidSpeed: (v: number) => void;
  fanPause: boolean;
  onFanPause: (v: boolean) => void;
}

export function OptionsPanel({ restore, safeZ, onRestore, onSafeZ, rapidSpeed, onRapidSpeed, fanPause, onFanPause }: Props) {
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
        <div className="flex items-start gap-4 border-t border-line pt-4">
          <Switch checked={fanPause} onChange={onFanPause} label={t.fanLabel} />
          <span className="flex cursor-pointer flex-col" onClick={() => onFanPause(!fanPause)}>
            <span className="font-semibold">{t.fanLabel}</span>
            <span className="text-sm text-muted">{t.fanHint}</span>
          </span>
        </div>
        <div className="border-t border-line pt-4">
          <div className="mb-2 text-sm text-muted">{t.rapidSpeed}</div>
          <Stepper
            value={`${num(rapidSpeed, 0)} mm/min`}
            onMinus={() => onRapidSpeed(rapidSpeed - RAPID_SPEED_STEP)}
            onPlus={() => onRapidSpeed(rapidSpeed + RAPID_SPEED_STEP)}
            minusDisabled={rapidSpeed <= RAPID_SPEED_MIN}
            plusDisabled={rapidSpeed >= RAPID_SPEED_MAX}
            minusLabel={t.less}
            plusLabel={t.more}
          />
        </div>
      </div>
    </Card>
  );
}
