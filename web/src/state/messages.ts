import { create } from "zustand";

export interface SessionMessageEntry {
  id: string;
  role: "user" | "system";
  text: string;
  timestamp: number;
}

const MAX_MESSAGES_PER_SESSION = 200;

interface MessagesState {
  bySessionId: Record<string, SessionMessageEntry[]>;
  appendMessage: (sessionId: string, role: "user" | "system", text: string) => void;
}

export const useMessagesStore = create<MessagesState>((set) => ({
  bySessionId: {},
  appendMessage: (sessionId, role, text) =>
    set((s) => {
      const prev = s.bySessionId[sessionId] ?? [];
      const entry: SessionMessageEntry = {
        id: `${Date.now()}-${prev.length}`,
        role,
        text,
        timestamp: Date.now(),
      };
      const next = [...prev, entry];
      const capped =
        next.length > MAX_MESSAGES_PER_SESSION
          ? next.slice(next.length - MAX_MESSAGES_PER_SESSION)
          : next;
      return {
        bySessionId: {
          ...s.bySessionId,
          [sessionId]: capped,
        },
      };
    }),
}));
