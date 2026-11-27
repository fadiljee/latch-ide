import { useState } from "react";
import { Button } from "../../components/ui/Button";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useNotificationsStore } from "../../state/notifications";
import { useTotalCostUsd } from "../../state/selectors";
import { useUsageStore } from "../../state/usage";
import { triggerGc } from "../../transport/daemon-client";

interface UsagePageProps {
  labels: Catalog["pages"];
}

export function UsagePage({ labels }: UsagePageProps) {
  const token = useHostsStore((s) => s.token);
  const totalCost = useTotalCostUsd();
  const bySessionId = useUsageStore((s) => s.bySessionId);
  const gcReport = useUsageStore((s) => s.gcReport);
  const setGcReport = useUsageStore((s) => s.setGcReport);
  const pushToast = useNotificationsStore((s) => s.pushToast);
  const [busy, setBusy] = useState(false);

  const runGc = async (dryRun: boolean) => {
    if (!token) {
      return;
    }
    setBusy(true);
    try {
      const report = await triggerGc(token, dryRun);
      setGcReport(report);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "GC failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex-1 space-y-6 overflow-y-auto p-5 text-xs">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold text-fg">{labels.usageHeading}</h2>
          <p className="mt-0.5 text-fg-3">{labels.usageSubtitle}</p>
        </div>
        <div className="flex items-center gap-2">
          <Button disabled={busy || !token} onClick={() => void runGc(true)}>
            {labels.gcDryRun}
          </Button>
          <Button variant="primary" disabled={busy || !token} onClick={() => void runGc(false)}>
            {labels.gcApply}
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <div className="rounded-lg border border-line bg-surface p-4">
          <p className="text-fg-3">Recorded Cost</p>
          <p className="mt-1 font-mono text-lg font-semibold text-fg">
            ${totalCost.toFixed(4)}
          </p>
        </div>
        <div className="rounded-lg border border-line bg-surface p-4">
          <p className="text-fg-3">Tracked Sessions</p>
          <p className="mt-1 font-mono text-lg font-semibold text-fg">
            {Object.keys(bySessionId).length}
          </p>
        </div>
      </div>

      {gcReport ? (
        <div className="rounded-lg border border-line bg-surface p-4 font-mono text-fg-2">
          <p className="mb-2 font-sans text-xs font-semibold text-fg">
            GC Report ({gcReport.dry_run ? "dry-run" : "applied"})
          </p>
          <p>
            telemetry={gcReport.telemetry_deleted} · audit={gcReport.audit_deleted} · tmp_dirs=
            {gcReport.tmp_dirs_removed} · logs={gcReport.log_files_removed} · bytes=
            {gcReport.bytes_freed}
          </p>
        </div>
      ) : null}
    </div>
  );
}
