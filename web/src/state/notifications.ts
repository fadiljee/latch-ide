import { create } from "zustand";
import type { ApprovalRequest } from "../transport/protocol";

export interface ToastItem {
  id: string;
  tone: "info" | "success" | "danger";
  message: string;
}

interface NotificationsState {
  approvals: ApprovalRequest[];
  toasts: ToastItem[];
  setApprovals: (list: ApprovalRequest[]) => void;
  removeApproval: (id: string) => void;
  pushToast: (tone: ToastItem["tone"], message: string) => void;
  dismissToast: (id: string) => void;
}

export const useNotificationsStore = create<NotificationsState>((set) => ({
  approvals: [],
  toasts: [],
  setApprovals: (approvals) => set({ approvals }),
  removeApproval: (id) =>
    set((s) => ({ approvals: s.approvals.filter((a) => a.id !== id) })),
  pushToast: (tone, message) =>
    set((s) => ({
      toasts: [
        ...s.toasts.slice(-3),
        { id: `${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, tone, message },
      ],
    })),
  dismissToast: (id) =>
    set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
}));
