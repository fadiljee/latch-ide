import { useMemo, useState } from "react";
import type { SessionDiff } from "../../lib/api/types";

interface DiffViewerProps {
  diff: SessionDiff;
  onClose: () => void;
  title: string;
  closeLabel: string;
}

interface SplitRow {
  left: string;
  right: string;
  kind: "context" | "add" | "del" | "meta";
}

function parseSplitRows(patch: string): SplitRow[] {
  const rows: SplitRow[] = [];
  for (const line of patch.split("\n")) {
    if (line.startsWith("+++") || line.startsWith("---") || line.startsWith("@@") || line.startsWith("diff ")) {
      rows.push({ left: line, right: line, kind: "meta" });
    } else if (line.startsWith("+")) {
      rows.push({ left: "", right: line, kind: "add" });
    } else if (line.startsWith("-")) {
      rows.push({ left: line, right: "", kind: "del" });
    } else {
      rows.push({ left: line, right: line, kind: "context" });
    }
  }
  return rows;
}

export function DiffViewer({ diff, onClose, title, closeLabel }: DiffViewerProps) {
  const [mode, setMode] = useState<"split" | "unified">("split");
  const rows = useMemo(() => parseSplitRows(diff.patch), [diff.patch]);

  return (
    <section
      aria-label={title}
      className="mt-6 rounded-lg border border-slate-800 bg-slate-900/80 p-4"
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-slate-100">{title}</h3>
          <p className="text-xs text-slate-400">
            base: {diff.base_branch} · {diff.files.length} file(s)
            {diff.truncated ? " · truncated" : ""}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setMode(mode === "split" ? "unified" : "split")}
            className="hidden rounded border border-slate-700 px-2.5 py-1 text-xs hover:bg-slate-800 sm:inline-flex"
          >
            {mode === "split" ? "Unified" : "Side-by-Side"}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="rounded border border-slate-700 px-2.5 py-1 text-xs hover:bg-slate-800"
          >
            {closeLabel}
          </button>
        </div>
      </div>

      {diff.files.length > 0 ? (
        <ul className="mb-3 flex flex-wrap gap-2 text-xs">
          {diff.files.map((f) => (
            <li
              key={f.path}
              className="rounded bg-slate-800/80 px-2 py-1 font-mono text-slate-200"
            >
              {f.path}{" "}
              <span className="text-emerald-400">+{f.additions}</span>{" "}
              <span className="text-rose-400">-{f.deletions}</span>
            </li>
          ))}
        </ul>
      ) : null}

      <div className="max-h-96 overflow-auto rounded border border-slate-800 bg-slate-950 font-mono text-xs">
        {mode === "unified" ? (
          <pre className="p-3 whitespace-pre-wrap break-words text-slate-200">{diff.patch || "(no diff)"}</pre>
        ) : (
          <div className="divide-y divide-slate-900">
            {rows.map((row, idx) => (
              <div
                key={idx}
                className="grid grid-cols-1 sm:grid-cols-2"
              >
                <div
                  className={`px-2 py-0.5 whitespace-pre-wrap break-words ${
                    row.kind === "del"
                      ? "bg-rose-950/50 text-rose-200"
                      : row.kind === "meta"
                        ? "bg-slate-900 text-cyan-300"
                        : "text-slate-300"
                  }`}
                >
                  {row.left}
                </div>
                <div
                  className={`border-t border-slate-900 px-2 py-0.5 whitespace-pre-wrap break-words sm:border-t-0 sm:border-l ${
                    row.kind === "add"
                      ? "bg-emerald-950/50 text-emerald-200"
                      : row.kind === "meta"
                        ? "bg-slate-900 text-cyan-300"
                        : "text-slate-300"
                  }`}
                >
                  {row.right}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
