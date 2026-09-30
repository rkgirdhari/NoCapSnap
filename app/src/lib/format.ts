const time = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });
const weekday = new Intl.DateTimeFormat(undefined, { weekday: "long" });
const day = new Intl.DateTimeFormat(undefined, { weekday: "long", month: "long", day: "numeric" });

export const formatTime = (iso: string) => time.format(new Date(iso));

export function formatBytes(n: number | null): string {
  if (n == null) return "—";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export const shortDigest = (sha256: string) => sha256.slice(0, 8);

export function isSameLocalDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function dayLabel(iso: string, now = new Date()): string {
  const d = new Date(iso);
  if (isSameLocalDay(d, now)) return "Today";
  const yesterday = new Date(now);
  yesterday.setDate(now.getDate() - 1);
  if (isSameLocalDay(d, yesterday)) return "Yesterday";
  return day.format(d);
}

/** "Wednesday · Dinner service" — the way the pass talks about time. */
export function serviceLine(now = new Date()): string {
  const h = now.getHours();
  const service =
    h < 11 ? "Morning prep" : h < 16 ? "Lunch service" : h < 22 ? "Dinner service" : "Late service";
  return `${weekday.format(now)} · ${service}`;
}
