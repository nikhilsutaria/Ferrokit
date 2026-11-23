import { describe, expect, it } from 'vitest';
import { tomlToYaml, yamlToToml } from './yaml-toml';

describe('yamlToToml', () => {
  it('converts nested mappings into tables', () => {
    const out = yamlToToml('title: ferrokit\nserver:\n  port: 8080\n  tags: [pdf, dev]\n');
    expect(out).toBe('title = "ferrokit"\n\n[server]\nport = 8080\ntags = [ "pdf", "dev" ]\n');
  });

  it('explains values TOML cannot hold', () => {
    expect(() => yamlToToml('a: 1\nb:\n  c: null\n')).toThrow(/no null.*b\.c/);
    expect(() => yamlToToml('list: [1, ~]\n')).toThrow(/no null.*list\[1\]/);
    expect(() => yamlToToml('- 1\n- 2\n')).toThrow(/top level/);
    expect(() => yamlToToml('just text')).toThrow(/top level/);
  });

  it('refuses more than one document', () => {
    expect(() => yamlToToml('a: 1\n---\nb: 2\n')).toThrow(/2 documents/);
  });

  it('reports YAML errors with a line and column', () => {
    expect(() => yamlToToml('a: 1\n b: 2\n')).toThrow(/^Not valid YAML: line 2, column 3: /);
  });
});

describe('tomlToYaml', () => {
  it('converts tables and arrays', () => {
    expect(tomlToYaml('title = "ferrokit"\n\n[server]\nport = 8080\ntags = ["pdf", "dev"]\n')).toBe(
      'title: ferrokit\nserver:\n  port: 8080\n  tags:\n    - pdf\n    - dev\n',
    );
  });

  it('uses the chosen indent', () => {
    expect(tomlToYaml('[a]\nb = 1\n', 4)).toBe('a:\n    b: 1\n');
  });

  it('keeps dates readable', () => {
    expect(tomlToYaml('day = 2024-01-02\nat = 1979-05-27T07:32:00Z\n')).toBe('day: 2024-01-02\nat: 1979-05-27T07:32:00.000Z\n');
  });

  it('reports TOML errors with a line and column', () => {
    expect(() => tomlToYaml('a = 1\na = 2\n')).toThrow(/^Not valid TOML: line 2, column \d+: /);
  });

  it('round-trips with yamlToToml', () => {
    const yaml = 'name: x\nsizes:\n  - 1\n  - 2.5\nnested:\n  ok: true\n';
    expect(tomlToYaml(yamlToToml(yaml))).toBe(yaml);
  });
});
