import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { Button } from "../../components/ui/Button";
import { StatusDot } from "../../components/StatusDot";
import { useThemeStore } from "../../theme/theme-store";
import { openPtyStream, type PtyStreamHandle } from "./pty-stream";
import { readTerminalTheme } from "./terminal-theme";

interface TerminalViewProps {
  token: string;
  sessionId: string;
  workdir: string;
  agentProfile: string;
  resyncLabel: string;
  inputPlaceholder: string;
}

export function TerminalView({
  token,
  sessionId,
  workdir,
  agentProfile,
  resyncLabel,
  inputPlaceholder,
}: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const streamRef = useRef<PtyStreamHandle | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const [live, setLive] = useState(false);
  const [lineInput, setLineInput] = useState("");
  const style = useThemeStore((s) => s.style);
  const mode = useThemeStore((s) => s.resolvedMode);
  const accent = useThemeStore((s) => s.accent);

  useEffect(() => {
    const el = containerRef.current;
    if (!el) {
      return;
    }

    const term = new Terminal({
      cursorBlink: false,
      fontSize: 13,
      fontFamily: "var(--font-code)",
      theme: readTerminalTheme(),
      scrollback: 2000,
      convertEol: true,
    });
    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);

    try {
      term.open(el);
      fitAddon.fit();
    } catch {
      // Abaikan bila lingkungan tes tanpa dukungan canvas penuh
    }
    termRef.current = term;

    let stream: PtyStreamHandle | null = null;
    if (token) {
      stream = openPtyStream(token, sessionId, {
        onData: (chunk) => {
          term.write(chunk);
        },
        onConnectionChange: (isLive) => {
          setLive(isLive);
          if (isLive) {
            stream?.sendResize(term.cols, term.rows);
          }
        },
      });
      streamRef.current = stream;
    }

    const disposable = term.onData((data) => {
      stream?.sendInput(data);
    });

    const observer = new ResizeObserver(() => {
      try {
        fitAddon.fit();
        stream?.sendResize(term.cols, term.rows);
      } catch {
        // Abaikan bila belum terpasang
      }
    });
    observer.observe(el);

    return () => {
      observer.disconnect();
      disposable.dispose();
      stream?.close();
      streamRef.current = null;
      termRef.current = null;
      term.dispose();
    };
  }, [token, sessionId]);

  useEffect(() => {
    if (termRef.current) {
      termRef.current.options.theme = readTerminalTheme();
    }
  }, [style, mode, accent]);

  const handleSendLine = (ev: React.FormEvent) => {
    ev.preventDefault();
    streamRef.current?.sendInput(lineInput ? `${lineInput}\r` : "\r");
    setLineInput("");
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-terminal text-terminal-fg">
      <div className="flex items-center justify-between gap-2 border-b border-line bg-surface px-3 py-1.5 text-xs">
        <div className="flex min-w-0 items-center gap-2">
          <StatusDot status={live ? "done" : "idle"} label={live ? "PTY live" : "PTY offline"} />
          <span className="font-medium text-fg">{agentProfile}</span>
          <span className="hidden truncate font-mono text-[11px] text-fg-3 sm:inline">
            {workdir}
          </span>
        </div>
        <Button size="sm" variant="ghost" onClick={() => streamRef.current?.requestResync()}>
          {resyncLabel}
        </Button>
      </div>

      <div ref={containerRef} className="min-h-[260px] flex-1 overflow-hidden p-2" />

      <form
        onSubmit={handleSendLine}
        className="flex items-center gap-2 border-t border-line bg-surface px-3 py-2"
      >
        <span className="font-mono text-xs text-fg-3" aria-hidden="true">
          &gt;
        </span>
        <input
          type="text"
          value={lineInput}
          onChange={(e) => setLineInput(e.target.value)}
          placeholder={inputPlaceholder}
          aria-label={inputPlaceholder}
          className="min-w-0 flex-1 bg-transparent font-mono text-xs text-fg placeholder:text-fg-3 focus:outline-none"
        />
      </form>
    </div>
  );
}

export default TerminalView;
