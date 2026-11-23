// A thin wrapper over sql-formatter with plain-language parse errors.
import { format, type KeywordCase, type SqlLanguage } from 'sql-formatter';

export const DIALECTS: { id: SqlLanguage; label: string }[] = [
  { id: 'sql', label: 'Standard SQL' },
  { id: 'postgresql', label: 'PostgreSQL' },
  { id: 'mysql', label: 'MySQL' },
  { id: 'mariadb', label: 'MariaDB' },
  { id: 'sqlite', label: 'SQLite' },
  { id: 'transactsql', label: 'SQL Server (T-SQL)' },
  { id: 'bigquery', label: 'BigQuery' },
  { id: 'snowflake', label: 'Snowflake' },
  { id: 'plsql', label: 'Oracle PL/SQL' },
];

export interface SqlFormatOptions {
  dialect?: SqlLanguage;
  keywordCase?: KeywordCase;
  indent?: 2 | 4;
}

function plainError(message: string, label: string): string {
  const first = message.split('\n')[0];
  const lead = `Couldn’t read this as ${label}:`;
  const unexpected = /^Parse error: Unexpected "([\s\S]*?)" at line (\d+) column (\d+)/.exec(first);
  if (unexpected) {
    const snippet = unexpected[1].length > 40 ? unexpected[1].slice(0, 39) + '…' : unexpected[1];
    return `${lead} unexpected “${snippet}” at line ${unexpected[2]}, column ${unexpected[3]}. Check the SQL, or pick another dialect.`;
  }
  const token = /^Parse error at token: (.*) at line (\d+) column (\d+)/.exec(first);
  if (token) {
    if (token[1] === '«EOF»') {
      return `${lead} the query ended too early (line ${token[2]}, column ${token[3]}). Check for a missing closing bracket, quote or keyword.`;
    }
    return `${lead} unexpected “${token[1]}” at line ${token[2]}, column ${token[3]}. Check the SQL, or pick another dialect.`;
  }
  return `${lead} ${first.replace(/^Parse error:\s*/, '')}`;
}

export function formatSql(sql: string, { dialect = 'sql', keywordCase = 'preserve', indent = 2 }: SqlFormatOptions = {}): string {
  try {
    return format(sql, { language: dialect, keywordCase, tabWidth: indent });
  } catch (e) {
    const label = DIALECTS.find((d) => d.id === dialect)?.label ?? dialect;
    throw new Error(plainError((e as Error).message, label));
  }
}
