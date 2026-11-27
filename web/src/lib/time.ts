export function formatRelativeShort(epochMs: number, nowMs: number = Date.now()): string {
  const diffSec = Math.max(0, Math.floor((nowMs - epochMs) / 1000));
  if (diffSec < 60) {
    return "now";
  }
  const mins = Math.floor(diffSec / 60);
  if (mins < 60) {
    return `${mins}m`;
  }
  const hours = Math.floor(mins / 60);
  if (hours < 24) {
    return `${hours}h`;
  }
  const days = Math.floor(hours / 24);
  const remHours = hours % 24;
  return remHours > 0 ? `${days}d ${remHours}h` : `${days}d`;
}
