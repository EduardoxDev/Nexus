const LOCALE = "pt-BR";

const dateTime = new Intl.DateTimeFormat(LOCALE, {
  dateStyle: "short",
  timeStyle: "medium",
});
const relative = new Intl.RelativeTimeFormat(LOCALE, { numeric: "auto" });
const integer = new Intl.NumberFormat(LOCALE);
const decimal = new Intl.NumberFormat(LOCALE, {
  maximumFractionDigits: 1,
  minimumFractionDigits: 1,
});

export function formatDateTime(ms: number): string {
  return dateTime.format(new Date(ms));
}

/** "há 5 minutos", "ontem". Usa o instante `now` para ser testável. */
export function formatRelative(ms: number, now: number = Date.now()): string {
  const diff = ms - now;
  const abs = Math.abs(diff);
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (abs < minute) return relative.format(Math.round(diff / 1000), "second");
  if (abs < hour) return relative.format(Math.round(diff / minute), "minute");
  if (abs < day) return relative.format(Math.round(diff / hour), "hour");
  return relative.format(Math.round(diff / day), "day");
}

export function formatInteger(n: number): string {
  return integer.format(n);
}

export function formatBytes(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB"] as const;
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${integer.format(bytes)} B` : `${decimal.format(value)} ${units[unit]}`;
}

export function formatDuration(ms: number): string {
  if (ms < 1000) return `${integer.format(ms)} ms`;
  if (ms < 60_000) return `${decimal.format(ms / 1000)} s`;
  const minutes = Math.floor(ms / 60_000);
  const seconds = Math.round((ms % 60_000) / 1000);
  return `${minutes} min ${seconds} s`;
}

export function shortHash(hash: string, length = 12): string {
  return hash.slice(0, length);
}
