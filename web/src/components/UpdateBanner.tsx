import { useEffect, useState } from "react";
import { CATALOGS } from "../lib/i18n/catalog";
import { useHostsStore } from "../state/hosts";
import { useUiStore } from "../state/ui";
import { fetchUpdateInfo } from "../transport/daemon-client";
import type { UpdateInfo } from "../transport/protocol";

const DISMISS_KEY = "latch_update_dismissed";
const RECHECK_MS = 60 * 60 * 1000;

function readDismissed(): string {
  try {
    return window.localStorage.getItem(DISMISS_KEY) ?? "";
  } catch {
    return "";
  }
}

export function UpdateBanner() {
  const token = useHostsStore((s) => s.token);
  const connected = useHostsStore((s) => s.connected);
  const locale = useUiStore((s) => s.locale);
  const t = CATALOGS[locale].update;
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [dismissed, setDismissed] = useState<string>(readDismissed);

  useEffect(() => {
    if (!token || !connected) {
      return undefined;
    }
    let cancelled = false;
    const check = () => {
      fetchUpdateInfo(token)
        .then((next) => {
          if (!cancelled) {
            setInfo(next);
          }
        })
        .catch(() => {
          // Pemeriksaan versi bersifat opsional; abaikan bila gagal.
        });
    };
    check();
    const timer = window.setInterval(check, RECHECK_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [token, connected]);

  if (!info?.update_available || !info.latest || dismissed === info.latest) {
    return null;
  }
  const latest = info.latest;
  const releaseUrl = info.release_url;

  const dismiss = () => {
    try {
      window.localStorage.setItem(DISMISS_KEY, latest);
    } catch {
      // Penyimpanan tidak tersedia; sembunyikan untuk sesi ini saja.
    }
    setDismissed(latest);
  };

  return (
    <div
      role="status"
      className="flex shrink-0 flex-wrap items-center gap-x-3 gap-y-1 border-b border-line bg-sidebar px-3 py-1.5 text-[12px] text-fg-2"
    >
      <span className="font-medium text-fg">
        {t.available.replace("{latest}", latest).replace("{current}", `v${info.current}`)}
      </span>
      <span>{t.howTo}</span>
      <code className="rounded-sm border border-line bg-app px-1.5 py-0.5 font-mono text-fg">
        latch update
      </code>
      {releaseUrl ? (
        <a
          href={releaseUrl}
          target="_blank"
          rel="noreferrer noopener"
          className="underline underline-offset-2 hover:text-fg"
        >
          {t.notes}
        </a>
      ) : null}
      <button
        type="button"
        aria-label={t.dismiss}
        onClick={dismiss}
        className="ml-auto rounded-sm px-1.5 hover:bg-app hover:text-fg"
      >
        ×
      </button>
    </div>
  );
}
