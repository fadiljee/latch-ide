export type Locale = "id" | "en";

export interface Catalog {
  appName: string;
  localeToggleLabel: string;
  connection: {
    label: string;
    connected: string;
    reconnecting: string;
    tokenPlaceholder: string;
    connectButton: string;
    disconnectNote: string;
  };
  routes: {
    workspace: string;
    automations: string;
    bots: string;
    hosts: string;
    usage: string;
    settings: string;
  };
  sidebar: {
    heading: string;
    searchPlaceholder: string;
    statusMode: string;
    treeMode: string;
    addRepoPlaceholder: string;
    addRepoButton: string;
    emptyTitle: string;
    emptyBody: string;
    emptyFiltered: string;
    loading: string;
  };
  statusLabels: {
    needs_you: string;
    working: string;
    error: string;
    done: string;
    idle: string;
  };
  workspace: {
    terminalTab: string;
    chatTab: string;
    diffTab: string;
    splitToggle: string;
    singlePane: string;
    sidePanelToggle: string;
    emptyTitle: string;
    emptyBody: string;
    newSessionCta: string;
    loadingDiff: string;
    loadingTelemetry: string;
    resync: string;
  };
  sessionActions: {
    pause: string;
    resume: string;
    hibernate: string;
    kill: string;
    merge: string;
    discard: string;
    composerPlaceholder: string;
    send: string;
    screenPreview: string;
    historyTitle: string;
    noMessages: string;
  };
  newSession: {
    title: string;
    projectLabel: string;
    profileLabel: string;
    taskLabel: string;
    taskPlaceholder: string;
    baseRefLabel: string;
    baseRefPlaceholder: string;
    launch: string;
    cancel: string;
    needRepo: string;
  };
  sidePanel: {
    gitTab: string;
    checksTab: string;
    trafficTab: string;
    conflictCheck: string;
    cleanMerge: string;
    conflictFound: string;
    noChanges: string;
    approve: string;
    deny: string;
    noApprovals: string;
    noTraffic: string;
    diffTitle: string;
    close: string;
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
    launchTitle: string;
    workflowPlaceholder: string;
    taskPlaceholder: string;
    runButton: string;
  };
  pages: {
    botsHeading: string;
    botsSubtitle: string;
    hostsHeading: string;
    hostsSubtitle: string;
    remoteHint: string;
    usageHeading: string;
    usageSubtitle: string;
    gcDryRun: string;
    gcApply: string;
    settingsHeading: string;
    settingsSubtitle: string;
    styleLabel: string;
    modeLabel: string;
    accentLabel: string;
    tokenLabel: string;
    languageLabel: string;
  };
  palette: {
    placeholder: string;
    empty: string;
    toggleSidebar: string;
    toggleSidePanel: string;
    toggleSplit: string;
    openNewSession: string;
    openSettings: string;
    openFloating: string;
  };
  floating: {
    title: string;
    needsYou: string;
    noPending: string;
  };
}

