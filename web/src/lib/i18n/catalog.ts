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
    idle: string;
  };
  nav: {
    sessions: string;
    pipelines: string;
    approvals: string;
    newSession: string;
    searchPlaceholder: string;
    allProjects: string;
    sessionsOnly: string;
    byProject: string;
    terminalTab: string;
    diffTab: string;
    sendPlaceholder: string;
    noSessionSelected: string;
    selectOrCreate: string;
    pause: string;
    resume: string;
    hibernate: string;
    merge: string;
    kill: string;
    discard: string;
    createTitle: string;
    projectLabel: string;
    profileLabel: string;
    taskLabel: string;
    taskPlaceholder: string;
    launch: string;
    cancel: string;
    addRepoPlaceholder: string;
    addRepoButton: string;
    approve: string;
    deny: string;
    noApprovals: string;
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
      waitingApproval: "Menunggu Anda",
      readyReview: "Selesai",
      attention: "Gagal",
      idle: "Idle",
    },
    nav: {
      sessions: "Sesi",
      pipelines: "Pipeline & Arena",
      approvals: "Persetujuan",
      newSession: "+ Sesi baru",
      searchPlaceholder: "Cari sesi, branch, atau tugas…",
      allProjects: "Semua proyek",
      sessionsOnly: "Daftar sesi",
      byProject: "Per proyek",
      terminalTab: "Terminal",
      diffTab: "Diff",
      sendPlaceholder: "Kirim input atau perintah ke sesi…",
      noSessionSelected: "Pilih sesi di sidebar atau buat sesi baru",
      selectOrCreate: "Setiap sesi berjalan terisolasi di git worktree sendiri.",
      pause: "Jeda",
      resume: "Lanjut",
      hibernate: "Hibernasi",
      merge: "Merge",
      kill: "Hentikan",
      discard: "Buang",
      createTitle: "Sesi Agen Baru",
      projectLabel: "Proyek",
      profileLabel: "Profil Agen",
      taskLabel: "Tugas / Prompt",
      taskPlaceholder: "Tambahkan filter tag beserta unit test...",
      launch: "Jalankan Sesi",
      cancel: "Batal",
      addRepoPlaceholder: "/path/ke/repo-git",
      addRepoButton: "Tambah Repo",
      approve: "Setujui",
      deny: "Tolak",
      noApprovals: "Tidak ada permintaan persetujuan tertunda.",
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
      running: "Working",
      waitingApproval: "Waiting on you",
      readyReview: "Done",
      attention: "Failed",
      idle: "Idle",
    },
    nav: {
      sessions: "Sessions",
      pipelines: "Pipelines & Arena",
      approvals: "Approvals",
      newSession: "+ New session",
      searchPlaceholder: "Search sessions, branches, and tasks…",
      allProjects: "All projects",
      sessionsOnly: "Sessions only",
      byProject: "By project",
      terminalTab: "Terminal",
      diffTab: "Diff",
      sendPlaceholder: "Send input or command to session…",
      noSessionSelected: "Select a session from the sidebar or start a new one",
      selectOrCreate: "Each session runs isolated in its own git worktree.",
      pause: "Pause",
      resume: "Resume",
      hibernate: "Hibernate",
      merge: "Merge",
      kill: "Kill",
      discard: "Discard",
      createTitle: "New Agent Session",
      projectLabel: "Project",
      profileLabel: "Agent Profile",
      taskLabel: "Task / Prompt",
      taskPlaceholder: "Add a tag filter with unit tests...",
      launch: "Launch Session",
      cancel: "Cancel",
      addRepoPlaceholder: "/path/to/git-repo",
      addRepoButton: "Add Repo",
      approve: "Approve",
      deny: "Deny",
      noApprovals: "No pending approval requests.",
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
