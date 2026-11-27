import { wsBase } from "../../transport/daemon-client";

export interface PtyStreamHandle {
  sendInput: (data: string) => void;
  sendResize: (cols: number, rows: number) => void;
  requestResync: () => void;
  close: () => void;
}

export function openPtyStream(
  token: string,
  sessionId: string,
  handlers: {
    onData: (chunk: Uint8Array | string) => void;
    onConnectionChange: (live: boolean) => void;
  },
): PtyStreamHandle {
  const ws = new WebSocket(`${wsBase()}/api/v1/sessions/${encodeURIComponent(sessionId)}/pty`);
  ws.binaryType = "arraybuffer";

  ws.addEventListener("open", () => {
    ws.send(JSON.stringify({ type: "auth", token }));
    handlers.onConnectionChange(true);
  });

  ws.addEventListener("message", (ev) => {
    if (ev.data instanceof ArrayBuffer) {
      handlers.onData(new Uint8Array(ev.data));
      return;
    }
    if (typeof ev.data === "string") {
      try {
        const parsed = JSON.parse(ev.data) as { type?: string; bytes?: number };
        if (parsed.type === "dropped" && typeof parsed.bytes === "number") {
          handlers.onData(`\r\n[...${parsed.bytes} bytes dropped...]\r\n`);
        }
      } catch {
        handlers.onData(ev.data);
      }
    }
  });

  ws.addEventListener("close", () => handlers.onConnectionChange(false));
  ws.addEventListener("error", () => handlers.onConnectionChange(false));

  return {
    sendInput: (data) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "input", data }));
      }
    },
    sendResize: (cols, rows) => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "resize", cols, rows }));
      }
    },
    requestResync: () => {
      if (ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: "resync" }));
      }
    },
    close: () => ws.close(),
  };
}
