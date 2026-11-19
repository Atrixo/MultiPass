import * as Dialog from "@radix-ui/react-dialog";
import { Minus, Plus, X } from "lucide-react";
import type { ButtonHTMLAttributes, ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const variants: Record<Variant, string> = {
  primary: "bg-primary text-primary-fg hover:bg-primary-hover shadow-sm",
  secondary: "bg-surface text-ink border border-line hover:bg-surface-2",
  ghost: "bg-transparent text-ink hover:bg-surface-2",
  danger: "bg-surface text-danger border border-line hover:bg-danger-soft",
};

export function cx(...c: (string | false | null | undefined)[]): string {
  return c.filter(Boolean).join(" ");
}

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  icon?: ReactNode;
  big?: boolean;
}

export function Button({ variant = "secondary", icon, big, className, children, ...rest }: ButtonProps) {
  return (
    <button
      type="button"
      className={cx(
        "inline-flex items-center justify-center gap-2 rounded-xl font-medium transition-colors",
        "disabled:opacity-45 disabled:shadow-none",
        big ? "min-h-14 px-6 text-lg" : "min-h-11 px-4 text-[15px]",
        variants[variant],
        className,
      )}
      {...rest}
    >
      {icon}
      {children}
    </button>
  );
}

export function IconButton({
  label,
  className,
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={cx(
        "inline-flex size-11 items-center justify-center rounded-xl border border-line bg-surface text-ink",
        "transition-colors hover:bg-surface-2 disabled:opacity-40",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export function Card({ title, aside, children, className }: { title?: ReactNode; aside?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <section className={cx("flex min-h-0 flex-col rounded-2xl border border-line bg-surface shadow-sm", className)}>
      {title && (
        <header className="flex min-h-14 items-center justify-between gap-3 border-b border-line px-5">
          <h2 className="text-[15px] font-semibold tracking-wide text-muted uppercase">{title}</h2>
          {aside}
        </header>
      )}
      {children}
    </section>
  );
}

export function Switch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={cx(
        "relative inline-flex h-9 w-16 shrink-0 items-center rounded-full border-2 transition-colors",
        checked ? "border-primary bg-primary" : "border-line bg-surface-2",
      )}
    >
      <span
        className={cx(
          "inline-block size-7 rounded-full bg-white shadow transition-transform",
          checked ? "translate-x-7" : "translate-x-0.5",
        )}
      />
    </button>
  );
}

interface StepperProps {
  value: ReactNode;
  onMinus: () => void;
  onPlus: () => void;
  minusDisabled?: boolean;
  plusDisabled?: boolean;
  minusLabel: string;
  plusLabel: string;
  onValueClick?: () => void;
  valueLabel?: string;
  disabled?: boolean;
}

export function Stepper(p: StepperProps) {
  const side = "inline-flex size-12 items-center justify-center rounded-xl border border-line bg-surface hover:bg-surface-2 disabled:opacity-40";
  return (
    <div className="inline-flex items-center gap-2">
      <button type="button" className={side} aria-label={p.minusLabel} title={p.minusLabel} onClick={p.onMinus} disabled={p.disabled || p.minusDisabled}>
        <Minus className="size-5" />
      </button>
      {p.onValueClick ? (
        <button
          type="button"
          onClick={p.onValueClick}
          disabled={p.disabled}
          title={p.valueLabel}
          aria-label={p.valueLabel}
          className="min-h-12 min-w-20 rounded-xl border-2 border-primary bg-primary-soft px-3 text-lg font-semibold tabular-nums hover:brightness-95 disabled:opacity-40"
        >
          {p.value}
        </button>
      ) : (
        <span className={cx("min-w-24 text-center text-lg font-semibold tabular-nums", p.disabled && "opacity-40")}>{p.value}</span>
      )}
      <button type="button" className={side} aria-label={p.plusLabel} title={p.plusLabel} onClick={p.onPlus} disabled={p.disabled || p.plusDisabled}>
        <Plus className="size-5" />
      </button>
    </div>
  );
}

interface ModalProps {
  open: boolean;
  onClose?: () => void;
  title: ReactNode;
  icon?: ReactNode;
  children: ReactNode;
  actions?: ReactNode;
  wide?: boolean;
  closeLabel: string;
}

export function Modal({ open, onClose, title, icon, children, actions, wide, closeLabel }: ModalProps) {
  return (
    <Dialog.Root open={open} onOpenChange={(o) => !o && onClose?.()}>
      <Dialog.Portal>
        <Dialog.Overlay className="anim-fade fixed inset-0 z-40 bg-black/45" />
        <Dialog.Content
          aria-describedby={undefined}
          onInteractOutside={(e) => !onClose && e.preventDefault()}
          onEscapeKeyDown={(e) => !onClose && e.preventDefault()}
          className={cx(
            "anim-pop fixed top-1/2 left-1/2 z-50 flex max-h-[88vh] w-[92vw] -translate-x-1/2 -translate-y-1/2 flex-col",
            "rounded-2xl border border-line bg-surface text-ink shadow-2xl",
            wide ? "max-w-3xl" : "max-w-xl",
          )}
        >
          <div className="flex items-center gap-3 border-b border-line px-6 py-4">
            {icon}
            <Dialog.Title className="flex-1 text-xl font-semibold">{title}</Dialog.Title>
            {onClose && (
              <Dialog.Close asChild>
                <IconButton label={closeLabel} className="border-transparent">
                  <X className="size-5" />
                </IconButton>
              </Dialog.Close>
            )}
          </div>
          <div className="scroll-thin min-h-0 flex-1 overflow-y-auto px-6 py-5">{children}</div>
          {actions && <div className="flex flex-wrap justify-end gap-3 border-t border-line px-6 py-4">{actions}</div>}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export function Notice({ tone, icon, children }: { tone: "warn" | "danger" | "ok" | "info"; icon?: ReactNode; children: ReactNode }) {
  const tones = {
    warn: "border-warn-line bg-warn-soft text-warn",
    danger: "border-danger/40 bg-danger-soft text-danger",
    ok: "border-ok/40 bg-ok-soft text-ok",
    info: "border-line bg-primary-soft text-ink",
  };
  return (
    <div className={cx("flex gap-3 rounded-xl border px-4 py-3 text-[15px]", tones[tone])}>
      {icon && <span className="mt-0.5 shrink-0">{icon}</span>}
      <div className="min-w-0">{children}</div>
    </div>
  );
}
