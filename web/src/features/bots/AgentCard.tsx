import { useState } from "react";
import { ModelBadge } from "../../components/ModelBadge";
import { Button } from "../../components/ui/Button";
import { Input } from "../../components/ui/Input";
import { Segmented } from "../../components/ui/Segmented";
import type { Catalog } from "../../lib/i18n/catalog";
import type { AgentAuthMode, AgentProfileStatus } from "../../transport/protocol";

interface AgentCardProps {
  agent: AgentProfileStatus;
  activeSessions: number;
  labels: Catalog["agents"];
  connected: boolean;
  busy: boolean;
  onSwitchAuthMode: (agent: AgentProfileStatus, mode: AgentAuthMode) => void;
  onSaveRuntimeKey: (agent: AgentProfileStatus, apiKey: string) => Promise<void>;
  onClearRuntimeKey: (agent: AgentProfileStatus) => Promise<void>;
  onOAuthLogin: (agent: AgentProfileStatus) => void;
  onLaunch: (agent: AgentProfileStatus) => void;
  onDelete: (agent: AgentProfileStatus) => void;
}

export function AgentCard({
  agent,
  activeSessions,
  labels,
  connected,
  busy,
  onSwitchAuthMode,
  onSaveRuntimeKey,
  onClearRuntimeKey,
  onOAuthLogin,
  onLaunch,
  onDelete,
}: AgentCardProps) {
  const [keyDraft, setKeyDraft] = useState("");
  const { profile } = agent;
  const isShell = profile.auth_mode === "none";

  const authOptions = [
    { value: "oauth" as const, label: labels.oauthMode },
    { value: "api_key" as const, label: labels.apiKeyMode },
  ];

  const apiKeyBadgeText =
    agent.api_key_source === "env"
      ? labels.apiKeyConfiguredEnv
      : agent.api_key_source === "runtime_memory"
        ? labels.apiKeyConfiguredMemory
        : labels.apiKeyMissing;

  const handleSaveKey = async (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!keyDraft.trim()) {
      return;
    }
    await onSaveRuntimeKey(agent, keyDraft.trim());
    setKeyDraft("");
  };

  return (
    <article className="flex flex-col justify-between gap-3 rounded-lg border border-line bg-surface p-4 text-xs shadow-[var(--panel-shadow)]">
      <div className="space-y-2.5">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <h3 className="truncate text-sm font-semibold text-fg">
              {profile.display_name || agent.id}
            </h3>
            <div className="mt-1 flex flex-wrap items-center gap-1.5">
              <ModelBadge profile={agent.id} />
              {profile.proxy_provider ? (
                <span className="font-mono text-[11px] text-fg-3">{profile.proxy_provider}</span>
              ) : null}
            </div>
          </div>

          <span
            className={
              agent.installed
                ? "shrink-0 rounded-sm border border-done/40 bg-done/15 px-2 py-0.5 font-mono text-[11px] text-done"
                : "shrink-0 rounded-sm border border-line bg-raised px-2 py-0.5 font-mono text-[11px] text-fg-3"
            }
          >
            {agent.installed ? labels.installed : labels.notInstalled}
          </span>
        </div>

        {profile.description ? (
          <p className="text-fg-2 leading-relaxed">{profile.description}</p>
        ) : null}

        <div className="flex flex-wrap items-center justify-between gap-2 font-mono text-[11px] text-fg-3">
          <span className="truncate" title={agent.command_path ?? profile.command}>
            $ {profile.command}
            {profile.args.length > 0 ? ` ${profile.args.join(" ")}` : ""}
          </span>
          <span>
            {labels.activeSessions}: <strong className="text-fg">{activeSessions}</strong>
          </span>
        </div>

        {!isShell ? (
          <div className="space-y-2 border-t border-line pt-2.5">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <span className="text-fg-2">{labels.authLabel}</span>
              <Segmented
                label={`${labels.authLabel} ${agent.id}`}
                value={profile.auth_mode === "api_key" ? "api_key" : "oauth"}
                options={authOptions}
                onChange={(next) => {
                  if (connected && !busy && next !== profile.auth_mode) {
                    onSwitchAuthMode(agent, next);
                  }
                }}
              />
            </div>

            {profile.auth_mode === "api_key" ? (
              <div className="space-y-2 rounded-md border border-line bg-app p-2.5">
                <div className="flex flex-wrap items-center justify-between gap-1.5">
                  <span className="font-mono text-[11px] text-fg-2">
                    {labels.apiKeyEnvLabel}:{" "}
                    <strong className="text-fg">{profile.api_key_env || "—"}</strong>
                  </span>
                  <span
                    className={
                      agent.api_key_configured
                        ? "rounded-sm border border-done/40 bg-done/15 px-1.5 py-0.5 font-mono text-[10px] text-done"
                        : "rounded-sm border border-needs/40 bg-needs/15 px-1.5 py-0.5 font-mono text-[10px] text-needs"
                    }
                  >
                    {apiKeyBadgeText}
                  </span>
                </div>

                {profile.api_key_env ? (
                  <form onSubmit={(e) => void handleSaveKey(e)} className="space-y-1.5">
                    <div className="flex gap-1.5">
                      <Input
                        id={`runtime-key-${agent.id}`}
                        type="password"
                        value={keyDraft}
                        onChange={(e) => setKeyDraft(e.target.value)}
                        placeholder={labels.runtimeKeyPlaceholder}
                        aria-label={`${labels.apiKeyEnvLabel} ${agent.id}`}
                        disabled={!connected || busy}
                      />
                      <Button
                        type="submit"
                        size="sm"
                        variant="secondary"
                        disabled={!connected || busy || !keyDraft.trim()}
                      >
                        {labels.saveRuntimeKey}
                      </Button>
                      {agent.api_key_source === "runtime_memory" ? (
                        <Button
                          size="sm"
                          variant="ghost"
                          disabled={!connected || busy}
                          onClick={() => void onClearRuntimeKey(agent)}
                        >
                          {labels.clearRuntimeKey}
                        </Button>
                      ) : null}
                    </div>
                    <p className="text-[11px] text-fg-3">{labels.runtimeKeyHint}</p>
                  </form>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null}
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 border-t border-line pt-3">
        <div className="flex flex-wrap items-center gap-1.5">
          <Button size="sm" variant="primary" onClick={() => onLaunch(agent)}>
            {labels.launchSession}
          </Button>
          {profile.auth_mode === "oauth" ? (
            <Button
              size="sm"
              variant="secondary"
              disabled={!connected || busy || !agent.installed}
              onClick={() => onOAuthLogin(agent)}
            >
              {labels.loginOauth}
            </Button>
          ) : null}
        </div>

        {!agent.builtin ? (
          <Button
            size="sm"
            variant="danger"
            disabled={!connected || busy}
            onClick={() => onDelete(agent)}
          >
            {labels.removeCustom}
          </Button>
        ) : null}
      </div>
    </article>
  );
}
