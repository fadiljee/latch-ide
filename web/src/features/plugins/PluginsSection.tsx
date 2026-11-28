import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useNotificationsStore } from "../../state/notifications";
import { deletePlugin, upsertPlugin } from "../../transport/daemon-client";
import type { PluginConfig, PluginKind } from "../../transport/protocol";
import { PLUGIN_PRESETS } from "./presets";

interface PluginsSectionProps {
  labels: Catalog["integrations"];
}

export function PluginsSection({ labels }: PluginsSectionProps) {
  const token = useHostsStore((s) => s.token);
  const plugins = useIntegrationsStore((s) => s.plugins);
  const upsertPluginLocal = useIntegrationsStore((s) => s.upsertPluginLocal);
  const removePluginLocal = useIntegrationsStore((s) => s.removePluginLocal);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [kind, setKind] = useState<PluginKind>("mcp_bundle");
  const [source, setSource] = useState("");
  const [description, setDescription] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);

  const connected = Boolean(token);
  const installedIds = new Set(plugins.map((p) => p.id));

  const savePlugin = async (plugin: PluginConfig, resetForm: boolean) => {
    if (!token) {
      return;
    }
    setBusyId(plugin.id);
    try {
      const saved = await upsertPlugin(token, plugin);
      upsertPluginLocal(saved);
      if (resetForm) {
        setId("");
        setName("");
        setSource("");
        setDescription("");
      }
      pushToast("success", `Plugin ${saved.name} saved`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed saving plugin");
    } finally {
      setBusyId(null);
    }
  };

  const handleToggle = (plugin: PluginConfig) => {
    void savePlugin({ ...plugin, enabled: !plugin.enabled }, false);
  };

  const handleRemove = (pluginId: string) => {
    if (!token) {
      return;
    }
    setBusyId(pluginId);
    void deletePlugin(token, pluginId)
      .then(() => {
        removePluginLocal(pluginId);
        pushToast("info", `Removed plugin ${pluginId}`);
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed deleting plugin"),
      )
      .finally(() => setBusyId(null));
  };

  const handleSubmit = (ev: React.FormEvent) => {
    ev.preventDefault();
    const cleanId = id.trim();
    const cleanName = name.trim();
    const cleanSource = source.trim();
    if (!cleanId || !cleanName || !cleanSource) {
      return;
    }
    const plugin: PluginConfig = {
      id: cleanId,
      name: cleanName,
      kind,
      source: cleanSource,
      command: null,
      enabled: true,
      description: description.trim() || null,
    };
    void savePlugin(plugin, true);
  };

  return (
    <div className="space-y-5">
      <section className="space-y-2.5">
        <h3 className="text-sm font-semibold text-fg">{labels.pluginsPresetsHeading}</h3>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {PLUGIN_PRESETS.map((preset) => {
            const installed = installedIds.has(preset.id);
            return (
              <div
                key={preset.id}
                className="flex flex-col justify-between gap-2.5 rounded-lg border border-line bg-surface p-3.5 shadow-[var(--panel-shadow)]"
              >
                <div className="space-y-1">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-semibold text-fg">{preset.name}</span>
                    <span className="rounded-sm border border-line bg-raised px-1.5 py-0.5 font-mono text-[10px] text-fg-2">
                      {preset.kind}
                    </span>
                  </div>
                  {preset.description ? (
                    <p className="text-fg-2 leading-relaxed">{preset.description}</p>
                  ) : null}
                  <p className="truncate font-mono text-[11px] text-fg-3">{preset.source}</p>
                </div>
                <div className="flex justify-end pt-1">
                  <Button
                    size="sm"
                    variant={installed ? "secondary" : "primary"}
                    disabled={!connected || installed || busyId === preset.id}
                    onClick={() => void savePlugin(preset, false)}
                  >
                    {installed ? labels.installedPreset : labels.installPreset}
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      </section>

      <section className="space-y-2.5">
        {plugins.length === 0 ? (
          <p role="status" className="rounded-lg border border-line bg-surface p-4 text-fg-3">
            {labels.pluginsEmpty}
          </p>
        ) : (
          <ul className="space-y-2">
            {plugins.map((plugin) => (
              <li
                key={plugin.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-line bg-surface px-4 py-3"
              >
                <div className="min-w-0 space-y-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-semibold text-fg">{plugin.name}</span>
                    <span className="rounded-sm border border-line bg-raised px-1.5 py-0.5 font-mono text-[10px] text-fg-2">
                      {plugin.id}
                    </span>
                    <span className="rounded-sm border border-line bg-app px-1.5 py-0.5 font-mono text-[10px] text-fg-3">
                      {plugin.kind}
                    </span>
                  </div>
                  <p className="truncate font-mono text-[11px] text-fg-3">{plugin.source}</p>
                  {plugin.description ? <p className="text-fg-2">{plugin.description}</p> : null}
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant={plugin.enabled ? "primary" : "secondary"}
                    aria-pressed={plugin.enabled}
                    disabled={!connected || busyId === plugin.id}
                    onClick={() => handleToggle(plugin)}
                  >
                    {plugin.enabled ? labels.enabled : labels.disabled}
                  </Button>
                  <Button
                    size="sm"
                    variant="danger"
                    disabled={!connected || busyId === plugin.id}
                    onClick={() => handleRemove(plugin.id)}
                  >
                    {labels.remove}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      <form
        onSubmit={handleSubmit}
        className="space-y-3 rounded-lg border border-line bg-surface p-4"
      >
        <h3 className="text-sm font-semibold text-fg">{labels.pluginsAddHeading}</h3>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label htmlFor="plugin-id" className="block">
            <span className="mb-1 block text-fg-2">{labels.mcpIdLabel}</span>
            <Input
              id="plugin-id"
              type="text"
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder="my-plugin"
              required
            />
          </label>
          <label htmlFor="plugin-name" className="block">
            <span className="mb-1 block text-fg-2">{labels.mcpNameLabel}</span>
            <Input
              id="plugin-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My Plugin"
              required
            />
          </label>
          <label htmlFor="plugin-kind" className="block">
            <span className="mb-1 block text-fg-2">{labels.pluginKindLabel}</span>
            <select
              id="plugin-kind"
              value={kind}
              onChange={(e) => setKind(e.target.value as PluginKind)}
              className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
            >
              <option value="mcp_bundle">mcp_bundle</option>
              <option value="agent_adapter">agent_adapter</option>
              <option value="workflow_pack">workflow_pack</option>
              <option value="hook">hook</option>
            </select>
          </label>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label htmlFor="plugin-source" className="block">
            <span className="mb-1 block text-fg-2">{labels.pluginSourceLabel}</span>
            <Input
              id="plugin-source"
              type="text"
              value={source}
              onChange={(e) => setSource(e.target.value)}
              placeholder="github:org/latch-plugin"
              required
            />
          </label>
          <label htmlFor="plugin-desc" className="block">
            <span className="mb-1 block text-fg-2">{labels.pluginDescLabel}</span>
            <Input
              id="plugin-desc"
              type="text"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional summary..."
            />
          </label>
        </div>

        <div className="flex justify-end">
          <Button
            type="submit"
            variant="primary"
            disabled={!connected || busyId !== null || !id.trim() || !name.trim() || !source.trim()}
          >
            {labels.pluginSave}
          </Button>
        </div>
      </form>
    </div>
  );
}
