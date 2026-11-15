// A typed web address → something Chromium may load. Mirrors `chrome::check_url` in ferrokit-worker.

export function normaliseUrl(input: string): string | null {
  const s = input.trim();
  if (!s) return null;
  const lower = s.toLowerCase();
  if (lower.startsWith('http://') || lower.startsWith('https://')) return s;
  if (s.includes('://') || /^(javascript|data|file|blob|about):/.test(lower)) return null;
  const host = s.split(/[/?#]/)[0];
  if (!host || /\s/.test(host)) return null;
  if (!(host.includes('.') || host.includes(':') || host === 'localhost')) return null;
  return `https://${s}`;
}
