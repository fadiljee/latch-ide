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
  pipelines: {
    heading: string;
    empty: string;
    arenaHeading: string;
    viewArena: string;
    cancel: string;
    selectWinner: string;
    winner: string;
    passed: string;
    failed: string;
    profile: string;
    status: string;
    duration: string;
    diffLines: string;
    cost: string;
    action: string;
    noArenaSteps: string;
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
    pipelines: {
      heading: "Pipeline DAG & Arena",
      empty: "Belum ada eksekusi pipeline. Mulai lewat `latch pipeline run`.",
      arenaHeading: "Matriks Perbandingan Fan-Out Arena",
      viewArena: "Arena",
      cancel: "Batalkan",
      selectWinner: "Pilih Pemenang",
      winner: "Pemenang",
      passed: "Lolos",
      failed: "Gagal",
      profile: "Profil",
      status: "Status",
      duration: "Durasi",
      diffLines: "Baris Diff",
      cost: "Biaya",
      action: "Aksi",
      noArenaSteps: "Tidak ada langkah fan-out arena pada pipeline ini.",
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
    pipelines: {
      heading: "DAG Pipelines & Arena",
      empty: "No pipeline runs yet. Start one via `latch pipeline run`.",
      arenaHeading: "Fan-Out Arena Comparison Matrix",
      viewArena: "Arena",
      cancel: "Cancel",
      selectWinner: "Select Winner",
      winner: "Winner",
      passed: "Pass",
      failed: "Fail",
      profile: "Profile",
      status: "Status",
      duration: "Duration",
      diffLines: "Diff Lines",
      cost: "Cost",
      action: "Action",
      noArenaSteps: "No fan-out arena steps recorded for this pipeline.",
    },
  },
};
