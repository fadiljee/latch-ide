import { useMemo } from "react";
import type { Session, UiStatus } from "../transport/protocol";
import { useHostsStore } from "./hosts";
import { mapSessionUiStatus, useSessionsStore } from "./sessions";
import { useUiStore } from "./ui";
import { useUsageStore } from "./usage";

export interface SessionGroup {
  status: UiStatus;
  items: Session[];
}

const STATUS_ORDER: readonly UiStatus[] = ["needs_you", "working", "error", "done", "idle"];

export function useFilteredSessions(): Session[] {
  const byId = useSessionsStore((s) => s.byId);
  const order = useSessionsStore((s) => s.order);
  const selectedProjectId = useHostsStore((s) => s.selectedProjectId);
  const searchQuery = useUiStore((s) => s.searchQuery);

  return useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const result: Session[] = [];
    for (const id of order) {
      const ses = byId[id];
      if (!ses) {
        continue;
      }
      if (selectedProjectId !== "all" && ses.project_id !== selectedProjectId) {
        continue;
      }
      if (q) {
        const hay = `${ses.task ?? ""} ${ses.branch_name} ${ses.agent_profile} ${ses.id}`.toLowerCase();
        if (!hay.includes(q)) {
          continue;
        }
      }
      result.push(ses);
    }
    return result;
  }, [byId, order, selectedProjectId, searchQuery]);
}

export function useGroupedSessions(): SessionGroup[] {
  const filtered = useFilteredSessions();

  return useMemo(() => {
    const buckets: Record<UiStatus, Session[]> = {
      needs_you: [],
      working: [],
      error: [],
      done: [],
      idle: [],
    };
    for (const ses of filtered) {
      buckets[mapSessionUiStatus(ses.status)].push(ses);
    }
    return STATUS_ORDER.map((status) => ({
      status,
      items: buckets[status],
    })).filter((g) => g.items.length > 0);
  }, [filtered]);
}

export function useNeedsYouCount(): number {
  const byId = useSessionsStore((s) => s.byId);
  const order = useSessionsStore((s) => s.order);

  return useMemo(() => {
    let count = 0;
    for (const id of order) {
      const ses = byId[id];
      if (ses && mapSessionUiStatus(ses.status) === "needs_you") {
        count += 1;
      }
    }
    return count;
  }, [byId, order]);
}

export function useTotalCostUsd(): number {
  const bySessionId = useUsageStore((s) => s.bySessionId);
  return useMemo(() => {
    let total = 0;
    for (const summary of Object.values(bySessionId)) {
      total += summary.estimated_cost_usd;
    }
    return total;
  }, [bySessionId]);
}
