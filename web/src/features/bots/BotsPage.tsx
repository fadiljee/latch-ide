import { Button } from "../../components/ui/Button";
import { ModelBadge } from "../../components/ModelBadge";
import type { Catalog } from "../../lib/i18n/catalog";
import { useFilteredSessions } from "../../state/selectors";
import { useUiStore } from "../../state/ui";
import { BUILTIN_PROFILES } from "../sessions/ModelPicker";

interface BotsPageProps {
  labels: Catalog["pages"];
  newSessionLabel: string;
}

export function BotsPage({ labels, newSessionLabel }: BotsPageProps) {
  const sessions = useFilteredSessions();
  const setNewSessionOpen = useUiStore((s) => s.setNewSessionOpen);

  return (
    <div className="flex-1 space-y-6 overflow-y-auto p-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold text-fg">{labels.botsHeading}</h2>
          <p className="mt-0.5 text-xs text-fg-3">{labels.botsSubtitle}</p>
        </div>
        <Button variant="primary" onClick={() => setNewSessionOpen(true)}>
          {newSessionLabel}
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {BUILTIN_PROFILES.map((prof) => {
          const activeCount = sessions.filter(
            (s) => s.agent_profile === prof.id && s.status === "running",
          ).length;
          return (
            <div
              key={prof.id}
              className="rounded-lg border border-line bg-surface p-4 text-xs shadow-[var(--panel-shadow)]"
            >
              <div className="flex items-center justify-between">
                <ModelBadge profile={prof.id} />
                <span className="font-mono text-fg-3">{prof.provider}</span>
              </div>
              <p className="mt-3 text-fg-2">
                Active sessions: <span className="font-mono text-fg">{activeCount}</span>
              </p>
            </div>
          );
        })}
      </div>
    </div>
  );
}
