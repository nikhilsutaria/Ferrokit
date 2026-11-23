import { describe, expect, it } from 'vitest';
import { gitGroups, searchGit } from './git-cheatsheet';

const all = () => gitGroups.flatMap((g) => g.commands);
const found = (q: string) => searchGit(q).flatMap((g) => g.commands.map((c) => c.command));

describe('git cheat sheet data', () => {
  it('has the expected groups in order', () => {
    expect(gitGroups.map((g) => g.name)).toEqual([
      'Set up',
      'Start a repository',
      'Everyday work',
      'Branches',
      'Remotes',
      'Undoing things',
      'History and inspecting',
      'Stash',
      'Tags',
      'Cleaning up',
    ]);
    for (const g of gitGroups) expect(g.commands.length).toBeGreaterThan(0);
  });

  it('has no duplicate commands', () => {
    const cmds = all().map((c) => c.command);
    expect(new Set(cmds).size).toBe(cmds.length);
  });

  it('starts every command with git and describes it', () => {
    for (const c of all()) {
      expect(c.command.startsWith('git ')).toBe(true);
      expect(c.description.length).toBeGreaterThan(10);
    }
  });

  it('uses modern commands', () => {
    const cmds = all().map((c) => c.command);
    expect(cmds).toContain('git switch -c <branch>');
    expect(cmds).toContain('git restore --staged <file>');
    expect(cmds.some((c) => c.startsWith('git checkout'))).toBe(false);
  });

  it('covers all three reset modes and warns about --hard', () => {
    const cmds = all();
    for (const mode of ['--soft', '--mixed', '--hard']) expect(cmds.some((c) => c.command.includes(`reset ${mode}`))).toBe(true);
    const hard = cmds.find((c) => c.command.includes('reset --hard'))!;
    expect(hard.warning).toMatch(/lost|can.t/i);
    expect(cmds.some((c) => c.command === 'git reflog')).toBe(true);
  });
});

describe('searchGit', () => {
  it('returns everything for an empty query', () => {
    expect(searchGit('')).toEqual(gitGroups);
  });

  it('finds by command', () => {
    expect(found('switch -c')).toContain('git switch -c <branch>');
    expect(found('stash pop')).toEqual(['git stash pop']);
  });

  it('finds by description words in any order', () => {
    const res = found('away throw');
    expect(res).toContain('git reset --hard HEAD~1');
    expect(res).toContain('git restore <file>');
  });

  it('is case insensitive and drops empty groups', () => {
    expect(found('REFLOG')).toContain('git reflog');
    expect(searchGit('zzz-not-a-command')).toEqual([]);
  });
});
