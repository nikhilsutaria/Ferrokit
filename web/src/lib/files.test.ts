import { describe, expect, it } from 'vitest';
import { formatBytes, stem } from './files';
import { filenameFrom } from './api';

describe('files', () => {
  it('formats sizes', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(50 * 1024 * 1024)).toBe('50 MB');
  });
  it('strips extensions', () => {
    expect(stem('report.final.pdf')).toBe('report.final');
    expect(stem('.env')).toBe('.env');
  });
  it('reads content-disposition, preferring the UTF-8 name', () => {
    expect(filenameFrom(`attachment; filename="R_sum_.pdf"; filename*=UTF-8''R%C3%A9sum%C3%A9.pdf`)).toBe('Résumé.pdf');
    expect(filenameFrom('attachment; filename="a.pdf"')).toBe('a.pdf');
    expect(filenameFrom(null)).toBeNull();
  });
});
