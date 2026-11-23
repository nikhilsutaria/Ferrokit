// Build <head> tags for search results and link previews (Open Graph and Twitter / X cards).

export interface MetaFields {
  title: string;
  description: string;
  url: string;
  image: string;
  imageAlt: string;
  siteName: string;
  type: 'website' | 'article';
  locale: string;
  card: 'summary' | 'summary_large_image';
  twitter: string;
}

export function emptyMeta(): MetaFields {
  return {
    title: '',
    description: '',
    url: '',
    image: '',
    imageAlt: '',
    siteName: '',
    type: 'website',
    locale: '',
    card: 'summary_large_image',
    twitter: '',
  };
}

export const TITLE_MAX = 60;
export const DESCRIPTION_MAX = 160;

export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function handle(s: string): string {
  const bare = s.trim().replace(/^@+/, '');
  return bare ? `@${bare}` : '';
}

/** The tags, one per line, in three groups separated by blank lines. Empty fields are left out. */
export function buildMetaTags(input: MetaFields): string {
  const f = {
    title: input.title.trim(),
    description: input.description.trim(),
    url: input.url.trim(),
    image: input.image.trim(),
    imageAlt: input.image.trim() ? input.imageAlt.trim() : '',
    siteName: input.siteName.trim(),
    locale: input.locale.trim(),
    twitter: handle(input.twitter),
  };
  if (!Object.values(f).some(Boolean)) return '';

  const meta = (attr: 'name' | 'property', key: string, value: string) =>
    value ? [`<meta ${attr}="${key}" content="${escapeHtml(value)}">`] : [];

  const basic = [
    ...(f.title ? [`<title>${escapeHtml(f.title)}</title>`] : []),
    ...meta('name', 'description', f.description),
    ...(f.url ? [`<link rel="canonical" href="${escapeHtml(f.url)}">`] : []),
  ];
  const og = [
    ...meta('property', 'og:type', input.type),
    ...meta('property', 'og:title', f.title),
    ...meta('property', 'og:description', f.description),
    ...meta('property', 'og:url', f.url),
    ...meta('property', 'og:image', f.image),
    ...meta('property', 'og:image:alt', f.imageAlt),
    ...meta('property', 'og:site_name', f.siteName),
    ...meta('property', 'og:locale', f.locale),
  ];
  const twitter = [
    ...meta('name', 'twitter:card', input.card),
    ...meta('name', 'twitter:site', f.twitter),
    ...meta('name', 'twitter:title', f.title),
    ...meta('name', 'twitter:description', f.description),
    ...meta('name', 'twitter:image', f.image),
    ...meta('name', 'twitter:image:alt', f.imageAlt),
  ];
  return [basic, og, twitter]
    .filter((g) => g.length)
    .map((g) => g.join('\n'))
    .join('\n\n');
}

export type MetaHints = Partial<Record<keyof MetaFields, string>>;

function isFullUrl(s: string): boolean {
  return /^https?:\/\/[^\s/]+\.[^\s/]+/i.test(s);
}

/** Gentle advice per field: long text, relative URLs, odd handles. */
export function metaHints(f: MetaFields): MetaHints {
  const hints: MetaHints = {};
  const title = f.title.trim().length;
  const desc = f.description.trim().length;
  if (title > TITLE_MAX) {
    hints.title = `${title} characters. Search results usually cut titles off after about ${TITLE_MAX}.`;
  }
  if (desc > DESCRIPTION_MAX) {
    hints.description = `${desc} characters. Search results usually cut descriptions off after about ${DESCRIPTION_MAX}.`;
  }
  for (const key of ['url', 'image'] as const) {
    const v = f[key].trim();
    if (v && !isFullUrl(v)) hints[key] = 'Use a full address starting with https://, as link previews ignore relative ones.';
  }
  const h = f.twitter.trim().replace(/^@+/, '');
  if (h && !/^\w{1,15}$/.test(h)) hints.twitter = 'Handles are up to 15 letters, numbers and underscores.';
  return hints;
}
