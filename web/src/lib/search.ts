import { categoryById, type Tool } from './registry';

const norm = (s: string) =>
  s
    .toLowerCase()
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/[^a-z0-9]+/g, ' ')
    .trim();

interface Indexed {
  tool: Tool;
  name: string[];
  keywords: string[];
  rest: string[];
}

function index(tools: Tool[]): Indexed[] {
  return tools.map((tool) => ({
    tool,
    name: norm(tool.name).split(' '),
    keywords: tool.keywords.flatMap((k) => norm(k).split(' ')),
    rest: norm(`${tool.description} ${categoryById.get(tool.category)?.name ?? ''} ${tool.id}`).split(' '),
  }));
}

function termScore(term: string, words: string[], exact: number, prefix: number, infix: number): number {
  let best = 0;
  for (const w of words) {
    if (w === term) return exact;
    if (w.startsWith(term)) best = Math.max(best, prefix);
    else if (term.length >= 3 && w.includes(term)) best = Math.max(best, infix);
  }
  return best;
}

let cache: { tools: Tool[]; idx: Indexed[] } | null = null;

/**
 * Rank tools for a query. Every term must match somewhere; name matches beat keyword
 * matches beat description matches, and ready tools rank above planned ones.
 */
export function search(tools: Tool[], query: string, limit = 50): Tool[] {
  const terms = norm(query).split(' ').filter(Boolean);
  if (terms.length === 0) return [];
  if (cache?.tools !== tools) cache = { tools, idx: index(tools) };

  const scored: { tool: Tool; score: number }[] = [];
  for (const item of cache.idx) {
    let total = 0;
    let ok = true;
    for (const term of terms) {
      const s = Math.max(
        termScore(term, item.name, 100, 60, 25),
        termScore(term, item.keywords, 70, 45, 15),
        termScore(term, item.rest, 30, 20, 8),
      );
      if (s === 0) {
        ok = false;
        break;
      }
      total += s;
    }
    if (!ok) continue;
    if (item.tool.ready) total += 40;
    scored.push({ tool: item.tool, score: total });
  }
  scored.sort((a, b) => b.score - a.score || a.tool.name.localeCompare(b.tool.name));
  return scored.slice(0, limit).map((s) => s.tool);
}
