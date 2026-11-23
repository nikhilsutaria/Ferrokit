import { describe, expect, it } from 'vitest';
import { DIALECTS, formatSql } from './sql-format';

describe('formatSql', () => {
  it('puts clauses on their own lines', () => {
    expect(formatSql('select a, b from t where x = 1 and y = 2')).toBe(
      'select\n  a,\n  b\nfrom\n  t\nwhere\n  x = 1\n  and y = 2',
    );
  });

  it('changes keyword case and indent', () => {
    expect(formatSql('select a from t', { keywordCase: 'upper', indent: 4 })).toBe('SELECT\n    a\nFROM\n    t');
    expect(formatSql('SELECT a FROM t', { keywordCase: 'lower' })).toBe('select\n  a\nfrom\n  t');
    expect(formatSql('SELECT a from t', { keywordCase: 'preserve' })).toBe('SELECT\n  a\nfrom\n  t');
  });

  it('understands dialect-specific syntax', () => {
    expect(formatSql('select top 5 [name] from dbo.users', { dialect: 'transactsql' })).toContain('[name]');
    expect(formatSql('select `name` from users', { dialect: 'mysql' })).toContain('`name`');
    expect(formatSql('select a::int from t', { dialect: 'postgresql' })).toContain('a::int');
  });

  it('explains parse errors plainly, with the position', () => {
    expect(() => formatSql('select `a` from t', { dialect: 'postgresql' })).toThrow(
      /^Couldn’t read this as PostgreSQL: unexpected “`a` FROM t” at line 1, column 8\. Check the SQL, or pick another dialect\.$/i,
    );
    expect(() => formatSql('select * from t where (a = 1')).toThrow(/ended too early/);
  });

  it('offers every dialect in the list', () => {
    for (const d of DIALECTS) expect(() => formatSql('select 1', { dialect: d.id })).not.toThrow();
    expect(DIALECTS.map((d) => d.label)).toContain('SQL Server (T-SQL)');
  });
});
