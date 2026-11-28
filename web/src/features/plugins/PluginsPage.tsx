import { useState } from "react";
import { TabBar, type TabOption } from "../../components/TabBar";
import type { Catalog } from "../../lib/i18n/catalog";
import { useIntegrationsStore } from "../../state/integrations";
import { GitHubSection } from "./GitHubSection";
import { McpSection } from "./McpSection";
import { PluginsSection } from "./PluginsSection";

type IntegrationsTab = "github" | "mcp" | "plugins";

interface PluginsPageProps {
  labels: Catalog["integrations"];
}

export function PluginsPage({ labels }: PluginsPageProps) {
  const [tab, setTab] = useState<IntegrationsTab>("github");
  const mcpCount = useIntegrationsStore((s) => s.mcpServers.filter((m) => m.enabled).length);
  const pluginCount = useIntegrationsStore((s) => s.plugins.filter((p) => p.enabled).length);

  const tabs: readonly TabOption<IntegrationsTab>[] = [
    { id: "github", label: labels.githubTab },
    { id: "mcp", label: labels.mcpTab, badge: mcpCount },
    { id: "plugins", label: labels.pluginsTab, badge: pluginCount },
  ];

  return (
    <div className="flex-1 space-y-5 overflow-y-auto p-5 text-xs">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-base font-semibold text-fg">{labels.heading}</h2>
          <p className="mt-0.5 max-w-2xl text-fg-3">{labels.subtitle}</p>
        </div>
        <TabBar label={labels.heading} tabs={tabs} active={tab} onSelect={setTab} />
      </div>

      <div role="tabpanel">
        {tab === "github" ? (
          <GitHubSection labels={labels} />
        ) : tab === "mcp" ? (
          <McpSection labels={labels} />
        ) : (
          <PluginsSection labels={labels} />
        )}
      </div>
    </div>
  );
}
