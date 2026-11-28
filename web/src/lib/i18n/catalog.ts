export type Locale = "id" | "en";

export interface Catalog {
  appName: string;
  localeToggleLabel: string;
  update: {
    available: string;
    howTo: string;
    notes: string;
    dismiss: string;
  };
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
    plugins: string;
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
    gridMode: string;
    boardMode: string;
    boardHeading: string;
    boardSubtitle: string;
    boardColumnEmpty: string;
    focusedCell: string;
    clickToFocus: string;
    expandPane: string;
    sidePanelToggle: string;
    emptyTitle: string;
    emptyBody: string;
    newSessionCta: string;
    openAntigravityCli: string;
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
    bracketedPaste: string;
    previewToggle: string;
    previewHeading: string;
    templatesLabel: string;
  };
  newSession: {
    title: string;
    projectLabel: string;
    profileLabel: string;
    taskLabel: string;
    taskPlaceholder: string;
    taskOptionalHint: string;
    agyOptionsHeading: string;
    agyModeLabel: string;
    agyEffortLabel: string;
    agyModelLabel: string;
    agyDefaultOption: string;
    agyContinueLabel: string;
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
  agents: {
    addCustom: string;
    searchPlaceholder: string;
    filterAll: string;
    filterInstalled: string;
    filterCustom: string;
    installed: string;
    notInstalled: string;
    authLabel: string;
    oauthMode: string;
    apiKeyMode: string;
    loginOauth: string;
    launchSession: string;
    removeCustom: string;
    apiKeyEnvLabel: string;
    apiKeyConfiguredEnv: string;
    apiKeyConfiguredMemory: string;
    apiKeyMissing: string;
    runtimeKeyPlaceholder: string;
    saveRuntimeKey: string;
    clearRuntimeKey: string;
    runtimeKeyHint: string;
    customDialogTitle: string;
    idLabel: string;
    displayNameLabel: string;
    commandLabel: string;
    argsLabel: string;
    loginCmdLabel: string;
    providerLabel: string;
    saveAgent: string;
    cancel: string;
    activeSessions: string;
    emptySearch: string;
  };
  integrations: {
    heading: string;
    subtitle: string;
    githubTab: string;
    mcpTab: string;
    pluginsTab: string;
    ghConnected: string;
    ghDisconnected: string;
    ghNotInstalled: string;
    ghConnectOauth: string;
    ghRefreshRepos: string;
    ghClonePlaceholder: string;
    ghCloneButton: string;
    ghCloneAndAdd: string;
    ghNoRepos: string;
    mcpAddHeading: string;
    mcpPresetsHeading: string;
    mcpEmpty: string;
    mcpIdLabel: string;
    mcpNameLabel: string;
    mcpTransportLabel: string;
    mcpCommandLabel: string;
    mcpArgsLabel: string;
    mcpUrlLabel: string;
    mcpEnvKeysLabel: string;
    mcpEnvKeysHint: string;
    mcpSave: string;
    pluginsAddHeading: string;
    pluginsPresetsHeading: string;
    pluginsEmpty: string;
    pluginKindLabel: string;
    pluginSourceLabel: string;
    pluginDescLabel: string;
    pluginSave: string;
    installPreset: string;
    installedPreset: string;
    enabled: string;
    disabled: string;
    remove: string;
  };
  palette: {
    placeholder: string;
    empty: string;
    toggleSidebar: string;
    toggleSidePanel: string;
    toggleSplit: string;
    openBoard: string;
    openGrid: string;
    openAntigravityCli: string;
    openNewSession: string;
    openAgents: string;
    openPlugins: string;
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
    update: {
      available: "Versi baru Latch IDE tersedia: {latest} (terpasang: {current}).",
      howTo: "Perbarui dengan menjalankan:",
      notes: "Catatan rilis",
      dismiss: "Tutup notifikasi pembaruan",
    },
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
      plugins: "Integrasi",
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
      gridMode: "Grid 2×2",
      boardMode: "Papan",
      boardHeading: "Papan Sesi Mission Control",
      boardSubtitle: "Pantau seluruh sesi agen secara langsung berdasarkan status dan buka terminal/diff dengan 1 klik.",
      boardColumnEmpty: "Tidak ada sesi",
      focusedCell: "Fokus",
      clickToFocus: "Klik fokus",
      expandPane: "Fokus Penuh",
      sidePanelToggle: "Panel Kanan",
      emptyTitle: "Pilih sesi di sidebar atau buat sesi baru",
      emptyBody: "Setiap sesi berjalan di git worktree terisolasi dengan proxy telemetri dan sandbox.",
      newSessionCta: "+ Sesi Baru",
      openAntigravityCli: "Antigravity CLI",
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
      bracketedPaste: "Bracketed Paste",
      previewToggle: "Pratinjau",
      previewHeading: "Pratinjau Payload PTY",
      templatesLabel: "Templat Pesan Cepat",
    },
    newSession: {
      title: "Buat Sesi Agen Baru",
      projectLabel: "Proyek",
      profileLabel: "Profil Agen",
      taskLabel: "Deskripsi Tugas (opsional)",
      taskPlaceholder: "Tambahkan filter tag beserta unit test...",
      taskOptionalHint: "Biarkan kosong untuk membuka terminal CLI interaktif.",
      agyOptionsHeading: "Opsi Antigravity CLI (agy)",
      agyModeLabel: "Mode Eksekusi (--mode)",
      agyEffortLabel: "Penalaran (--effort)",
      agyModelLabel: "Model (--model)",
      agyDefaultOption: "Bawaan CLI",
      agyContinueLabel: "Lanjutkan percakapan terakhir (--continue)",
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
      botsHeading: "Semua Agen Coding (OAuth & API Key)",
      botsSubtitle:
        "Dukungan penuh Antigravity (agy), Claude, Codex, Gemini, OpenCode, dan CLI agen lainnya. Default OAuth, dapat beralih ke API Key kapan saja.",
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
    agents: {
      addCustom: "+ Tambah Agen",
      searchPlaceholder: "Cari agen (antigravity, claude, codex, opencode...)",
      filterAll: "Semua",
      filterInstalled: "Terpasang",
      filterCustom: "Custom",
      installed: "Terpasang",
      notInstalled: "Belum terpasang",
      authLabel: "Mode Login",
      oauthMode: "OAuth (Default)",
      apiKeyMode: "API Key",
      loginOauth: "Login OAuth",
      launchSession: "Jalankan",
      removeCustom: "Hapus",
      apiKeyEnvLabel: "Variabel Env",
      apiKeyConfiguredEnv: "Siap (Host Env)",
      apiKeyConfiguredMemory: "Siap (RAM Sesi)",
      apiKeyMissing: "Belum diset",
      runtimeKeyPlaceholder: "Tempel API key sementara (RAM daemon saja)...",
      saveRuntimeKey: "Simpan RAM",
      clearRuntimeKey: "Hapus",
      runtimeKeyHint: "Kunci hanya disimpan di RAM daemon selama sesi aktif, tidak pernah ditulis ke disk atau SQLite.",
      customDialogTitle: "Tambah Profil Agen CLI Custom",
      idLabel: "ID Profil (huruf/angka/-/_)",
      displayNameLabel: "Nama Tampilan",
      commandLabel: "Perintah Biner CLI",
      argsLabel: "Argumen Default (pisah spasi)",
      loginCmdLabel: "Perintah Login OAuth (opsional, pisah spasi)",
      providerLabel: "Provider Proxy (opsional untuk mode API Key)",
      saveAgent: "Simpan Agen",
      cancel: "Batal",
      activeSessions: "Sesi aktif",
      emptySearch: "Tidak ada agen yang cocok dengan filter saat ini.",
    },
    integrations: {
      heading: "Integrasi GitHub, MCP & Plugin",
      subtitle:
        "Hubungkan GitHub lewat OAuth (`gh`), daftarkan server Model Context Protocol (MCP), dan aktifkan plugin tambahan untuk seluruh agen.",
      githubTab: "GitHub",
      mcpTab: "MCP Servers",
      pluginsTab: "Plugins",
      ghConnected: "Terhubung ke GitHub",
      ghDisconnected: "Belum login GitHub",
      ghNotInstalled: "GitHub CLI (`gh`) belum terpasang di sistem.",
      ghConnectOauth: "Login GitHub (OAuth)",
      ghRefreshRepos: "Muat Repositori",
      ghClonePlaceholder: "owner/nama-repo",
      ghCloneButton: "Clone & Tambah",
      ghCloneAndAdd: "Clone ke Latch",
      ghNoRepos: "Klik Muat Repositori untuk menampilkan daftar repo akun GitHub Anda.",
      mcpAddHeading: "Tambah Server MCP Custom",
      mcpPresetsHeading: "Preset MCP Populer (1-Klik)",
      mcpEmpty: "Belum ada server MCP terdaftar. Pilih preset di atas atau tambahkan server baru.",
      mcpIdLabel: "ID Server",
      mcpNameLabel: "Nama Server",
      mcpTransportLabel: "Transport",
      mcpCommandLabel: "Perintah (stdio)",
      mcpArgsLabel: "Argumen (pisah spasi)",
      mcpUrlLabel: "Endpoint URL (http)",
      mcpEnvKeysLabel: "Nama Env Key (pisah koma, bukan nilainya)",
      mcpEnvKeysHint: "Simpan nama variabel lingkungan saja (contoh: GITHUB_PERSONAL_ACCESS_TOKEN).",
      mcpSave: "Simpan Server MCP",
      pluginsAddHeading: "Daftarkan Plugin Custom",
      pluginsPresetsHeading: "Katalog Plugin Cepat",
      pluginsEmpty: "Belum ada plugin aktif. Pasang dari katalog atau tambahkan plugin custom.",
      pluginKindLabel: "Kategori Plugin",
      pluginSourceLabel: "Sumber (repo GitHub, paket npm/cargo, atau path)",
      pluginDescLabel: "Deskripsi Singkat",
      pluginSave: "Simpan Plugin",
      installPreset: "Pasang",
      installedPreset: "Terpasang",
      enabled: "Aktif",
      disabled: "Nonaktif",
      remove: "Hapus",
    },
    palette: {
      placeholder: "Ketik perintah atau nama sesi...",
      empty: "Tidak ada perintah yang cocok.",
      toggleSidebar: "Buka/Tutup Sidebar Sesi",
      toggleSidePanel: "Buka/Tutup Panel Kanan",
      toggleSplit: "Ganti Tata Letak Pane (Satu / Split / Grid 2×2 / Papan)",
      openBoard: "Buka Papan Sesi (Mission Control)",
      openGrid: "Buka Grid Terminal 2×2",
      openAntigravityCli: "Luncurkan Antigravity CLI (agy)",
      openNewSession: "Buat Sesi Agen Baru",
      openAgents: "Kelola Agen (OAuth & API Key)",
      openPlugins: "Buka Integrasi (GitHub, MCP, Plugin)",
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
    update: {
      available: "A new version of Latch IDE is available: {latest} (installed: {current}).",
      howTo: "Update by running:",
      notes: "Release notes",
      dismiss: "Dismiss update notification",
    },
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
      plugins: "Integrations",
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
      gridMode: "2×2 Grid",
      boardMode: "Board",
      boardHeading: "Mission Control Session Board",
      boardSubtitle: "Track every agent session live by status and jump straight into terminal or diff.",
      boardColumnEmpty: "No sessions",
      focusedCell: "Focused",
      clickToFocus: "Click to focus",
      expandPane: "Focus Pane",
      sidePanelToggle: "Side Panel",
      emptyTitle: "Select a session or launch a new one",
      emptyBody: "Each session runs in an isolated git worktree with proxy telemetry and sandboxing.",
      newSessionCta: "+ New Session",
      openAntigravityCli: "Antigravity CLI",
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
      bracketedPaste: "Bracketed Paste",
      previewToggle: "Preview",
      previewHeading: "PTY Payload Preview",
      templatesLabel: "Quick Prompt Templates",
    },
    newSession: {
      title: "Launch Agent Session",
      projectLabel: "Project",
      profileLabel: "Agent Profile",
      taskLabel: "Task Description (optional)",
      taskPlaceholder: "Add a tag filter with unit tests...",
      taskOptionalHint: "Leave blank to launch an interactive CLI terminal.",
      agyOptionsHeading: "Antigravity CLI (agy) Options",
      agyModeLabel: "Execution Mode (--mode)",
      agyEffortLabel: "Reasoning Effort (--effort)",
      agyModelLabel: "Model (--model)",
      agyDefaultOption: "CLI Default",
      agyContinueLabel: "Continue last conversation (--continue)",
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
      botsHeading: "All Coding Agents (OAuth & API Key)",
      botsSubtitle:
        "Built-in support for Antigravity (agy), Claude, Codex, Gemini, OpenCode, and custom CLIs. OAuth by default, API Key optional.",
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
    agents: {
      addCustom: "+ Add Agent",
      searchPlaceholder: "Filter agents (antigravity, claude, codex, opencode...)",
      filterAll: "All",
      filterInstalled: "Installed",
      filterCustom: "Custom",
      installed: "Installed",
      notInstalled: "Not installed",
      authLabel: "Auth Mode",
      oauthMode: "OAuth (Default)",
      apiKeyMode: "API Key",
      loginOauth: "OAuth Login",
      launchSession: "Launch",
      removeCustom: "Delete",
      apiKeyEnvLabel: "Env Variable",
      apiKeyConfiguredEnv: "Ready (Host Env)",
      apiKeyConfiguredMemory: "Ready (Session RAM)",
      apiKeyMissing: "Not set",
      runtimeKeyPlaceholder: "Paste temporary API key (daemon RAM only)...",
      saveRuntimeKey: "Set in RAM",
      clearRuntimeKey: "Clear",
      runtimeKeyHint: "Stored in daemon memory only for active runtime, never written to disk or SQLite.",
      customDialogTitle: "Add Custom CLI Agent Profile",
      idLabel: "Profile ID (letters/digits/-/_)",
      displayNameLabel: "Display Name",
      commandLabel: "CLI Binary Command",
      argsLabel: "Default Args (space-separated)",
      loginCmdLabel: "OAuth Login Command (optional, space-separated)",
      providerLabel: "Proxy Provider (optional for API Key mode)",
      saveAgent: "Save Agent",
      cancel: "Cancel",
      activeSessions: "Active sessions",
      emptySearch: "No agents match your current filter.",
    },
    integrations: {
      heading: "GitHub, MCP & Plugins",
      subtitle:
        "Connect GitHub via OAuth (`gh`), register Model Context Protocol (MCP) servers, and enable plugins across all agents.",
      githubTab: "GitHub",
      mcpTab: "MCP Servers",
      pluginsTab: "Plugins",
      ghConnected: "Connected to GitHub",
      ghDisconnected: "GitHub not authenticated",
      ghNotInstalled: "GitHub CLI (`gh`) is not installed on this machine.",
      ghConnectOauth: "Connect GitHub (OAuth)",
      ghRefreshRepos: "Load Repositories",
      ghClonePlaceholder: "owner/repo-name",
      ghCloneButton: "Clone & Add",
      ghCloneAndAdd: "Clone to Latch",
      ghNoRepos: "Click Load Repositories to fetch your GitHub repositories.",
      mcpAddHeading: "Add Custom MCP Server",
      mcpPresetsHeading: "Popular MCP Presets (1-Click)",
      mcpEmpty: "No MCP servers configured yet. Install a preset above or add a custom server.",
      mcpIdLabel: "Server ID",
      mcpNameLabel: "Display Name",
      mcpTransportLabel: "Transport",
      mcpCommandLabel: "Command (stdio)",
      mcpArgsLabel: "Arguments (space-separated)",
      mcpUrlLabel: "Endpoint URL (http)",
      mcpEnvKeysLabel: "Env Key Names (comma-separated, never values)",
      mcpEnvKeysHint: "Store environment variable names only (e.g. GITHUB_PERSONAL_ACCESS_TOKEN).",
      mcpSave: "Save MCP Server",
      pluginsAddHeading: "Register Custom Plugin",
      pluginsPresetsHeading: "Quick Plugin Catalog",
      pluginsEmpty: "No plugins registered yet. Install from the catalog or add a custom plugin.",
      pluginKindLabel: "Plugin Kind",
      pluginSourceLabel: "Source (GitHub repo, npm/cargo package, or path)",
      pluginDescLabel: "Short Description",
      pluginSave: "Save Plugin",
      installPreset: "Install",
      installedPreset: "Installed",
      enabled: "Enabled",
      disabled: "Disabled",
      remove: "Delete",
    },
    palette: {
      placeholder: "Type a command or session name...",
      empty: "No matching commands.",
      toggleSidebar: "Toggle Session Sidebar",
      toggleSidePanel: "Toggle Side Panel",
      toggleSplit: "Cycle Pane Layout (Single / Split / 2×2 Grid / Board)",
      openBoard: "Open Session Board (Mission Control)",
      openGrid: "Open 2×2 Terminal Grid",
      openAntigravityCli: "Launch Antigravity CLI (agy)",
      openNewSession: "New Agent Session",
      openAgents: "Manage Agents (OAuth & API Key)",
      openPlugins: "Open Integrations (GitHub, MCP, Plugins)",
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
