// Formatting helpers for the device information page.

export const NOT_SHARED = 'Not shared by this browser';

/** Date.getTimezoneOffset() minutes (positive west of UTC) as "UTC+13:00". */
export function utcOffset(offsetMinutes: number): string {
  const east = -offsetMinutes;
  const sign = east < 0 ? '-' : '+';
  const abs = Math.abs(east);
  const hh = String(Math.floor(abs / 60)).padStart(2, '0');
  const mm = String(abs % 60).padStart(2, '0');
  return `UTC${sign}${hh}:${mm}`;
}

export function formatSize(width: number, height: number): string {
  return `${width} × ${height}`;
}

/** navigator.deviceMemory is rounded by the browser, so say so. */
export function formatMemory(gb: number): string {
  return gb < 1 ? `About ${Math.round(gb * 1024)} MB` : `About ${gb} GB`;
}

/** The value as text, or NOT_SHARED when the browser gave nothing. */
export function orNotShared<T>(value: T | null | undefined, format?: (v: T) => string): string {
  if (value === undefined || value === null || value === '') return NOT_SHARED;
  return format ? format(value) : String(value);
}

export interface InfoGroup {
  title: string;
  rows: [label: string, value: string][];
}

export function toPlainText(groups: InfoGroup[]): string {
  return groups.map((g) => [g.title, ...g.rows.map(([k, v]) => `${k}: ${v}`)].join('\n') + '\n').join('\n');
}
