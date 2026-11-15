import { describe, expect, it } from 'vitest';
import { search } from './search';
import type { Tool } from './registry';

const t = (id: string, name: string, keywords = '', ready = true): Tool => ({
  id,
  name,
  category: 'x',
  description: '',
  keywords: keywords.split(' ').filter(Boolean),
  runtime: 'browser',
  ready,
});

const tools = [
  t('merge-pdf', 'Merge PDFs', 'combine join'),
  t('split-pdf', 'Split PDF', 'separate'),
  t('hash-text', 'Hash text', 'sha256 md5'),
  t('json-yaml', 'JSON and YAML', 'yml'),
  t('json-formatter', 'JSON formatter', 'prettify'),
  t('json-xml', 'JSON and XML', 'xml', false),
];

describe('search', () => {
  it('finds by name prefix and keyword', () => {
    expect(search(tools, 'mer')[0].id).toBe('merge-pdf');
    expect(search(tools, 'combine')[0].id).toBe('merge-pdf');
    expect(search(tools, 'sha256')[0].id).toBe('hash-text');
  });

  it('requires every term to match', () => {
    expect(search(tools, 'json yaml').map((x) => x.id)).toEqual(['json-yaml']);
    expect(search(tools, 'pdf zebra')).toEqual([]);
  });

  it('ranks ready tools above planned ones', () => {
    const ids = search(tools, 'json').map((x) => x.id);
    expect(ids.at(-1)).toBe('json-xml');
  });

  it('ignores case, accents and punctuation', () => {
    expect(search(tools, '  HASH!! ')[0].id).toBe('hash-text');
    expect(search(tools, '')).toEqual([]);
  });
});
