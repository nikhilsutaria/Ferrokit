// Text statistics.

export const READING_WPM = 238;
export const SPEAKING_WPM = 150;

const STOP_WORDS = new Set(
  (
    'a about above after again against all am an and any are as at be because been before being below between both but by ' +
    'can could did do does doing down during each few for from further had has have having he her here hers herself him ' +
    'himself his how i if in into is it its itself just me more most my myself no nor not now of off on once only or other ' +
    'our ours ourselves out over own same she should so some such than that the their theirs them themselves then there ' +
    'these they this those through to too under until up very was we were what when where which while who whom why will ' +
    'with would you your yours yourself yourselves it’s i’m don’t can’t won’t isn’t it\'s i\'m don\'t can\'t won\'t isn\'t'
  ).split(' '),
);

const WORD_RE = /[\p{L}\p{N}\p{M}]+(?:['’\-][\p{L}\p{N}\p{M}]+)*/gu;

export interface TextStats {
  characters: number;
  charactersNoSpaces: number;
  words: number;
  lines: number;
  sentences: number;
  paragraphs: number;
  averageWordLength: number;
  readingSeconds: number;
  speakingSeconds: number;
  topWords: { word: string; count: number }[];
}

/** Split into user-perceived characters (grapheme clusters) when the browser can. */
export function graphemes(text: string): string[] {
  if (typeof Intl !== 'undefined' && 'Segmenter' in Intl) {
    const seg = new Intl.Segmenter(undefined, { granularity: 'grapheme' });
    return Array.from(seg.segment(text), (s) => s.segment);
  }
  return Array.from(text);
}

export function words(text: string): string[] {
  return text.match(WORD_RE) ?? [];
}

export function countSentences(text: string): number {
  // A sentence ends at . ! ? or … (plus closing quotes or brackets) followed by space or the end,
  // or at a blank line. Any trailing text without punctuation counts as a sentence too.
  const pieces = text.split(/(?<=[.!?…]+["'”’)\]]*)\s+|\n\s*\n/);
  return pieces.filter((p) => /[\p{L}\p{N}]/u.test(p)).length;
}

export function stats(text: string): TextStats {
  const g = graphemes(text);
  const w = words(text);
  const letters = w.reduce((sum, word) => sum + graphemes(word.replace(/['’-]/g, '')).length, 0);

  const counts = new Map<string, number>();
  for (const word of w) {
    const lower = word.toLowerCase().replace(/’/g, "'");
    if (STOP_WORDS.has(lower) || /^\p{N}+$/u.test(lower)) continue;
    counts.set(lower, (counts.get(lower) ?? 0) + 1);
  }
  const topWords = [...counts]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, 10)
    .map(([word, count]) => ({ word, count }));

  return {
    characters: g.length,
    charactersNoSpaces: g.filter((c) => !/^\s+$/u.test(c)).length,
    words: w.length,
    lines: text === '' ? 0 : text.split(/\r\n|\r|\n/).length,
    sentences: countSentences(text),
    paragraphs: text.split(/(?:\r?\n[ \t]*){2,}/).filter((p) => p.trim()).length,
    averageWordLength: w.length ? letters / w.length : 0,
    readingSeconds: (w.length / READING_WPM) * 60,
    speakingSeconds: (w.length / SPEAKING_WPM) * 60,
    topWords,
  };
}

/** "45 s", "3 min 20 s", "1 h 5 min". */
export function formatDuration(seconds: number): string {
  const s = Math.round(seconds);
  if (s < 60) return `${s} s`;
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const rest = s % 60;
  if (h) return m ? `${h} h ${m} min` : `${h} h`;
  return rest ? `${m} min ${rest} s` : `${m} min`;
}