export const CATALOGS: Record<Locale, Catalog> = {
  id: {
    appName: "Latch IDE",
    localeToggleLabel: "Ganti bahasa antarmuka",
    connection: {
      label: "Status daemon",
      connected: "Terhubung",
      reconnecting: "Menyambung ulang",
      tokenPlaceholder: "Tempel bearer token (latch token show)",
      connectButton: "Hubungkan",
      disconnectNote: "Masukkan bearer token dari `latch token show` untuk mengakses daemon lokal.",
    },
    routes: {
      workspace: "Workspace",
      automations: "Automasi",
      bots: "Agen",
      hosts: "Host",
      usage: "Penggunaan",
      settings: "Pengaturan",
    },
    sidebar: {
      heading: "Daftar Sesi",
      searchPlaceholder: "Saring sesi, branch, atau tugas...",
      statusMode: "Status",
      treeMode: "Pohon Proyek",
      addRepoPlaceholder: "/path/ke/repositori-git",
      addRepoButton: "Tambah Repo",
      emptyTitle: "Belum ada sesi aktif",
      emptyBody: "Daftarkan repositori Git lalu buat sesi agen terisolasi baru.",
      emptyFiltered: "Tidak ada sesi yang cocok dengan kata kunci pencarian.",
      loading: "Memuat sesi dari daemon...",
    },
    statusLabels: {
      needs_you: "Menunggu Anda",
      working: "Berjalan",
      error: "Perlu Perhatian",
      done: "Selesai",
      idle: "Hibernasi",
    },
    workspace: {
      terminalTab: "Terminal",
      chatTab: "Aktivitas",
      diffTab: "Diff",
      splitToggle: "Split Pane",
      singlePane: "Satu Pane",
      sidePanelToggle: "Panel Kanan",
      emptyTitle: "Pilih sesi di sidebar atau buat sesi baru",
      emptyBody: "Setiap sesi berjalan di git worktree terisolasi dengan proxy telemetri dan sandbox.",
      newSessionCta: "+ Sesi Baru",
      loadingDiff: "Memuat diff worktree...",
      loadingTelemetry: "Memuat telemetri proxy...",
      resync: "Sinkronkan Layar",
    },
    sessionActions: {
      pause: "Jeda",
      resume: "Lanjut",
      hibernate: "Hibernasi",
      kill: "Hentikan",
      merge: "Squash Merge",
      discard: "Buang",
      composerPlaceholder: "Kirim perintah atau balasan ke PTY sesi...",
      send: "Kirim",
      screenPreview: "Cuplikan Layar vt100",
      historyTitle: "Riwayat Input Sesi",
      noMessages: "Belum ada input yang dikirim dari panel ini.",
    },
    newSession: {
      title: "Buat Sesi Agen Baru",
      projectLabel: "Proyek",
      profileLabel: "Profil Agen",
      taskLabel: "Deskripsi Tugas",
      taskPlaceholder: "Tambahkan filter tag beserta unit test...",
      baseRefLabel: "Branch Dasar (opsional)",
      baseRefPlaceholder: "Kosongkan untuk branch bawaan proyek",
      launch: "Jalankan Sesi",
      cancel: "Batal",
      needRepo: "Tambahkan minimal satu repositori Git terlebih dahulu.",
    },
    sidePanel: {
      gitTab: "Git",
      checksTab: "Persetujuan",
      trafficTab: "Trafik LLM",
      conflictCheck: "Cek Konflik",
      cleanMerge: "Bersih: siap digabungkan ke branch utama.",
      conflictFound: "Konflik terdeteksi pada berkas:",
      noChanges: "Belum ada perubahan di worktree sesi.",
      approve: "Setujui",
      deny: "Tolak",
      noApprovals: "Tidak ada permintaan persetujuan yang menunggu.",
      noTraffic: "Belum ada permintaan proxy LLM yang tercatat untuk sesi ini.",
      diffTitle: "Tinjauan Diff Sesi",
      close: "Tutup",
    },
    pipelines: {
      heading: "Pipeline DAG & Arena",
      empty: "Belum ada eksekusi pipeline. Jalankan `workflow.toml` di bawah untuk memulai.",
      arenaHeading: "Matriks Perbandingan Fan-Out Arena",
      viewArena: "Buka Arena",
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
      launchTitle: "Jalankan Pipeline workflow.toml",
      workflowPlaceholder: "Tempel isi workflow.toml...",
      taskPlaceholder: "Tugas untuk pipeline DAG...",
      runButton: "Jalankan Pipeline",
    },
    pages: {
      botsHeading: "Profil Agen Terdaftar",
      botsSubtitle: "Profil bawaan yang siap dijalankan di git worktree terisolasi.",
      hostsHeading: "Daemon & Host Terhubung",
      hostsSubtitle: "Status runtime daemon lokal dan panduan terowongan SSH aman.",
      remoteHint: "Hubungkan host jarak jauh tanpa listener publik dengan `latch remote user@host`.",
      usageHeading: "Penggunaan Biaya & Retensi",
      usageSubtitle: "Ringkasan biaya sesi tercatat dan pembersihan data retensi (`latch gc`).",
      gcDryRun: "Simulasi GC",
      gcApply: "Jalankan GC",
      settingsHeading: "Pengaturan Tampilan & Koneksi",
      settingsSubtitle: "Atur gaya visual, mode warna, aksen, bahasa, dan bearer token daemon.",
      styleLabel: "Gaya Tata Letak",
      modeLabel: "Mode Warna",
      accentLabel: "Warna Aksen",
      tokenLabel: "Bearer Token Daemon",
      languageLabel: "Bahasa Antarmuka",
    },
    palette: {
      placeholder: "Ketik perintah atau nama sesi...",
      empty: "Tidak ada perintah yang cocok.",
      toggleSidebar: "Buka/Tutup Sidebar Sesi",
      toggleSidePanel: "Buka/Tutup Panel Kanan",
      toggleSplit: "Aktifkan/Matikan Split Pane",
      openNewSession: "Buat Sesi Agen Baru",
      openSettings: "Buka Pengaturan",
      openFloating: "Buka Jendela Mengambang",
    },
    floating: {
      title: "Kontrol Cepat Latch",
      needsYou: "Perlu Persetujuan",
      noPending: "Semua sesi berjalan lancar tanpa antrean persetujuan.",
    },
  },
  en: {
    appName: "Latch IDE",
    localeToggleLabel: "Switch interface language",
    connection: {
      label: "Daemon status",
      connected: "Connected",
      reconnecting: "Reconnecting",
      tokenPlaceholder: "Paste bearer token (latch token show)",
      connectButton: "Connect",
      disconnectNote: "Enter the bearer token from `latch token show` to connect to the local daemon.",
    },
    routes: {
      workspace: "Workspace",
      automations: "Automations",
      bots: "Agents",
      hosts: "Hosts",
      usage: "Usage",
      settings: "Settings",
    },
    sidebar: {
      heading: "Sessions",
      searchPlaceholder: "Filter sessions, branches, or tasks...",
      statusMode: "Status",
      treeMode: "Project Tree",
      addRepoPlaceholder: "/path/to/git-repo",
      addRepoButton: "Add Repo",
      emptyTitle: "No active sessions yet",
      emptyBody: "Register a Git repository and launch an isolated agent session.",
      emptyFiltered: "No sessions match your search filter.",
      loading: "Loading sessions from daemon...",
    },
    statusLabels: {
      needs_you: "Needs You",
      working: "Working",
      error: "Needs Attention",
      done: "Done",
      idle: "Hibernated",
    },
    workspace: {
      terminalTab: "Terminal",
      chatTab: "Activity",
      diffTab: "Diff",
      splitToggle: "Split Pane",
      singlePane: "Single Pane",
      sidePanelToggle: "Side Panel",
      emptyTitle: "Select a session or launch a new one",
      emptyBody: "Each session runs in an isolated git worktree with proxy telemetry and sandboxing.",
      newSessionCta: "+ New Session",
      loadingDiff: "Loading worktree diff...",
      loadingTelemetry: "Loading proxy telemetry...",
      resync: "Resync Screen",
    },
    sessionActions: {
      pause: "Pause",
      resume: "Resume",
      hibernate: "Hibernate",
      kill: "Kill",
      merge: "Squash Merge",
      discard: "Discard",
      composerPlaceholder: "Send input or command to the session PTY...",
      send: "Send",
      screenPreview: "vt100 Screen Snapshot",
      historyTitle: "Session Input History",
      noMessages: "No inputs sent from this composer yet.",
    },
    newSession: {
      title: "Launch Agent Session",
      projectLabel: "Project",
      profileLabel: "Agent Profile",
      taskLabel: "Task Description",
      taskPlaceholder: "Add a tag filter with unit tests...",
      baseRefLabel: "Base Ref (optional)",
      baseRefPlaceholder: "Leave empty for project default branch",
      launch: "Launch Session",
      cancel: "Cancel",
      needRepo: "Register at least one Git repository first.",
    },
    sidePanel: {
      gitTab: "Git",
      checksTab: "Approvals",
      trafficTab: "LLM Traffic",
      conflictCheck: "Check Conflicts",
      cleanMerge: "Clean: ready to merge into default branch.",
      conflictFound: "Conflicts detected in files:",
      noChanges: "No worktree changes recorded yet.",
      approve: "Approve",
      deny: "Deny",
      noApprovals: "No pending approval requests.",
      noTraffic: "No proxy LLM requests recorded for this session yet.",
      diffTitle: "Session Diff Review",
      close: "Close",
    },
    pipelines: {
      heading: "DAG Pipelines & Arena",
      empty: "No pipeline runs yet. Run a workflow.toml below to get started.",
      arenaHeading: "Fan-Out Arena Comparison Matrix",
      viewArena: "Open Arena",
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
      launchTitle: "Run workflow.toml Pipeline",
      workflowPlaceholder: "Paste workflow.toml content...",
      taskPlaceholder: "Pipeline task prompt...",
      runButton: "Run Pipeline",
    },
    pages: {
      botsHeading: "Registered Agent Profiles",
      botsSubtitle: "Built-in profiles ready to execute inside isolated git worktrees.",
      hostsHeading: "Daemon & Connected Hosts",
      hostsSubtitle: "Local daemon runtime status and secure SSH forwarding instructions.",
      remoteHint: "Connect remote hosts without public listeners via `latch remote user@host`.",
      usageHeading: "Cost Usage & Retention",
      usageSubtitle: "Recorded session token/cost summary and retention cleanup (`latch gc`).",
      gcDryRun: "Dry-Run GC",
      gcApply: "Run GC",
      settingsHeading: "Appearance & Connection",
      settingsSubtitle: "Customize layout style, color mode, accent color, language, and daemon token.",
      styleLabel: "Layout Style",
      modeLabel: "Color Mode",
      accentLabel: "Accent Color",
      tokenLabel: "Daemon Bearer Token",
      languageLabel: "Interface Language",
    },
    palette: {
      placeholder: "Type a command or session name...",
      empty: "No matching commands.",
      toggleSidebar: "Toggle Session Sidebar",
      toggleSidePanel: "Toggle Side Panel",
      toggleSplit: "Toggle Split Pane",
      openNewSession: "New Agent Session",
      openSettings: "Open Settings",
      openFloating: "Toggle Floating Control",
    },
    floating: {
      title: "Latch Quick Control",
      needsYou: "Waiting on You",
      noPending: "All sessions running smoothly with no pending approvals.",
    },
  },
};
