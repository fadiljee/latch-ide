import { useNeedsYouCount, useTotalCostUsd } from "../state/selectors";
import { useNotificationsStore } from "../state/notifications";

interface UsagePillProps {
  onClick?: () => void;
}

export function UsagePill({ onClick }: UsagePillProps) {
  const totalCost = useTotalCostUsd();
  const needsYou = useNeedsYouCount();
  const approvalsCount = useNotificationsStore((s) => s.approvals.length);
  const alerts = needsYou + approvalsCount;

  return (
    <button
      type="button"
      onClick={onClick}
      className="inline-flex items-center gap-2 rounded-md border border-line bg-raised px-2.5 py-1 font-mono text-[11px] text-fg-2 hover:bg-hover hover:text-fg"
    >
      <span>${totalCost.toFixed(3)}</span>
      {alerts > 0 ? (
        <span className="rounded-sm bg-needs/20 px-1.5 text-needs">{alerts}</span>
      ) : null}
    </button>
  );
}
