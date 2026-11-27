export type Locale = "id" | "en";

export interface Catalog {
  appName: string;
  subtitle: string;
  localeToggleLabel: string;
  boardHeading: string;
  emptySessions: string;
  connectionLabel: string;
  connected: string;
  disconnected: string;
  tokenPlaceholder: string;
  connectButton: string;
  diffTitle: string;
  closeLabel: string;
  viewDiff: string;
  columns: {
    running: string;
    waitingApproval: string;
    readyReview: string;
    attention: string;
  };
}

export const CATALOGS: Record<Locale, Catalog> = {
  id: {
    appName: "Latch Workbench",
    subtitle: "Mission Control Sesi Agen",
    localeToggleLabel: "Ganti bahasa antarmuka",
    boardHeading: "Papan Sesi",
    emptySessions: "Belum ada sesi aktif. Mulai lewat CLI atau dialog sesi baru.",
    connectionLabel: "Status daemon",
    connected: "Terhubung ke daemon",
    disconnected: "Menunggu koneksi daemon",
    tokenPlaceholder: "Tempel bearer token (latch token show)",
    connectButton: "Hubungkan",
    diffTitle: "Tinjauan Diff Sesi",
    closeLabel: "Tutup",
    viewDiff: "Diff",
    columns: {
      running: "Berjalan",
      waitingApproval: "Butuh Persetujuan",
      readyReview: "Siap Review",
      attention: "Perlu Perhatian",
    },
  },
  en: {
    appName: "Latch Workbench",
    subtitle: "Agent Session Mission Control",
    localeToggleLabel: "Switch interface language",
    boardHeading: "Session Board",
    emptySessions: "No active sessions yet. Start one via CLI or the new session dialog.",
    connectionLabel: "Daemon status",
    connected: "Connected to daemon",
    disconnected: "Waiting for daemon connection",
    tokenPlaceholder: "Paste bearer token (latch token show)",
    connectButton: "Connect",
    diffTitle: "Session Diff Review",
    closeLabel: "Close",
    viewDiff: "Diff",
    columns: {
      running: "Running",
      waitingApproval: "Waiting Approval",
      readyReview: "Ready for Review",
      attention: "Needs Attention",
    },
  },
};
