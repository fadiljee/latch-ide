import { useEffect, useRef, useState } from "react";
import { connectPtySocket, type PtyConnection } from "../../lib/api/client";

interface TerminalPaneProps {
  token: string;
  sessionId: string;
  workdir: string;
  agentProfile: string;
  sendPlaceholder: string;
}

function stripAnsi(raw: string): string {
  let out = "";
  let i = 0;
  while (i < raw.length) {
    if (raw.charCodeAt(i) === 27) {
      const next = raw[i + 1];
      if (next === "[") {
        i += 2;
        while (i < raw.length) {
          const code = raw.charCodeAt(i);
          i += 1;
          if (code >= 0x40 && code <= 0x7e) {
            break;
          }
        }
        continue;
      }
      if (next === "]") {
        i += 2;
        while (i < raw.length && raw.charCodeAt(i) !== 7) {
          i += 1;
        }
        i += 1;
        continue;
      }
    }
    out += raw[i] ?? "";
    i += 1;
  }
  return out;
}

export function TerminalPane({
  token,
  sessionId,
  workdir,
  agentProfile,
  sendPlaceholder,
}: TerminalPaneProps) {
  const [buffer, setBuffer] = useState("");
  const [input, setInput] = useState("");
  const [live, setLive] = useState(false);
  const connRef = useRef<PtyConnection | null>(null);
  const scrollRef = useRef<HTMLPreElement | null>(null);

  useEffect(() => {
    setBuffer("");
    if (!token) {
      return;
    }
    const conn = connectPtySocket(
      token,
      sessionId,
      (chunk) => {
        setBuffer((prev) => {
          const next = prev + stripAnsi(chunk);
          return next.length > 120_000 ? next.slice(next.length - 120_000) : next;
        });
      },
      (isLive) => setLive(isLive),
    );
    connRef.current = conn;
    return () => {
      conn.close();
      connRef.current = null;
    };
  }, [token, sessionId]);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [buffer]);

  const handleSend = (ev: React.FormEvent) => {
    ev.preventDefault();
    if (!input.trim() && input.length === 0) {
      connRef.current?.sendInput("\r");
      return;
    }
    connRef.current?.sendInput(`${input}\r`);
    setInput("");
  };

  return (
    <div className="flex min-h-0 flex-1 flex-col bg-[#090b10]">
      <div className="flex items-center justify-between border-b border-slate-800/80 bg-[#121622] px-4 py-2 text-xs">
        <div className="flex items-center gap-2">
          <span
            className={`inline-block h-2 w-2 rounded-full ${live ? "bg-emerald-400" : "bg-slate-500"}`}
            aria-hidden="true"
          />
          <span className="font-medium text-slate-200">{agentProfile}</span>
          <span className="hidden font-mono text-[11px] text-slate-500 sm:inline">
            cwd: {workdir}
          </span>
        </div>
        <span className="font-mono text-[11px] text-slate-500">{sessionId.slice(0, 8)}</span>
      </div>

      <pre
        ref={scrollRef}
        className="min-h-[320px] flex-1 overflow-auto p-4 font-mono text-[13px] leading-relaxed whitespace-pre-wrap break-words text-slate-200"
      >
        {buffer || `• ${agentProfile}\n  cwd: ${workdir}\n\nWaiting for terminal output…`}
      </pre>

      <form
        onSubmit={handleSend}
        className="flex items-center gap-2 border-t border-slate-800/80 bg-[#0d1018] px-3 py-2.5"
      >
        <span className="font-mono text-sm text-slate-500" aria-hidden="true">
          &gt;
        </span>
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder={sendPlaceholder}
          aria-label={sendPlaceholder}
          className="min-w-0 flex-1 bg-transparent font-mono text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none"
        />
      </form>
    </div>
  );
}

export default TerminalPane;
