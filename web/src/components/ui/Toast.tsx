import { useNotificationsStore } from "../../state/notifications";
import { cn } from "../../lib/cn";

export function ToastList() {
  const toasts = useNotificationsStore((s) => s.toasts);
  const dismissToast = useNotificationsStore((s) => s.dismissToast);

  if (toasts.length === 0) {
    return null;
  }

  return (
    <div
      aria-live="polite"
      className="fixed right-4 bottom-9 z-50 flex max-w-sm flex-col gap-2"
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role="status"
          className={cn(
            "flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-xs shadow-[var(--panel-shadow)]",
            toast.tone === "danger"
              ? "border-danger/60 bg-surface text-danger"
              : toast.tone === "success"
                ? "border-done/60 bg-surface text-done"
                : "border-line-strong bg-surface text-fg",
          )}
        >
          <span>{toast.message}</span>
          <button
            type="button"
            onClick={() => dismissToast(toast.id)}
            aria-label="Dismiss notification"
            className="text-fg-3 hover:text-fg"
          >
            x
          </button>
        </div>
      ))}
    </div>
  );
}
