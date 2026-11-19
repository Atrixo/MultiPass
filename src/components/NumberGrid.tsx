import { useI18n } from "../lib/i18n";
import { TOOL_MAX, TOOL_MIN } from "../lib/state";
import { Modal, cx } from "./ui";

interface Props {
  open: boolean;
  description: string;
  current: number;
  used: Set<number>;
  onPick: (n: number) => void;
  onClose: () => void;
}

export function NumberGrid({ open, description, current, used, onPick, onClose }: Props) {
  const { t } = useI18n();
  const numbers = Array.from({ length: TOOL_MAX - TOOL_MIN + 1 }, (_, i) => i + TOOL_MIN);
  return (
    <Modal open={open} onClose={onClose} title={t.chooseNumberFor(description)} closeLabel={t.close} wide>
      <div className="grid grid-cols-10 gap-1.5">
        {numbers.map((n) => (
          <button
            key={n}
            type="button"
            onClick={() => onPick(n)}
            className={cx(
              "relative flex h-11 items-center justify-center rounded-lg border text-base font-semibold tabular-nums transition-colors",
              n === current ? "border-primary bg-primary text-primary-fg" : "border-line bg-surface hover:border-primary hover:bg-primary-soft",
            )}
          >
            {n}
            {used.has(n) && n !== current && (
              <span className="absolute top-1 right-1 size-2 rounded-full bg-warn" title={t.usedElsewhere} />
            )}
          </button>
        ))}
      </div>
      <p className="mt-3 flex items-center gap-2 text-sm text-muted">
        <span className="inline-block size-2 rounded-full bg-warn" /> {t.usedElsewhere}
      </p>
    </Modal>
  );
}
