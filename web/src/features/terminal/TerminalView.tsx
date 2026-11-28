import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { Button } from "../../components/ui/Button";
import { StatusDot } from "../../components/StatusDot";
import { useThemeStore } from "../../theme/theme-store";
import { openPtyStream, type PtyStreamHandle } from "./pty-stream";
import { readTerminalTheme } from "./terminal-theme";

const MAX_PAUSED_BUFFER_BYTES = 64 * 1024;

interface TerminalViewProps {
  token: string;
  sessionId: string;
  workdir: string;
  agentProfile: string;
  resyncLabel: string;
  inputPlaceholder: string;
  focused?: boolean;
  onFocus?: () => void;
}

export function TerminalView({
  token,
  sessionId,
  workdir,
  agentProfile,
  resyncLabel,
  inputPlaceholder,
  focused = true,
  onFocus,
}: TerminalViewProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const streamRef = useRef<PtyStreamHandle | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const focusedRef = useRef(focused);
  const onFocusRef = useRef(onFocus);
  const visibleRef = useRef(true);
  const pausedChunksRef = useRef<Array<string | Uint8Array>>([]);
  const pausedBytesRef = useRef(0);
  const [live, setLive] = useState(false);
  const [lineInput, setLineInput] = useState("");
  const style = useThemeStore((s) => s.style);
  const mode = useThemeStore((s) => s.resolvedMode);
  const accent = useThemeStore((s) => s.accent);

  useEffect(() => {
    focusedRef.current = focused;
  }, [focused]);

  useEffect(() => {
    onFocusRef.current = onFocus;
  }, [onFocus]);

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

    const flushPaused = () => {
      if (pausedChunksRef.current.length === 0) {
        return;
      }
      if (pausedBytesRef.current >= MAX_PAUSED_BUFFER_BYTES) {
        pausedChunksRef.current = [];
        pausedBytesRef.current = 0;
        streamRef.current?.requestResync();
        return;
      }
      for (const chunk of pausedChunksRef.current) {
        term.write(chunk);
      }
      pausedChunksRef.current = [];
      pausedBytesRef.current = 0;
    };

    const updateVisibility = (isVisible: boolean) => {
      const wasHidden = !visibleRef.current;
      visibleRef.current = isVisible && document.visibilityState !== "hidden";
      if (wasHidden && visibleRef.current) {
        flushPaused();
      }
    };

    let stream: PtyStreamHandle | null = null;
    if (token) {
      stream = openPtyStream(token, sessionId, {
        onData: (chunk) => {
          if (!visibleRef.current) {
            if (pausedBytesRef.current < MAX_PAUSED_BUFFER_BYTES) {
              pausedChunksRef.current.push(chunk);
              pausedBytesRef.current +=
                typeof chunk === "string" ? chunk.length : chunk.byteLength;
            }
            return;
          }
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

    // Input hanya diteruskan bila sel terminal ini sedang fokus (IT-110).
    const disposable = term.onData((data) => {
      if (focusedRef.current) {
        stream?.sendInput(data);
      }
    });

    const onVisibilityChange = () => {
      updateVisibility(document.visibilityState !== "hidden");
    };
    document.addEventListener("visibilitychange", onVisibilityChange);

    let intersectionObserver: IntersectionObserver | null = null;
    if (typeof IntersectionObserver !== "undefined") {
      intersectionObserver = new IntersectionObserver((entries) => {
        const entry = entries[0];
        if (entry) {
          updateVisibility(entry.isIntersecting);
        }
      });
      intersectionObserver.observe(el);
    }

    const observer = new ResizeObserver(() => {
      if (!visibleRef.current) {
        return;
      }
      try {
        fitAddon.fit();
        stream?.sendResize(term.cols, term.rows);
      } catch {
        // Abaikan bila belum terpasang
      }
    });
    observer.observe(el);

    const onContainerActivate = () => {
      onFocusRef.current?.();
    };
    el.addEventListener("pointerdown", onContainerActivate);
    el.addEventListener("focusin", onContainerActivate);

    return () => {
      el.removeEventListener("pointerdown", onContainerActivate);
      el.removeEventListener("focusin", onContainerActivate);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      intersectionObserver?.disconnect();
      observer.disconnect();
      disposable.dispose();
      stream?.close();
      streamRef.current = null;
      termRef.current = null;
      pausedChunksRef.current = [];
      pausedBytesRef.current = 0;
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
    if (!focused) {
      return;
    }
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

      <div ref={containerRef} className="min-h-[180px] flex-1 overflow-hidden p-2" />

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
          disabled={!focused}
          onChange={(e) => setLineInput(e.target.value)}
          placeholder={inputPlaceholder}
          aria-label={inputPlaceholder}
          className="min-w-0 flex-1 bg-transparent font-mono text-xs text-fg placeholder:text-fg-3 focus:outline-none disabled:opacity-50"
        />
      </form>
    </div>
  );
}

export default TerminalView;
