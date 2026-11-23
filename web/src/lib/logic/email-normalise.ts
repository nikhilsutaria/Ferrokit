// Normalise email addresses the way the big providers treat them, to find duplicates.

export interface EmailOptions {
  /** Lowercase the part before the @. Almost every provider ignores its case. */
  lowercaseLocal: boolean;
  /** Remove +tags for every domain, not just providers known to support them. */
  stripAllTags: boolean;
}

export interface EmailRow {
  input: string;
  /** The normalised address, or the trimmed input when it doesn't look like an address. */
  output: string;
  valid: boolean;
  reason?: string;
}

export interface EmailReport {
  rows: EmailRow[];
  /** Normalised addresses that more than one entry collapses to. */
  duplicates: { address: string; inputs: string[] }[];
  invalid: EmailRow[];
  /** Valid normalised addresses, each once, in first-seen order. */
  unique: string[];
}

type Rule = { dots?: boolean; plus?: boolean; dash?: boolean; domain?: string };

const MICROSOFT = /^(?:(?:outlook|hotmail|live)\.(?:com|[a-z]{2}|co\.[a-z]{2}|com\.[a-z]{2})|msn\.com)$/;
const YAHOO = /^(?:yahoo\.(?:com|[a-z]{2}|co\.[a-z]{2}|com\.[a-z]{2})|ymail\.com|rocketmail\.com)$/;
const PLUS_ONLY = new Set([
  'icloud.com',
  'me.com',
  'mac.com',
  'fastmail.com',
  'fastmail.fm',
  'protonmail.com',
  'protonmail.ch',
  'proton.me',
  'pm.me',
]);

function ruleFor(domain: string): Rule | null {
  if (domain === 'gmail.com' || domain === 'googlemail.com') return { dots: true, plus: true, domain: 'gmail.com' };
  if (MICROSOFT.test(domain) || PLUS_ONLY.has(domain)) return { plus: true };
  if (YAHOO.test(domain)) return { dash: true };
  return null;
}

/** Split on new lines, commas and semicolons, but not inside "quoted names" or <angle brackets>. */
export function splitEntries(text: string): string[] {
  const out: string[] = [];
  let cur = '';
  let quoted = false;
  let angle = false;
  for (const ch of text) {
    if (ch === '"' && !angle) quoted = !quoted;
    else if (ch === '<' && !quoted) angle = true;
    else if (ch === '>' && !quoted) angle = false;
    if ((ch === '\n' || ch === '\r') || (!quoted && !angle && (ch === ',' || ch === ';'))) {
      out.push(cur);
      cur = '';
      quoted = angle = false;
    } else cur += ch;
  }
  out.push(cur);
  return out.map((s) => s.trim()).filter(Boolean);
}

/** The bare address from `Name <addr>`, `<addr>` or `mailto:addr`. */
export function extractAddress(entry: string): string {
  let s = entry.trim();
  const m = /<([^<>]*)>\s*$/.exec(s);
  if (m) s = m[1].trim();
  return s.replace(/^mailto:/i, '').trim();
}

const LOCAL_CHAR = /^[\p{L}\p{N}!#$%&'*+/=?^_`{|}~.-]+$/u;
const LABEL = /^[\p{L}\p{N}](?:[\p{L}\p{N}-]*[\p{L}\p{N}])?$/u;

/** Null if the address looks plausible, otherwise what looks wrong. */
export function checkAddress(addr: string): string | null {
  if (/\s/.test(addr)) return 'Contains a space.';
  const at = addr.split('@').length - 1;
  if (at === 0) return 'Has no @ sign.';
  if (at > 1) return 'Has more than one @ sign.';
  const [local, domain] = addr.split('@');
  if (!local) return 'Has nothing before the @.';
  if (!domain) return 'Has nothing after the @.';
  if (local.includes('..') || domain.includes('..')) return 'Has two dots in a row.';
  if (/^\.|\.$/.test(local)) return 'The part before the @ starts or ends with a dot.';
  if (local.length > 64) return 'The part before the @ is longer than 64 characters.';
  if (!LOCAL_CHAR.test(local)) return 'The part before the @ has unusual characters.';
  const labels = domain.replace(/\.$/, '').split('.');
  if (labels.length < 2) return 'The domain has no dot, as in example.com.';
  if (!labels.every((l) => LABEL.test(l))) return 'The domain has unusual characters, or a part that starts or ends with a hyphen.';
  const tld = labels[labels.length - 1];
  if (!/^(?:\p{L}{2,}|xn--[a-z0-9-]+)$/iu.test(tld)) return 'The domain doesn’t end in a real-looking ending such as .com or .nz.';
  return null;
}

/** Normalise one bare address. Assumes it passed checkAddress. */
export function normaliseEmail(addr: string, opts: EmailOptions): string {
  const s = addr.trim();
  const at = s.lastIndexOf('@');
  let local = s.slice(0, at);
  let domain = s.slice(at + 1).toLowerCase().replace(/\.$/, '');
  const rule = ruleFor(domain);
  const cut = (sep: string) => {
    const i = local.indexOf(sep);
    if (i > 0) local = local.slice(0, i);
  };
  if (rule?.plus || opts.stripAllTags) cut('+');
  if (rule?.dash) cut('-');
  if (rule?.dots) local = local.replace(/\./g, '');
  if (rule?.domain) domain = rule.domain;
  if (opts.lowercaseLocal) local = local.toLowerCase();
  return `${local}@${domain}`;
}

export function analyseEmails(text: string, opts: EmailOptions): EmailReport {
  const rows: EmailRow[] = splitEntries(text).map((input) => {
    const addr = extractAddress(input);
    const reason = checkAddress(addr);
    return reason ? { input, output: input, valid: false, reason } : { input, output: normaliseEmail(addr, opts), valid: true };
  });

  const groups = new Map<string, string[]>();
  for (const r of rows) {
    if (!r.valid) continue;
    const list = groups.get(r.output);
    if (list) list.push(r.input);
    else groups.set(r.output, [r.input]);
  }

  return {
    rows,
    duplicates: [...groups].filter(([, inputs]) => inputs.length > 1).map(([address, inputs]) => ({ address, inputs })),
    invalid: rows.filter((r) => !r.valid),
    unique: [...groups.keys()],
  };
}
