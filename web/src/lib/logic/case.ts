// Split identifiers and phrases into words, then join them in different cases.

/**
 * Split text into words. Handles camelCase, PascalCase, acronyms (XMLHttpRequest → XML Http Request),
 * snake_case, kebab-case, dots, slashes and spaces. Digits stay attached to the word before them
 * (base64Encode → base64 Encode).
 */
export function splitWords(text: string): string[] {
  return text
    .replace(/([\p{Ll}\p{N}])(\p{Lu})/gu, '$1 $2')
    .replace(/(\p{Lu})(\p{Lu}\p{Ll})/gu, '$1 $2')
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean);
}

const cap = (w: string) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase();
const lower = (ws: string[]) => ws.map((w) => w.toLowerCase());

export type CaseId =
  | 'camel'
  | 'pascal'
  | 'snake'
  | 'constant'
  | 'kebab'
  | 'title'
  | 'sentence'
  | 'lower'
  | 'upper'
  | 'dot'
  | 'path';

export const CASES: { id: CaseId; label: string }[] = [
  { id: 'camel', label: 'camelCase' },
  { id: 'pascal', label: 'PascalCase' },
  { id: 'snake', label: 'snake_case' },
  { id: 'constant', label: 'CONSTANT_CASE' },
  { id: 'kebab', label: 'kebab-case' },
  { id: 'title', label: 'Title Case' },
  { id: 'sentence', label: 'Sentence case' },
  { id: 'lower', label: 'lower case' },
  { id: 'upper', label: 'UPPER CASE' },
  { id: 'dot', label: 'dot.case' },
  { id: 'path', label: 'path/case' },
];

function convertLine(line: string, to: CaseId): string {
  // lower and UPPER keep the text as written, punctuation and all.
  if (to === 'lower') return line.toLowerCase();
  if (to === 'upper') return line.toUpperCase();
  const w = splitWords(line);
  switch (to) {
    case 'camel':
      return w.map((x, i) => (i === 0 ? x.toLowerCase() : cap(x))).join('');
    case 'pascal':
      return w.map(cap).join('');
    case 'snake':
      return lower(w).join('_');
    case 'constant':
      return w.map((x) => x.toUpperCase()).join('_');
    case 'kebab':
      return lower(w).join('-');
    case 'title':
      return w.map(cap).join(' ');
    case 'sentence':
      return w.map((x, i) => (i === 0 ? cap(x) : x.toLowerCase())).join(' ');
    case 'dot':
      return lower(w).join('.');
    case 'path':
      return lower(w).join('/');
  }
}

/** Convert each line of `text` separately, so a list of names converts in one go. */
export function convertCase(text: string, to: CaseId): string {
  return text
    .split(/\r?\n/)
    .map((line) => convertLine(line, to))
    .join('\n');
}
