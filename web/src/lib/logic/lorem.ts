// Lorem ipsum generator with a built-in Latin word list.

const WORDS = (
  'lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor incididunt ut labore et dolore magna ' +
  'aliqua enim ad minim veniam quis nostrud exercitation ullamco laboris nisi aliquip ex ea commodo consequat duis aute ' +
  'irure in reprehenderit voluptate velit esse cillum fugiat nulla pariatur excepteur sint occaecat cupidatat non proident ' +
  'sunt culpa qui officia deserunt mollit anim id est laborum praesent elementum facilisis leo vel fringilla phasellus ' +
  'faucibus scelerisque eleifend donec pretium vulputate sapien nec sagittis aliquam malesuada bibendum arcu vitae ' +
  'pellentesque habitant morbi tristique senectus netus fames ac turpis egestas integer feugiat scelerisque varius quam ' +
  'quisque mauris cursus mattis molestie a iaculis at erat viverra justo nibh tellus orci porta non pulvinar neque ' +
  'laoreet suspendisse interdum posuere lorem ipsum etiam dignissim diam nunc congue sem fermentum tortor dui accumsan ' +
  'lacus euismod massa tincidunt odio purus semper eget duis libero volutpat blandit cras adipiscing ornare urna ' +
  'condimentum mattis nullam ultrices risus gravida hendrerit lectus vestibulum rhoncus est pellentesque mi proin ' +
  'sagittis nisl amet commodo nulla facilisi tempus imperdiet augue ligula ullamcorper mauris rutrum quisque tortor'
).split(' ');

export const LOREM_START = 'Lorem ipsum dolor sit amet, consectetur adipiscing elit';

export type LoremUnit = 'paragraphs' | 'sentences' | 'words';

export interface LoremOptions {
  unit: LoremUnit;
  count: number;
  startWithLorem: boolean;
  /** Returns a number in [0, 1); defaults to Math.random. */
  random?: () => number;
}

export function generateLorem({ unit, count, startWithLorem, random = Math.random }: LoremOptions): string[] {
  const n = Math.max(0, Math.floor(count));
  const pick = () => WORDS[Math.floor(random() * WORDS.length)];
  const between = (lo: number, hi: number) => lo + Math.floor(random() * (hi - lo + 1));
  const capital = (s: string) => s[0].toUpperCase() + s.slice(1);

  const sentence = () => {
    const words = Array.from({ length: between(6, 14) }, pick);
    // An occasional comma reads more naturally.
    if (words.length > 8 && random() < 0.5) words[between(2, words.length - 4)] += ',';
    return `${capital(words.join(' '))}.`;
  };

  if (unit === 'words') {
    const words = Array.from({ length: n }, pick);
    if (startWithLorem) {
      const start = LOREM_START.toLowerCase().replace(',', '').split(' ');
      words.splice(0, Math.min(n, start.length), ...start.slice(0, n));
    }
    return n ? [capital(words.join(' '))] : [];
  }

  const sentences = (k: number) => Array.from({ length: k }, sentence);
  const blocks: string[][] = unit === 'sentences' ? [sentences(n)] : Array.from({ length: n }, () => sentences(between(4, 7)));
  if (n && startWithLorem) blocks[0][0] = `${LOREM_START}.`;
  return blocks.filter((b) => b.length).map((b) => b.join(' '));
}
