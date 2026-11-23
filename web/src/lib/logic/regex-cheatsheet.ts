// JavaScript regular expression syntax, with a working example for each entry.

export interface RegexEntry {
  /** The syntax as people write it, such as `\d` or `(?<name>…)`. */
  syntax: string;
  description: string;
  /** Example pattern source (as for `new RegExp`), and flags. */
  pattern: string;
  flags?: string;
  /** Text the example pattern matches. */
  sample: string;
  /** Extra detail or a caveat. */
  note?: string;
}

export interface RegexSection {
  id: string;
  name: string;
  summary: string;
  entries: RegexEntry[];
}

export const regexSections: RegexSection[] = [
  {
    id: 'characters',
    name: 'Characters and escapes',
    summary: 'Most characters match themselves. These ones mean something else.',
    entries: [
      { syntax: '.', description: 'Any single character except a line break.', pattern: 'c.t', sample: 'cut', note: 'With the s flag it matches line breaks too.' },
      {
        syntax: '\\',
        description: 'Escape a special character to match it literally. The special ones are . * + ? ^ $ { } ( ) | [ ] \\ /',
        pattern: '3\\.14',
        sample: 'pi is about 3.14',
      },
      { syntax: '\\n  \\r  \\t', description: 'Line feed, carriage return and tab.', pattern: 'one\\ntwo', sample: 'one\ntwo' },
      { syntax: '\\xhh', description: 'The character with this two-digit hex code.', pattern: '\\x41', sample: 'ABC' },
      { syntax: '\\uhhhh', description: 'The character with this four-digit hex code.', pattern: 'caf\\u00e9', sample: 'café' },
      {
        syntax: '\\u{h…}',
        description: 'Any Unicode code point by its hex code. Needs the u or v flag.',
        pattern: '\\u{20AC}',
        flags: 'u',
        sample: 'Price: €20',
      },
    ],
  },
  {
    id: 'classes',
    name: 'Character classes',
    summary: 'Match one character out of a set.',
    entries: [
      { syntax: '[abc]', description: 'Any one of the characters listed.', pattern: 'gr[ae]y', sample: 'grey' },
      { syntax: '[^abc]', description: 'Any one character that isn’t listed.', pattern: 'q[^u]', sample: 'Iraqi' },
      { syntax: '[a-z]', description: 'Any character in a range. Ranges can be combined, as in [a-f0-9].', pattern: '[a-f0-9]+', sample: 'deadbeef' },
      { syntax: '\\d', description: 'A digit, 0 to 9.', pattern: '\\d{3}', sample: 'Room 404' },
      { syntax: '\\D', description: 'Any character that isn’t a digit.', pattern: '\\D+', sample: '42 apples' },
      {
        syntax: '\\w',
        description: 'A word character: A–Z, a–z, 0–9 or underscore.',
        pattern: '\\w+',
        sample: 'hello_world',
        note: 'ASCII only, so it doesn’t match é or ā. Use \\p{L} with the u flag for letters in any language.',
      },
      { syntax: '\\W', description: 'Any character that isn’t a word character.', pattern: '\\W', sample: 'a-b' },
      { syntax: '\\s', description: 'Whitespace: spaces, tabs, line breaks and Unicode spaces.', pattern: 'a\\sb', sample: 'a b' },
      { syntax: '\\S', description: 'Any character that isn’t whitespace.', pattern: '\\S+', sample: '  word  ' },
      {
        syntax: '[A--B]',
        description: 'Set subtraction: characters in A but not in B. Needs the v flag.',
        pattern: '[\\w--\\d]+',
        flags: 'v',
        sample: 'abc123',
      },
      {
        syntax: '[A&&B]',
        description: 'Set intersection: characters in both A and B. Needs the v flag.',
        pattern: '[\\p{L}&&\\p{ASCII}]+',
        flags: 'v',
        sample: 'café',
      },
    ],
  },
  {
    id: 'anchors',
    name: 'Anchors and boundaries',
    summary: 'Match a position rather than a character.',
    entries: [
      { syntax: '^', description: 'The start of the text, or of each line with the m flag.', pattern: '^Hello', sample: 'Hello there' },
      { syntax: '$', description: 'The end of the text, or of each line with the m flag.', pattern: 'end$', sample: 'the end' },
      {
        syntax: '\\b',
        description: 'A word boundary: between a word character and a non-word character.',
        pattern: '\\bcat\\b',
        sample: 'the cat sat',
        note: 'Uses the ASCII idea of a word, so accented letters count as boundaries.',
      },
      { syntax: '\\B', description: 'Any position that isn’t a word boundary.', pattern: '\\Bcat', sample: 'concatenate' },
    ],
  },
  {
    id: 'quantifiers',
    name: 'Quantifiers',
    summary: 'How many times the thing before them repeats. They are greedy unless followed by ?.',
    entries: [
      { syntax: '*', description: 'Zero or more times.', pattern: 'ab*c', sample: 'ac' },
      { syntax: '+', description: 'One or more times.', pattern: 'ab+c', sample: 'abbbc' },
      { syntax: '?', description: 'Zero or one time, which makes it optional.', pattern: 'colou?r', sample: 'color' },
      { syntax: '{n}', description: 'Exactly n times.', pattern: '\\d{4}', sample: 'Year 2026' },
      { syntax: '{n,}', description: 'n or more times.', pattern: '\\d{2,}', sample: '7 42 1234' },
      { syntax: '{n,m}', description: 'Between n and m times.', pattern: 'a{2,3}', sample: 'caaandy' },
      {
        syntax: 'Greedy',
        description: 'Quantifiers take as much as they can, then give back only if the rest fails.',
        pattern: '<.+>',
        sample: '<b>bold</b>',
      },
      {
        syntax: '*?  +?  ??  {n,m}?',
        description: 'Lazy: take as little as possible. Here it stops at the first >.',
        pattern: '<.+?>',
        sample: '<b>bold</b>',
      },
    ],
  },
  {
    id: 'groups',
    name: 'Groups and backreferences',
    summary: 'Treat several characters as one unit, and capture what they matched.',
    entries: [
      { syntax: '(…)', description: 'A capturing group. Numbered from 1 by its opening bracket.', pattern: '(\\d+)-(\\d+)', sample: 'pages 10-20' },
      { syntax: '(?:…)', description: 'A group that doesn’t capture, for grouping only.', pattern: '(?:ab)+', sample: 'ababab' },
      {
        syntax: '(?<name>…)',
        description: 'A named capturing group, read back as match.groups.name.',
        pattern: '(?<year>\\d{4})-(?<month>\\d{2})',
        sample: 'Due 2026-11',
      },
      { syntax: '\\1', description: 'Backreference: the same text that group 1 matched.', pattern: '(\\w)\\1', sample: 'hello' },
      { syntax: '\\k<name>', description: 'Backreference to a named group.', pattern: `(?<q>['"]).*?\\k<q>`, sample: 'say "hi" now' },
      { syntax: 'a|b', description: 'Either the left side or the right side.', pattern: 'cat|dog', sample: 'hot dog' },
      {
        syntax: '$1  $<name>  $&',
        description: 'In a replacement string: a numbered group, a named group, or the whole match.',
        pattern: '(\\w+) (\\w+)',
        sample: 'Ada Lovelace',
        note: '"Ada Lovelace".replace(/(\\w+) (\\w+)/, "$2, $1") gives "Lovelace, Ada".',
      },
    ],
  },
  {
    id: 'lookaround',
    name: 'Lookahead and lookbehind',
    summary: 'Check what comes before or after without including it in the match.',
    entries: [
      { syntax: '(?=…)', description: 'Lookahead: followed by this.', pattern: '\\d+(?=%)', sample: '50% off' },
      { syntax: '(?!…)', description: 'Negative lookahead: not followed by this.', pattern: '\\b\\d+\\b(?!%)', sample: '50% or 30' },
      { syntax: '(?<=…)', description: 'Lookbehind: preceded by this.', pattern: '(?<=\\$)\\d+', sample: 'costs $42' },
      { syntax: '(?<!…)', description: 'Negative lookbehind: not preceded by this.', pattern: '(?<!\\$)\\b\\d+', sample: '$42 or 17' },
    ],
  },
  {
    id: 'flags',
    name: 'Flags',
    summary: 'Letters after the closing slash, as in /cat/gi, that change how the whole pattern works.',
    entries: [
      {
        syntax: 'g',
        description: 'Global: find every match, not just the first. Needed for matchAll and replaceAll.',
        pattern: 'o',
        flags: 'g',
        sample: 'foo',
      },
      { syntax: 'i', description: 'Ignore case.', pattern: 'hello', flags: 'i', sample: 'HELLO' },
      { syntax: 'm', description: 'Multiline: ^ and $ match at the start and end of each line.', pattern: '^two', flags: 'm', sample: 'one\ntwo' },
      { syntax: 's', description: 'Dot all: . also matches line breaks.', pattern: 'one.two', flags: 's', sample: 'one\ntwo' },
      {
        syntax: 'u',
        description: 'Unicode: work in code points, and allow \\u{…} and \\p{…}. Also makes the syntax stricter.',
        pattern: '^.$',
        flags: 'u',
        sample: '𝒳',
        note: 'Without u, a character outside the Basic Multilingual Plane counts as two.',
      },
      {
        syntax: 'v',
        description: 'Unicode sets: everything u does, plus set subtraction and intersection inside [ ].',
        pattern: '[\\p{L}--[a-z]]',
        flags: 'v',
        sample: 'aBc',
        note: 'Use either u or v, not both.',
      },
      { syntax: 'y', description: 'Sticky: match only exactly at lastIndex, without searching ahead.', pattern: '\\d', flags: 'y', sample: '1a' },
      {
        syntax: 'd',
        description: 'Indices: each match gets an indices array with the start and end of every group.',
        pattern: '(\\d+)',
        flags: 'd',
        sample: 'abc 123',
      },
    ],
  },
  {
    id: 'unicode',
    name: 'Unicode property escapes',
    summary: 'Match characters by their Unicode category or script. All need the u or v flag.',
    entries: [
      { syntax: '\\p{L}', description: 'A letter in any language.', pattern: '\\p{L}+', flags: 'u', sample: 'Ngā mihi' },
      { syntax: '\\P{L}', description: 'Any character that isn’t a letter. Capital P negates any property.', pattern: '\\P{L}+', flags: 'u', sample: 'abc 123' },
      { syntax: '\\p{Lu}  \\p{Ll}', description: 'An uppercase or lowercase letter.', pattern: '\\p{Lu}', flags: 'u', sample: 'iPhone' },
      { syntax: '\\p{N}', description: 'A number in any script, not just 0–9.', pattern: '\\p{N}+', flags: 'u', sample: '٣٤٥' },
      { syntax: '\\p{P}', description: 'Punctuation.', pattern: '\\p{P}+', flags: 'u', sample: 'Wait… what?!' },
      { syntax: '\\p{Sc}', description: 'A currency symbol.', pattern: '\\p{Sc}\\d+', flags: 'u', sample: 'Only €20' },
      {
        syntax: '\\p{Script=Greek}',
        description: 'A character from one writing system. Script_Extensions (scx) also counts characters shared between scripts.',
        pattern: '\\p{Script=Greek}+',
        flags: 'u',
        sample: 'alpha is α',
      },
    ],
  },
  {
    id: 'common',
    name: 'Common patterns',
    summary: 'Good enough for a quick check. Read the caveat before trusting one.',
    entries: [
      {
        syntax: 'Email-ish',
        description: 'Something, an @, something, a dot, something, with no spaces.',
        pattern: '^[^\\s@]+@[^\\s@]+\\.[^\\s@]+$',
        sample: 'ada@example.com',
        note: 'Checks the rough shape only. Real addresses can be stranger than this, and the only real test is sending a message.',
      },
      {
        syntax: 'URL-ish',
        description: 'A web link starting with http:// or https://, up to the next space.',
        pattern: '\\bhttps?:\\/\\/[^\\s<>"]+',
        sample: 'See https://example.com/docs?page=2 for more',
        note: 'Often grabs a trailing full stop or bracket. To check a single URL properly, use new URL() instead.',
      },
      {
        syntax: 'ISO date',
        description: 'A YYYY-MM-DD date with a month from 01 to 12 and a day from 01 to 31.',
        pattern: '^\\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\\d|3[01])$',
        sample: '2026-11-23',
        note: 'Accepts dates that don’t exist, such as 2026-02-31. Check the real date in code.',
      },
      {
        syntax: 'Hex colour',
        description: 'A # followed by 3, 4, 6 or 8 hex digits.',
        pattern: '^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$',
        flags: 'i',
        sample: '#2f4bd8',
        note: 'Shape only. CSS also accepts names such as red and functions such as rgb().',
      },
      {
        syntax: 'IPv4 address',
        description: 'Four numbers from 0 to 255 separated by dots.',
        pattern: '^(?:(?:25[0-5]|2[0-4]\\d|1\\d\\d|[1-9]?\\d)\\.){3}(?:25[0-5]|2[0-4]\\d|1\\d\\d|[1-9]?\\d)$',
        sample: '192.168.1.10',
        note: 'Rejects 256 and leading zeros, but says nothing about whether the address is private, reserved or reachable.',
      },
    ],
  },
];

export function allEntries(sections: RegexSection[] = regexSections): RegexEntry[] {
  return sections.flatMap((s) => s.entries);
}

/** The sample text split around the first match of the example pattern. */
export function exampleMatch(e: RegexEntry): { before: string; match: string; after: string } | null {
  try {
    const m = new RegExp(e.pattern, e.flags ?? '').exec(e.sample);
    if (!m) return null;
    return { before: e.sample.slice(0, m.index), match: m[0], after: e.sample.slice(m.index + m[0].length) };
  } catch {
    return null;
  }
}

/** Every word must appear in the entry's syntax, description, pattern, note or section name. */
export function searchRegex(query: string, sections: RegexSection[] = regexSections): RegexSection[] {
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return sections;
  return sections
    .map((s) => ({
      ...s,
      entries: s.entries.filter((e) => {
        const hay = `${e.syntax} ${e.description} ${e.pattern} ${e.note ?? ''} ${s.name}`.toLowerCase();
        return words.every((w) => hay.includes(w));
      }),
    }))
    .filter((s) => s.entries.length);
}
