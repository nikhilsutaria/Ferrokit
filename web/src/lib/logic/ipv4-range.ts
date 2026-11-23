// An IPv4 range (start to end) as the fewest CIDR blocks, and the smallest single block that holds it.
// Uses plain arithmetic rather than bit operators, so 2^32 (a /0) doesn't overflow.

import { formatIp } from './ipv4';

export interface Block {
  network: number;
  prefix: number;
}

export interface RangeResult {
  start: string;
  end: string;
  /** True when the start was after the end and the two were swapped. */
  swapped: boolean;
  count: number;
  blocks: string[];
  cover: { cidr: string; size: number; extra: number };
}

const cidr = (b: Block) => `${formatIp(b.network)}/${b.prefix}`;

/** The fewest CIDR blocks that cover exactly start..end (inclusive). */
export function rangeToBlocks(start: number, end: number): Block[] {
  const out: Block[] = [];
  let at = start;
  while (at <= end) {
    // Grow the block while it stays aligned and inside the range.
    let k = 0;
    while (k < 32 && at % 2 ** (k + 1) === 0 && 2 ** (k + 1) <= end - at + 1) k++;
    out.push({ network: at, prefix: 32 - k });
    at += 2 ** k;
  }
  return out;
}

/** The smallest single CIDR block that contains both addresses. */
export function coveringCidr(a: number, b: number): Block {
  for (let prefix = 32; prefix > 0; prefix--) {
    const size = 2 ** (32 - prefix);
    if (Math.floor(a / size) === Math.floor(b / size)) return { network: Math.floor(a / size) * size, prefix };
  }
  return { network: 0, prefix: 0 };
}

export function expandRange(start: number, end: number): RangeResult {
  const swapped = start > end;
  if (swapped) [start, end] = [end, start];
  const count = end - start + 1;
  const cover = coveringCidr(start, end);
  const size = 2 ** (32 - cover.prefix);
  return {
    start: formatIp(start),
    end: formatIp(end),
    swapped,
    count,
    blocks: rangeToBlocks(start, end).map(cidr),
    cover: { cidr: cidr(cover), size, extra: size - count },
  };
}
