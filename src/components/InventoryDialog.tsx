import { RotateCcw, Trash2, Wrench } from "lucide-react";
import { useState } from "react";
import type { InventoryEntry } from "../lib/api";
import { useI18n } from "../lib/i18n";
import { Button, Modal } from "./ui";

interface Props {
  open: boolean;
  inventory: InventoryEntry[];
  onDelete: (description: string) => void;
  onRestoreDefaults: () => void;
  onClose: () => void;
}

export function InventoryDialog({ open, inventory, onDelete, onRestoreDefaults, onClose }: Props) {
  const { t } = useI18n();
  const [armed, setArmed] = useState<string | null>(null);
  const close = () => {
    setArmed(null);
    onClose();
  };
  return (
    <Modal
      open={open}
      onClose={close}
      title={t.inventoryTitle}
      icon={<Wrench className="size-6 text-primary" />}
      closeLabel={t.close}
      wide
      actions={
        <>
          <Button icon={<RotateCcw className="size-5" />} onClick={onRestoreDefaults}>
            {t.restoreDefaults}
          </Button>
          <Button variant="primary" onClick={close}>
            {t.close}
          </Button>
        </>
      }
    >
      <p className="mb-4 text-muted">{t.inventoryIntro}</p>
      {inventory.length === 0 ? (
        <p className="rounded-xl bg-surface-2 p-4">{t.inventoryEmpty}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {inventory.map((e) => (
            <li key={e.description} className="flex min-h-14 items-center gap-4 rounded-xl border border-line bg-surface-2 px-4 py-2">
              <span className="min-w-14 rounded-lg bg-primary px-2.5 py-1 text-center font-bold text-primary-fg tabular-nums">T{e.number}</span>
              <span className="min-w-0 flex-1 font-medium break-words">{e.description}</span>
              {armed === e.description ? (
                <Button
                  variant="danger"
                  icon={<Trash2 className="size-5" />}
                  onClick={() => {
                    onDelete(e.description);
                    setArmed(null);
                  }}
                >
                  {t.reallyDelete}
                </Button>
              ) : (
                <Button variant="ghost" icon={<Trash2 className="size-5" />} onClick={() => setArmed(e.description)}>
                  {t.deleteEntry}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
    </Modal>
  );
}
