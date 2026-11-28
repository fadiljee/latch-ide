import { CommandPalette } from "../components/CommandPalette";
import { ToastList } from "../components/ui/Toast";
import { NewSessionDialog } from "../features/sessions/NewSessionDialog";
import { UpdateBanner } from "../components/UpdateBanner";
import { SidePanel } from "../layout/SidePanel";
import { Sidebar } from "../layout/Sidebar";
import { StatusBar } from "../layout/StatusBar";
import { TitleBar } from "../layout/TitleBar";
import { Workspace } from "../layout/Workspace";
import { CATALOGS } from "../lib/i18n/catalog";
import { useUiStore } from "../state/ui";

export function AppShell() {
  const locale = useUiStore((s) => s.locale);
  const t = CATALOGS[locale];

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-app text-fg">
      <TitleBar />
      <UpdateBanner />
      <div className="flex min-h-0 flex-1 flex-col overflow-hidden md:flex-row">
        <Sidebar />
        <Workspace />
        <SidePanel />
      </div>
      <StatusBar />
      <CommandPalette />
      <NewSessionDialog
        labels={t.newSession}
        repoPlaceholder={t.sidebar.addRepoPlaceholder}
        addRepoLabel={t.sidebar.addRepoButton}
      />
      <ToastList />
    </div>
  );
}
