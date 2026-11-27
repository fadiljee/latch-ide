import { create } from "zustand";
import type { Session, SessionStatus, UiStatus } from "../transport/protocol";

export function mapSessionUiStatus(status: SessionStatus): UiStatus {
  switch (status) {
    case "waiting_approval":
      return "needs_you";
    case "running":
    case "paused":
      return "working";
    case "failed":
    case "tripped":
      return "error";
    case "completed":
      return "done";
    case "hibernated":
      return "idle";
  }
}

interface SessionsState {
  byId: Record<string, Session>;
  order: string[];
  unreadById: Record<string, number>;
  upsertMany: (list: Session[]) => void;
  upsertOne: (session: Session) => void;
  removeOne: (id: string) => void;
  markRead: (id: string) => void;
}

export const useSessionsStore = create<SessionsState>((set) => ({
  byId: {},
  order: [],
  unreadById: {},
  upsertMany: (list) =>
    set((s) => {
      const nextById: Record<string, Session> = {};
      const nextOrder: string[] = [];
      const nextUnread: Record<string, number> = { ...s.unreadById };
      for (const item of list) {
        nextById[item.id] = item;
        nextOrder.push(item.id);
        const prev = s.byId[item.id];
        if (prev && prev.status !== item.status && item.status === "waiting_approval") {
          nextUnread[item.id] = (nextUnread[item.id] ?? 0) + 1;
        }
      }
      return { byId: nextById, order: nextOrder, unreadById: nextUnread };
    }),
  upsertOne: (session) =>
    set((s) => ({
      byId: { ...s.byId, [session.id]: session },
      order: s.order.includes(session.id) ? s.order : [session.id, ...s.order],
    })),
  removeOne: (id) =>
    set((s) => ({
      byId: Object.fromEntries(Object.entries(s.byId).filter(([k]) => k !== id)),
      order: s.order.filter((x) => x !== id),
    })),
  markRead: (id) =>
    set((s) => ({
      unreadById: { ...s.unreadById, [id]: 0 },
    })),
}));
