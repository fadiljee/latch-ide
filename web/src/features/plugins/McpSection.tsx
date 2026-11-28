import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import { Segmented } from "../../components/ui/Segmented";
import type { Catalog } from "../../lib/i18n/catalog";
import { useHostsStore } from "../../state/hosts";
import { useIntegrationsStore } from "../../state/integrations";
import { useNotificationsStore } from "../../state/notifications";
import { deleteMcpServer, upsertMcpServer } from "../../transport/daemon-client";
import type { McpServerConfig, McpTransport } from "../../transport/protocol";
import { MCP_PRESETS } from "./presets";

interface McpSectionProps {
  labels: Catalog["integrations"];
}

export function McpSection({ labels }: McpSectionProps) {
  const token = useHostsStore((s) => s.token);
  const mcpServers = useIntegrationsStore((s) => s.mcpServers);
  const upsertMcpLocal = useIntegrationsStore((s) => s.upsertMcpLocal);
  const removeMcpLocal = useIntegrationsStore((s) => s.removeMcpLocal);
  const pushToast = useNotificationsStore((s) => s.pushToast);

  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [transport, setTransport] = useState<McpTransport>("stdio");
  const [command, setCommand] = useState("");
  const [argsText, setArgsText] = useState("");
  const [url, setUrl] = useState("");
  const [envKeysText, setEnvKeysText] = useState("");
  const [busyId, setBusyId] = useState<string | null>(null);

  const connected = Boolean(token);
  const installedIds = new Set(mcpServers.map((s) => s.id));

  const saveServer = async (server: McpServerConfig, resetForm: boolean) => {
    if (!token) {
      return;
    }
    setBusyId(server.id);
    try {
      const saved = await upsertMcpServer(token, server);
      upsertMcpLocal(saved);
      if (resetForm) {
        setId("");
        setName("");
        setCommand("");
        setArgsText("");
        setUrl("");
        setEnvKeysText("");
      }
      pushToast("success", `MCP ${saved.name} saved`);
    } catch (err) {
      pushToast("danger", err instanceof Error ? err.message : "Failed saving MCP server");
    } finally {
      setBusyId(null);
    }
  };

  const handleToggle = (server: McpServerConfig) => {
    void saveServer({ ...server, enabled: !server.enabled }, false);
  };

  const handleRemove = (serverId: string) => {
    if (!token) {
      return;
    }
    setBusyId(serverId);
    void deleteMcpServer(token, serverId)
      .then(() => {
        removeMcpLocal(serverId);
        pushToast("info", `Removed MCP ${serverId}`);
      })
      .catch((err) =>
        pushToast("danger", err instanceof Error ? err.message : "Failed deleting MCP server"),
      )
      .finally(() => setBusyId(null));
  };

  const handleSubmit = (ev: React.FormEvent) => {
    ev.preventDefault();
    const cleanId = id.trim();
    const cleanName = name.trim();
    if (!cleanId || !cleanName) {
      return;
    }
    const server: McpServerConfig = {
      id: cleanId,
      name: cleanName,
      transport,
      command: transport === "stdio" ? command.trim() || null : null,
      args:
        transport === "stdio"
          ? argsText
              .trim()
              .split(/\s+/)
              .filter((s) => s.length > 0)
          : [],
      url: transport === "http" ? url.trim() || null : null,
      env_keys: envKeysText
        .split(",")
        .map((k) => k.trim())
        .filter((k) => k.length > 0),
      enabled: true,
      description: null,
    };
    void saveServer(server, true);
  };

  const transportOptions = [
    { value: "stdio" as const, label: "stdio" },
    { value: "http" as const, label: "http" },
  ];

  const canSubmit =
    connected &&
    busyId === null &&
    Boolean(id.trim()) &&
    Boolean(name.trim()) &&
    (transport === "stdio" ? Boolean(command.trim()) : Boolean(url.trim()));

  return (
    <div className="space-y-5">
      <section className="space-y-2.5">
        <h3 className="text-sm font-semibold text-fg">{labels.mcpPresetsHeading}</h3>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {MCP_PRESETS.map((preset) => {
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
                      {preset.transport}
                    </span>
                  </div>
                  {preset.description ? (
                    <p className="text-fg-2 leading-relaxed">{preset.description}</p>
                  ) : null}
                  <p className="truncate font-mono text-[11px] text-fg-3">
                    {preset.command} {preset.args.join(" ")}
                  </p>
                </div>
                <div className="flex items-center justify-between gap-2 pt-1">
                  <span className="truncate font-mono text-[10px] text-fg-3">
                    {preset.env_keys.join(", ")}
                  </span>
                  <Button
                    size="sm"
                    variant={installed ? "secondary" : "primary"}
                    disabled={!connected || installed || busyId === preset.id}
                    onClick={() => void saveServer(preset, false)}
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
        {mcpServers.length === 0 ? (
          <p role="status" className="rounded-lg border border-line bg-surface p-4 text-fg-3">
            {labels.mcpEmpty}
          </p>
        ) : (
          <ul className="space-y-2">
            {mcpServers.map((srv) => (
              <li
                key={srv.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-line bg-surface px-4 py-3"
              >
                <div className="min-w-0 space-y-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-semibold text-fg">{srv.name}</span>
                    <span className="rounded-sm border border-line bg-raised px-1.5 py-0.5 font-mono text-[10px] text-fg-2">
                      {srv.id}
                    </span>
                    <span className="rounded-sm border border-line bg-app px-1.5 py-0.5 font-mono text-[10px] text-fg-3">
                      {srv.transport}
                    </span>
                  </div>
                  <p className="truncate font-mono text-[11px] text-fg-3">
                    {srv.transport === "stdio"
                      ? `${srv.command ?? ""} ${srv.args.join(" ")}`.trim()
                      : (srv.url ?? "")}
                  </p>
                  {srv.env_keys.length > 0 ? (
                    <div className="flex flex-wrap gap-1">
                      {srv.env_keys.map((key) => (
                        <span
                          key={key}
                          className="rounded-sm border border-line bg-app px-1.5 py-0.5 font-mono text-[10px] text-fg-2"
                        >
                          {key}
                        </span>
                      ))}
                    </div>
                  ) : null}
                </div>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant={srv.enabled ? "primary" : "secondary"}
                    aria-pressed={srv.enabled}
                    disabled={!connected || busyId === srv.id}
                    onClick={() => handleToggle(srv)}
                  >
                    {srv.enabled ? labels.enabled : labels.disabled}
                  </Button>
                  <Button
                    size="sm"
                    variant="danger"
                    disabled={!connected || busyId === srv.id}
                    onClick={() => handleRemove(srv.id)}
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
        <h3 className="text-sm font-semibold text-fg">{labels.mcpAddHeading}</h3>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <label htmlFor="mcp-id" className="block">
            <span className="mb-1 block text-fg-2">{labels.mcpIdLabel}</span>
            <Input
              id="mcp-id"
              type="text"
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder="my-mcp"
              required
            />
          </label>
          <label htmlFor="mcp-name" className="block">
            <span className="mb-1 block text-fg-2">{labels.mcpNameLabel}</span>
            <Input
              id="mcp-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My MCP Server"
              required
            />
          </label>
          <div className="space-y-1">
            <span className="block text-fg-2">{labels.mcpTransportLabel}</span>
            <Segmented
              label={labels.mcpTransportLabel}
              value={transport}
              options={transportOptions}
              onChange={setTransport}
            />
          </div>
        </div>

        {transport === "stdio" ? (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <label htmlFor="mcp-cmd" className="block">
              <span className="mb-1 block text-fg-2">{labels.mcpCommandLabel}</span>
              <Input
                id="mcp-cmd"
                type="text"
                value={command}
                onChange={(e) => setCommand(e.target.value)}
                placeholder="npx"
                required
              />
            </label>
            <label htmlFor="mcp-args" className="block">
              <span className="mb-1 block text-fg-2">{labels.mcpArgsLabel}</span>
              <Input
                id="mcp-args"
                type="text"
                value={argsText}
                onChange={(e) => setArgsText(e.target.value)}
                placeholder="-y @org/mcp-server"
              />
            </label>
          </div>
        ) : (
          <label htmlFor="mcp-url" className="block">
            <span className="mb-1 block text-fg-2">{labels.mcpUrlLabel}</span>
            <Input
              id="mcp-url"
              type="url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="http://127.0.0.1:8080/mcp"
              required
            />
          </label>
        )}

        <label htmlFor="mcp-env-keys" className="block">
          <span className="mb-1 block text-fg-2">{labels.mcpEnvKeysLabel}</span>
          <Input
            id="mcp-env-keys"
            type="text"
            value={envKeysText}
            onChange={(e) => setEnvKeysText(e.target.value)}
            placeholder="GITHUB_PERSONAL_ACCESS_TOKEN, DATABASE_URL"
          />
          <span className="mt-1 block text-[11px] text-fg-3">{labels.mcpEnvKeysHint}</span>
        </label>

        <div className="flex justify-end">
          <Button type="submit" variant="primary" disabled={!canSubmit}>
            {labels.mcpSave}
          </Button>
        </div>
      </form>
    </div>
  );
}
