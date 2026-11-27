import { useEffect, useRef, type ReactNode } from "react";
import { cn } from "../../lib/cn";

interface DialogProps {
  open: boolean;
  title: string;
  titleId: string;
  onClose: () => void;
  children: ReactNode;
  className?: string;
}

export function Dialog({ open, title, titleId, onClose, children, className }: DialogProps) {
  const closeRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!open) {
      return;
    }
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    closeRef.current?.focus();

    const onKeyDown = (ev: KeyboardEvent) => {
      if (ev.key === "Escape") {
        ev.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      previous?.focus();
    };
  }, [open, onClose]);

  if (!open) {
    return null;
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      className="fixed inset-0 z-50 flex items-center justify-center bg-app/80 p-4"
    >
      <div
        className={cn(
          "w-full max-w-lg rounded-xl border border-line-strong bg-surface p-5 shadow-[var(--panel-shadow)]",
          className,
        )}
      >
        <div className="mb-4 flex items-center justify-between gap-2">
          <h2 id={titleId} className="text-sm font-semibold text-fg">
            {title}
          </h2>
          <button
            ref={closeRef}
            type="button"
            aria-label="Close"
            onClick={onClose}
            className="rounded-sm border border-line px-2 py-0.5 text-xs text-fg-2 hover:bg-hover hover:text-fg"
          >
            Esc
          </button>
        </div>
        {children}
      </div>
    </div>
  );
}
