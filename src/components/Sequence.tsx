import { ArrowRight, Repeat } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { sequence, type Entry } from "../lib/state";

export function Sequence({ entries }: { entries: Entry[] }) {
  const { t } = useI18n();
  const steps = sequence(entries);
  return (
    <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-5 py-3">
      {steps.length === 0 ? (
        <p className="py-2 text-muted">{t.sequenceEmpty}</p>
      ) : (
        <ol className="flex flex-col gap-1.5">
          {steps.map((s, i) => (
            <li key={i} className="flex flex-wrap items-center gap-x-3 gap-y-1">
              <ArrowRight className="size-4 shrink-0 text-primary" />
              <span className="rounded-md bg-primary-soft px-2 py-0.5 text-sm font-semibold text-primary">{t.changeTo(s.tool)}</span>
              <span className="text-sm text-muted">{s.description}</span>
              {s.sameAsBefore && (
                <span className="inline-flex items-center gap-1 text-sm text-warn">
                  <Repeat className="size-4" />
                  {t.sameTool}
                </span>
              )}
              <span className="basis-full pl-7 font-medium">
                {s.position}. {s.name}
              </span>
            </li>
          ))}
        </ol>
      )}
    </div>
  );
}
