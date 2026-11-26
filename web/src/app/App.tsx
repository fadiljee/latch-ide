import { CATALOGS } from "../lib/i18n/catalog";
import { useWorkbenchStore } from "../lib/store/useWorkbenchStore";

export function App() {
  const locale = useWorkbenchStore((s) => s.locale);
  const sessions = useWorkbenchStore((s) => s.sessions);
  const setLocale = useWorkbenchStore((s) => s.setLocale);
  const t = CATALOGS[locale];

  const columns = [
    { key: "running", title: t.columns.running, count: sessions.filter((s) => s.status === "running").length },
    {
      key: "waiting",
      title: t.columns.waitingApproval,
      count: sessions.filter((s) => s.status === "waiting_approval").length,
    },
    {
      key: "review",
      title: t.columns.readyReview,
      count: sessions.filter((s) => s.status === "completed").length,
    },
    {
      key: "attention",
      title: t.columns.attention,
      count: sessions.filter((s) => s.status === "failed" || s.status === "tripped").length,
    },
  ];

  return (
    <div className="min-h-screen bg-slate-950 text-slate-100">
      <header className="flex items-center justify-between border-b border-slate-800 px-6 py-4">
        <div>
          <h1 className="text-lg font-semibold tracking-tight">{t.appName}</h1>
          <p className="text-sm text-slate-400">{t.subtitle}</p>
        </div>
        <div className="flex items-center gap-4">
          <span className="text-xs text-amber-300" aria-label={t.connectionLabel}>
            {t.disconnected}
          </span>
          <button
            type="button"
            onClick={() => setLocale(locale === "id" ? "en" : "id")}
            aria-label={t.localeToggleLabel}
            className="rounded border border-slate-700 px-3 py-1 text-xs font-medium hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-cyan-400"
          >
            {locale.toUpperCase()}
          </button>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-6 py-6">
        <section aria-labelledby="board-heading">
          <h2 id="board-heading" className="mb-4 text-base font-medium text-slate-200">
            {t.boardHeading}
          </h2>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-4">
            {columns.map((col) => (
              <div
                key={col.key}
                className="rounded-lg border border-slate-800 bg-slate-900/60 p-4"
              >
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-sm font-medium">{col.title}</span>
                  <span className="rounded bg-slate-800 px-2 py-0.5 text-xs text-slate-300">
                    {col.count}
                  </span>
                </div>
                {sessions.length === 0 ? (
                  <p className="text-xs text-slate-400">{t.emptySessions}</p>
                ) : null}
              </div>
            ))}
          </div>
        </section>
      </main>
    </div>
  );
}
export default App;
