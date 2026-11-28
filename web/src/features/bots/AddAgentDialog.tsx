import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { Dialog } from "../../components/ui/Dialog";
import { Input } from "../../components/ui/Input";
import { Segmented } from "../../components/ui/Segmented";
import type { Catalog } from "../../lib/i18n/catalog";
import type { AgentAuthMode, AgentProfile } from "../../transport/protocol";

interface AddAgentDialogProps {
  open: boolean;
  labels: Catalog["agents"];
  busy: boolean;
  onClose: () => void;
  onSave: (id: string, profile: AgentProfile) => Promise<void>;
}

function splitWhitespace(raw: string): string[] {
  return raw
    .trim()
    .split(/\s+/)
    .filter((token) => token.length > 0);
}

export function AddAgentDialog({ open, labels, busy, onClose, onSave }: AddAgentDialogProps) {
  const [id, setId] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [command, setCommand] = useState("");
  const [argsText, setArgsText] = useState("");
  const [authMode, setAuthMode] = useState<AgentAuthMode>("oauth");
  const [loginCmdText, setLoginCmdText] = useState("");
  const [apiKeyEnv, setApiKeyEnv] = useState("");
  const [proxyProvider, setProxyProvider] = useState("");

  const reset = () => {
    setId("");
    setDisplayName("");
    setCommand("");
    setArgsText("");
    setAuthMode("oauth");
    setLoginCmdText("");
    setApiKeyEnv("");
    setProxyProvider("");
  };

  const handleSubmit = async (ev: React.FormEvent) => {
    ev.preventDefault();
    const cleanId = id.trim();
    const cleanCmd = command.trim();
    if (!cleanId || !cleanCmd) {
      return;
    }

    const profile: AgentProfile = {
      display_name: displayName.trim() || null,
      command: cleanCmd,
      args: splitWhitespace(argsText),
      env: {},
      pass_env: [],
      auth_mode: authMode,
      api_key_env: apiKeyEnv.trim() || null,
      login_command: splitWhitespace(loginCmdText),
      proxy_mode: authMode === "api_key" && proxyProvider ? "injected" : "bypass",
      proxy_provider: proxyProvider || null,
      resume_args: [],
      extra_rw: [],
      extra_ro: [],
      category: "custom",
      description: null,
    };

    await onSave(cleanId, profile);
    reset();
  };

  const authOptions = [
    { value: "oauth" as const, label: labels.oauthMode },
    { value: "api_key" as const, label: labels.apiKeyMode },
  ];

  return (
    <Dialog
      open={open}
      title={labels.customDialogTitle}
      titleId="add-agent-dialog-title"
      onClose={onClose}
    >
      <form onSubmit={(e) => void handleSubmit(e)} className="space-y-3 text-xs">
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label htmlFor="custom-agent-id" className="block">
            <span className="mb-1 block text-fg-2">{labels.idLabel}</span>
            <Input
              id="custom-agent-id"
              type="text"
              value={id}
              onChange={(e) => setId(e.target.value)}
              placeholder="my-agent"
              required
            />
          </label>
          <label htmlFor="custom-agent-name" className="block">
            <span className="mb-1 block text-fg-2">{labels.displayNameLabel}</span>
            <Input
              id="custom-agent-name"
              type="text"
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              placeholder="My Coding CLI"
            />
          </label>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <label htmlFor="custom-agent-cmd" className="block">
            <span className="mb-1 block text-fg-2">{labels.commandLabel}</span>
            <Input
              id="custom-agent-cmd"
              type="text"
              value={command}
              onChange={(e) => setCommand(e.target.value)}
              placeholder="my-cli"
              required
            />
          </label>
          <label htmlFor="custom-agent-args" className="block">
            <span className="mb-1 block text-fg-2">{labels.argsLabel}</span>
            <Input
              id="custom-agent-args"
              type="text"
              value={argsText}
              onChange={(e) => setArgsText(e.target.value)}
              placeholder="--interactive"
            />
          </label>
        </div>

        <div className="flex flex-wrap items-center justify-between gap-2 pt-1">
          <span className="text-fg-2">{labels.authLabel}</span>
          <Segmented
            label={labels.authLabel}
            value={authMode === "api_key" ? "api_key" : "oauth"}
            options={authOptions}
            onChange={(next) => setAuthMode(next)}
          />
        </div>

        {authMode === "oauth" ? (
          <label htmlFor="custom-agent-login" className="block">
            <span className="mb-1 block text-fg-2">{labels.loginCmdLabel}</span>
            <Input
              id="custom-agent-login"
              type="text"
              value={loginCmdText}
              onChange={(e) => setLoginCmdText(e.target.value)}
              placeholder="my-cli login"
            />
          </label>
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <label htmlFor="custom-agent-env" className="block">
              <span className="mb-1 block text-fg-2">{labels.apiKeyEnvLabel}</span>
              <Input
                id="custom-agent-env"
                type="text"
                value={apiKeyEnv}
                onChange={(e) => setApiKeyEnv(e.target.value)}
                placeholder="MY_AGENT_API_KEY"
              />
            </label>
            <label htmlFor="custom-agent-provider" className="block">
              <span className="mb-1 block text-fg-2">{labels.providerLabel}</span>
              <select
                id="custom-agent-provider"
                value={proxyProvider}
                onChange={(e) => setProxyProvider(e.target.value)}
                className="w-full rounded-md border border-line bg-app px-2.5 py-1.5 text-xs text-fg focus:border-accent focus:outline-none"
              >
                <option value="">Bypass (direct)</option>
                <option value="openai">openai</option>
                <option value="anthropic">anthropic</option>
                <option value="google">google</option>
              </select>
            </label>
          </div>
        )}

        <div className="flex justify-end gap-2 pt-2">
          <Button variant="ghost" onClick={onClose}>
            {labels.cancel}
          </Button>
          <Button type="submit" variant="primary" disabled={busy || !id.trim() || !command.trim()}>
            {labels.saveAgent}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
