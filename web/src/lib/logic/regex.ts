// Run a regular expression safely: no zero-length infinite loops, capped match count.

export interface RegexMatch {
  index: number;
  end: number;
  text: string;
  /** Numbered groups, 1-based in the UI; undefined when a group didn't take part. */
  groups: (string | undefined)[];
  named: Record<string, string | undefined>;
}

export interface RegexResult {
  matches: RegexMatch[];
  truncated: boolean;
}

export const MAX_MATCHES = 1000;

export function compile(pattern: string, flags: string): RegExp {
  try {
    return new RegExp(pattern, flags);
  } catch (e) {
    throw new Error((e as Error).message.replace(/^Invalid regular expression: /, ''));
  }
}

export function runRegex(re: RegExp, text: string, cap = MAX_MATCHES): RegexResult {
  const matches: RegexMatch[] = [];
  const unicode = re.unicode || re.flags.includes('v');
  const toMatch = (m: RegExpExecArray): RegexMatch => ({
    index: m.index,
    end: m.index + m[0].length,
    text: m[0],
    groups: m.slice(1),
    named: m.groups ? { ...m.groups } : {},
  });

  re.lastIndex = 0;
  if (!re.global) {
    const m = re.exec(text);
    if (m) matches.push(toMatch(m));
    re.lastIndex = 0;
    return { matches, truncated: false };
  }

  let truncated = false;
  while (true) {
    const m = re.exec(text);
    if (!m) break;
    if (matches.length >= cap) {
      truncated = true;
      break;
    }
    matches.push(toMatch(m));
    if (m[0] === '') {
      // Step past an empty match, by a whole code point in unicode mode.
      const i = re.lastIndex;
      const cp = unicode ? text.codePointAt(i) : undefined;
      re.lastIndex = i + (cp !== undefined && cp > 0xffff ? 2 : 1);
      if (re.lastIndex > text.length) break;
    }
  }
  re.lastIndex = 0;
  return { matches, truncated };
}

export interface Segment {
  text: string;
  /** Index into the matches array, or -1 for unmatched text. */
  match: number;
}

/** Split text into alternating plain and matched segments for highlighting. Empty matches are skipped. */
export function segments(text: string, matches: RegexMatch[]): Segment[] {
  const out: Segment[] = [];
  let pos = 0;
  matches.forEach((m, k) => {
    if (m.end === m.index || m.index < pos) return;
    if (m.index > pos) out.push({ text: text.slice(pos, m.index), match: -1 });
    out.push({ text: m.text, match: k });
    pos = m.end;
  });
  if (pos < text.length) out.push({ text: text.slice(pos), match: -1 });
  return out;
}
