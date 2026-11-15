// Small helpers for the multi-image tools (collage, GIF maker, sprite sheet).
// Each mirrors what the server does, so the screen can describe the result before it is made.

/** Columns and rows for `n` photos; `columns` 0 picks a near-square grid. Same rule as the server. */
export function collageGrid(n: number, columns: number): { cols: number; rows: number } {
  const count = Math.max(0, Math.floor(n));
  const wanted = Math.max(0, Math.floor(columns) || 0);
  const cols = Math.max(1, wanted > 0 ? Math.min(wanted, Math.max(count, 1)) : Math.ceil(Math.sqrt(count)));
  return { cols, rows: Math.ceil(count / cols) };
}

/** "16:9" → 16/9; null for an empty or unreadable ratio. */
export function parseRatio(s: string): number | null {
  const parts = s.split(/[:x/×]/).map((p) => p.trim());
  if (parts.length < 1 || parts.length > 2 || parts.some((p) => p === '')) return null;
  const nums = parts.map(Number);
  const r = nums.length === 2 ? nums[0] / nums[1] : nums[0];
  return Number.isFinite(r) && r > 0 ? r : null;
}

/** GIF frame delays are stored in hundredths of a second, between 20 ms and 60 s. */
export function clampDelay(ms: number): number {
  if (!Number.isFinite(ms)) return 500;
  return Math.min(60_000, Math.max(20, Math.round(ms / 10) * 10));
}

/** Frames per second → delay per frame in ms (as the GIF will store it). */
export function fpsToDelay(fps: number): number {
  if (!Number.isFinite(fps) || fps <= 0) return 500;
  return clampDelay(1000 / fps);
}

/** Delay per frame → frames per second, rounded to one decimal. */
export function delayToFps(ms: number): number {
  return Math.round((1000 / clampDelay(ms)) * 10) / 10;
}

/** A CSS-safe class name, as the server makes one from a file name or prefix. */
export function className(s: string): string {
  let out = '';
  for (const c of s.toLowerCase()) {
    if (/^[a-z0-9]$/.test(c)) out += c;
    else if (!out.endsWith('-')) out += '-';
  }
  const t = out.replace(/^-+|-+$/g, '');
  return t || 'icon';
}
