import { useEffect, useState } from "react";
import { fetchSessionTelemetry } from "../../transport/daemon-client";
import type { SessionTelemetrySummary, TelemetryRecord } from "../../transport/protocol";

interface TrafficViewProps {
  token: string;
  sessionId: string;
  emptyLabel: string;
  loadingLabel: string;
}

export function TrafficView({
  token,
  sessionId,
  emptyLabel,
  loadingLabel,
}: TrafficViewProps) {
  const [summary, setSummary] = useState<SessionTelemetrySummary | null>(null);
  const [records, setRecords] = useState<TelemetryRecord[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!token) {
      return;
    }
    let cancelled = false;
    setLoading(true);
    void fetchSessionTelemetry(token, sessionId)
      .then((res) => {
        if (!cancelled) {
          setSummary(res.summary);
          setRecords(res.records);
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [token, sessionId]);

  if (loading) {
    return <p className="text-xs text-fg-3">{loadingLabel}</p>;
  }

  return (
    <div className="space-y-3 text-xs">
      {summary ? (
        <div className="grid grid-cols-2 gap-2 rounded-md border border-line bg-app p-2.5 font-mono">
          <div>
            <span className="text-fg-3">Requests: </span>
            <span className="text-fg">{summary.request_count}</span>
          </div>
          <div>
            <span className="text-fg-3">Cost: </span>
            <span className="text-fg">${summary.estimated_cost_usd.toFixed(4)}</span>
          </div>
          <div>
            <span className="text-fg-3">In: </span>
            <span className="text-fg">{summary.prompt_tokens}</span>
          </div>
          <div>
            <span className="text-fg-3">Out: </span>
            <span className="text-fg">{summary.completion_tokens}</span>
          </div>
        </div>
      ) : null}

      {records.length === 0 ? (
        <p className="text-fg-3">{emptyLabel}</p>
      ) : (
        <ul className="space-y-1.5">
          {records.slice(-40).reverse().map((rec, i) => (
            <li
              key={rec.id ?? i}
              className="rounded-md border border-line bg-app px-2.5 py-2 font-mono text-[11px]"
            >
              <div className="flex items-center justify-between">
                <span className="text-fg">{rec.model ?? rec.provider}</span>
                <span className={rec.error_kind ? "text-danger" : "text-done"}>
                  {rec.http_status ?? rec.error_kind ?? "ok"}
                </span>
              </div>
              <div className="mt-0.5 flex items-center justify-between text-fg-3">
                <span>
                  {rec.prompt_tokens}+{rec.completion_tokens} tok
                </span>
                <span>
                  {rec.latency_ms ?? 0}ms · ${rec.estimated_cost_usd.toFixed(4)}
                </span>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default TrafficView;
