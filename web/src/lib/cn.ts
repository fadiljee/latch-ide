export function cn(...parts: Array<string | false | null | undefined>): string {
  const out: string[] = [];
  for (const part of parts) {
    if (part) {
      out.push(part);
    }
  }
  return out.join(" ");
}
