import { useMemo, useState } from "react";
import { Button } from "../../components/ui/Button";
import type { SessionDiff } from "../../transport/protocol";

interface DiffViewProps {
  diff: SessionDiff;
  onClose?: () => void;
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
    if (
      line.startsWith("+++") ||
      line.startsWith("---") ||
      line.startsWith("@@") ||
      line.startsWith("diff ")
    ) {
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

export function DiffView({ diff, onClose, title, closeLabel }: DiffViewProps) {
  const [mode, setMode] = useState<"split" | "unified">("split");
  const rows = useMemo(() => parseSplitRows(diff.patch), [diff.patch]);

  return (
    <section
      aria-label={title}
      className="flex min-h-0 flex-1 flex-col overflow-hidden bg-surface p-4"
    >
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <div>
          <h3 className="text-sm font-semibold text-fg">{title}</h3>
          <p className="font-mono text-xs text-fg-3">
            base: {diff.base_branch} · {diff.files.length} file(s)
            {diff.truncated ? " · truncated" : ""}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            className="hidden sm:inline-flex"
            onClick={() => setMode(mode === "split" ? "unified" : "split")}
          >
            {mode === "split" ? "Unified" : "Side-by-Side"}
          </Button>
          {onClose ? (
            <Button size="sm" variant="ghost" onClick={onClose}>
              {closeLabel}
            </Button>
          ) : null}
        </div>
      </div>

      {diff.files.length > 0 ? (
        <ul className="mb-3 flex flex-wrap gap-2 text-xs">
          {diff.files.map((f) => (
            <li
              key={f.path}
              className="rounded-sm border border-line bg-raised px-2 py-0.5 font-mono text-fg"
            >
              {f.path} <span className="text-done">+{f.additions}</span>{" "}
              <span className="text-danger">-{f.deletions}</span>
            </li>
          ))}
        </ul>
      ) : null}

      <div className="min-h-0 flex-1 overflow-auto rounded-md border border-line bg-terminal font-mono text-xs">
        {mode === "unified" ? (
          <pre className="p-3 whitespace-pre-wrap break-words text-terminal-fg">
            {diff.patch || "(no diff)"}
          </pre>
        ) : (
          <div className="divide-y divide-line">
            {rows.map((row, idx) => (
              <div key={idx} className="grid grid-cols-1 sm:grid-cols-2">
                <div
                  className={
                    row.kind === "del"
                      ? "bg-danger/15 px-2 py-0.5 whitespace-pre-wrap break-words text-danger"
                      : row.kind === "meta"
                        ? "bg-raised px-2 py-0.5 whitespace-pre-wrap break-words text-accent"
                        : "px-2 py-0.5 whitespace-pre-wrap break-words text-terminal-fg"
                  }
                >
                  {row.left}
                </div>
                <div
                  className={
                    row.kind === "add"
                      ? "border-t border-line bg-done/15 px-2 py-0.5 whitespace-pre-wrap break-words text-done sm:border-t-0 sm:border-l"
                      : row.kind === "meta"
                        ? "border-t border-line bg-raised px-2 py-0.5 whitespace-pre-wrap break-words text-accent sm:border-t-0 sm:border-l"
                        : "border-t border-line px-2 py-0.5 whitespace-pre-wrap break-words text-terminal-fg sm:border-t-0 sm:border-l"
                  }
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

export default DiffView;
