import { describe, expect, it } from 'vitest';
import { buildMetaTags, emptyMeta, escapeHtml, metaHints, type MetaFields } from './meta-tags';

const full: MetaFields = {
  title: 'Ferrokit',
  description: 'A self-hosted toolbox.',
  url: 'https://example.com/',
  image: 'https://example.com/card.png',
  imageAlt: 'The Ferrokit logo',
  siteName: 'Ferrokit',
  type: 'website',
  locale: 'en_NZ',
  card: 'summary_large_image',
  twitter: 'ferrokit',
};

describe('escapeHtml', () => {
  it('escapes markup and quotes', () => {
    expect(escapeHtml(`<a href="x">Tom & Jerry's</a>`)).toBe('&lt;a href=&quot;x&quot;&gt;Tom &amp; Jerry&#39;s&lt;/a&gt;');
  });
});

describe('buildMetaTags', () => {
  it('writes every tag when everything is filled', () => {
    const html = buildMetaTags(full);
    expect(html.split('\n')).toEqual([
      '<title>Ferrokit</title>',
      '<meta name="description" content="A self-hosted toolbox.">',
      '<link rel="canonical" href="https://example.com/">',
      '',
      '<meta property="og:type" content="website">',
      '<meta property="og:title" content="Ferrokit">',
      '<meta property="og:description" content="A self-hosted toolbox.">',
      '<meta property="og:url" content="https://example.com/">',
      '<meta property="og:image" content="https://example.com/card.png">',
      '<meta property="og:image:alt" content="The Ferrokit logo">',
      '<meta property="og:site_name" content="Ferrokit">',
      '<meta property="og:locale" content="en_NZ">',
      '',
      '<meta name="twitter:card" content="summary_large_image">',
      '<meta name="twitter:site" content="@ferrokit">',
      '<meta name="twitter:title" content="Ferrokit">',
      '<meta name="twitter:description" content="A self-hosted toolbox.">',
      '<meta name="twitter:image" content="https://example.com/card.png">',
      '<meta name="twitter:image:alt" content="The Ferrokit logo">',
    ]);
  });

  it('leaves out empty fields', () => {
    const html = buildMetaTags({ ...emptyMeta(), title: 'Only a title' });
    expect(html).toContain('<title>Only a title</title>');
    expect(html).toContain('og:title');
    expect(html).not.toContain('description');
    expect(html).not.toContain('canonical');
    expect(html).not.toContain('og:image');
    expect(html).not.toContain('twitter:site');
    expect(html).not.toContain('og:locale');
  });

  it('writes nothing when nothing is filled', () => {
    expect(buildMetaTags(emptyMeta())).toBe('');
    expect(buildMetaTags({ ...emptyMeta(), title: '   ' })).toBe('');
  });

  it('escapes values and trims spaces', () => {
    const html = buildMetaTags({ ...emptyMeta(), title: '  "Fish" & <chips>  ' });
    expect(html).toContain('<title>&quot;Fish&quot; &amp; &lt;chips&gt;</title>');
    expect(html).toContain('content="&quot;Fish&quot; &amp; &lt;chips&gt;"');
  });

  it('keeps a single @ on the handle', () => {
    expect(buildMetaTags({ ...emptyMeta(), twitter: '@@ferrokit' })).toContain('content="@ferrokit"');
  });

  it('skips the image alt text when there is no image', () => {
    expect(buildMetaTags({ ...emptyMeta(), title: 'x', imageAlt: 'alt' })).not.toContain('image:alt');
  });
});

describe('metaHints', () => {
  it('warns about long titles and descriptions', () => {
    const hints = metaHints({ ...emptyMeta(), title: 'x'.repeat(61), description: 'y'.repeat(161) });
    expect(hints.title).toMatch(/61 characters/);
    expect(hints.description).toMatch(/161 characters/);
    expect(metaHints(full)).toEqual({});
  });

  it('asks for full URLs', () => {
    const hints = metaHints({ ...emptyMeta(), url: 'example.com', image: '/card.png' });
    expect(hints.url).toMatch(/https:\/\//);
    expect(hints.image).toMatch(/https:\/\//);
  });

  it('flags a handle with spaces', () => {
    expect(metaHints({ ...emptyMeta(), twitter: 'my name' }).twitter).toMatch(/letters/);
  });
});
