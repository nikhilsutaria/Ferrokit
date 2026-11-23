// Unwrap Microsoft Safe Links and Google redirect links back to the address they point at.

export interface DecodedLink {
  from: string;
  to: string;
}

// Outlook (worldwide and US government) and the Teams variant of Safe Links.
const SAFELINK_HOST = /^[a-z0-9-]+\.safelinks\.protection\.(outlook\.com|office365\.us)$/i;
const TEAMS_HOST = 'statics.teams.cdn.office.net';
const GOOGLE_HOST = /^(www\.)?google\.[a-z]{2,3}(\.[a-z]{2})?$/i;

/** The target of one redirect hop, or null if this isn't a redirect link. */
function unwrapOnce(link: string): string | null {
  let u: URL;
  try {
    u = new URL(link.replace(/&amp;/g, '&'));
  } catch {
    return null;
  }
  if (u.protocol !== 'https:' && u.protocol !== 'http:') return null;
  const host = u.hostname;
  let target: string | null = null;
  if (SAFELINK_HOST.test(host) || (host === TEAMS_HOST && u.pathname.includes('safelinks'))) {
    target = u.searchParams.get('url');
  } else if (GOOGLE_HOST.test(host) && u.pathname === '/url') {
    target = u.searchParams.get('q') ?? u.searchParams.get('url');
  }
  if (!target || !/^[a-z][a-z0-9+.-]*:/i.test(target.trim())) return null;
  return target.trim();
}

/** Follow nested redirect links to the end. Null if the link isn't a redirect at all. */
export function decodeLink(link: string): string | null {
  let current = link.trim();
  let decoded: string | null = null;
  for (let i = 0; i < 10; i++) {
    const next = unwrapOnce(current);
    if (!next) break;
    decoded = current = next;
  }
  return decoded;
}

// A run of URL characters; trailing punctuation is trimmed off afterwards.
const URL_IN_TEXT = /https?:\/\/[^\s<>"'`]+/gi;

/** Replace every safelink or Google redirect in a block of text with its real address. */
export function decodeText(text: string): { text: string; decoded: DecodedLink[] } {
  const decoded: DecodedLink[] = [];
  const out = text.replace(URL_IN_TEXT, (raw) => {
    const trail = /[.,;:!?)\]}]+$/.exec(raw)?.[0] ?? '';
    const link = trail ? raw.slice(0, -trail.length) : raw;
    const to = decodeLink(link);
    if (!to) return raw;
    decoded.push({ from: link, to });
    return to + trail;
  });
  return { text: out, decoded };
}
