import { create } from "zustand";
import type { GcReport, PipelineArenaResponse, PipelineRun, SessionTelemetrySummary } from "../transport/protocol";

interface UsageState {
  bySessionId: Record<string, SessionTelemetrySummary>;
  pipelines: PipelineRun[];
  activeArena: PipelineArenaResponse | null;
  gcReport: GcReport | null;
  setSessionSummary: (sessionId: string, summary: SessionTelemetrySummary) => void;
  setPipelines: (pipelines: PipelineRun[]) => void;
  upsertPipeline: (run: PipelineRun) => void;
  setActiveArena: (arena: PipelineArenaResponse | null) => void;
  setGcReport: (report: GcReport | null) => void;
}

export const useUsageStore = create<UsageState>((set) => ({
  bySessionId: {},
  pipelines: [],
  activeArena: null,
  gcReport: null,
  setSessionSummary: (sessionId, summary) =>
    set((s) => ({
      bySessionId: { ...s.bySessionId, [sessionId]: summary },
    })),
  setPipelines: (pipelines) => set({ pipelines }),
  upsertPipeline: (run) =>
    set((s) => {
      const exists = s.pipelines.some((p) => p.id === run.id);
      return {
        pipelines: exists
          ? s.pipelines.map((p) => (p.id === run.id ? run : p))
          : [run, ...s.pipelines],
      };
    }),
  setActiveArena: (activeArena) => set({ activeArena }),
  setGcReport: (gcReport) => set({ gcReport }),
}));
