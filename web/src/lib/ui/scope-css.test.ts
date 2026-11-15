import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';
import { scopeCss, splitSelectors } from './scope-css';

const S = '.scope';

describe('scopeCss', () => {
  it('prefixes every selector in a list', () => {
    expect(scopeCss('.a{color:red}.b > c, .d{margin:0}', S)).toBe('.scope .a{color:red}.scope .b > c,.scope .d{margin:0}');
  });

  it('does not split commas inside :is() or attribute values', () => {
    expect(scopeCss(':is(.a, .b) .c, [x="1,2"]{top:0}', S)).toBe('.scope :is(.a, .b) .c,.scope [x="1,2"]{top:0}');
  });

  it('keeps :root and the listed global selectors untouched', () => {
    const out = scopeCss(':root{--x:1}#a,.keep{display:none}', S, ['.keep']);
    expect(out).toBe(':root{--x:1}.scope #a,.keep{display:none}');
  });

  it('turns html and body into the scope element', () => {
    expect(scopeCss('html{font:x}body .a{top:0}', S)).toBe('.scope{font:x}.scope .a{top:0}');
  });

  it('recurses into @media and leaves its prelude alone', () => {
    expect(scopeCss('@media (prefers-color-scheme: dark){:root{--y:2}.a{color:red}}', S)).toBe(
      '@media (prefers-color-scheme: dark){:root{--y:2}.scope .a{color:red}}',
    );
  });

  it('copies @keyframes and @font-face verbatim', () => {
    const css = '@keyframes spin{from{transform:none}to{transform:rotate(1turn)}}@font-face{font-family:x}';
    expect(scopeCss(css, S)).toBe(css);
  });

  it('survives braces, semicolons and quotes inside url() and strings', () => {
    const css = `.a{background:url("data:image/svg+xml,<svg><style>a{fill:red}</style></svg>");--i:url(data:x{y});content:"}"}.b{top:0}`;
    expect(scopeCss(css, S)).toBe(`.scope ${css.replace('.b{', '.scope .b{')}`);
  });

  it('strips comments', () => {
    expect(scopeCss('/* .x{} */.a{top:0}', S)).toBe('.scope .a{top:0}');
  });
});

describe('splitSelectors', () => {
  it('splits only top-level commas', () => {
    expect(splitSelectors('.a, :is(.b,.c) d,[e=","]')).toEqual(['.a', ':is(.b,.c) d', '[e=","]']);
  });
});

describe('pdf.js viewer stylesheet', () => {
  const require = createRequire(import.meta.url);
  const css = readFileSync(require.resolve('pdfjs-dist/web/pdf_viewer.css'), 'utf8');
  const scoped = scopeCss(css, S, ['.hiddenCanvasElement']);

  it('leaves no selector outside the scope apart from :root and the keep list', () => {
    const selectors = [...scoped.matchAll(/(?:^|[{};])\s*([^{};@][^{};]*)\{/g)].flatMap((m) => splitSelectors(m[1]));
    expect(selectors.length).toBeGreaterThan(300);
    const loose = selectors.filter((s) => s !== ':root' && s !== '.hiddenCanvasElement' && !s.startsWith(`${S} `));
    expect(loose).toEqual([]);
  });

  it('keeps braces balanced', () => {
    const count = (ch: string) => scoped.replace(/"[^"]*"|'[^']*'/g, '').split(ch).length;
    expect(count('{')).toBe(count('}'));
  });
});
