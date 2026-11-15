// The office files LibreOffice handles, by kind. Mirrors `office::family` in ferrokit-worker.

export type Family = 'text' | 'sheet' | 'slides';

const EXT: Record<Family, string[]> = {
  text: ['doc', 'docx', 'odt', 'rtf', 'txt', 'wpd'],
  sheet: ['xls', 'xlsx', 'ods', 'csv'],
  slides: ['ppt', 'pptx', 'odp', 'pps', 'ppsx'],
};

export interface Target {
  ext: string;
  label: string;
}

const TARGETS: Record<Family, Target[]> = {
  text: [
    { ext: 'docx', label: 'Word (.docx)' },
    { ext: 'odt', label: 'OpenDocument text (.odt)' },
    { ext: 'rtf', label: 'Rich text (.rtf)' },
    { ext: 'txt', label: 'Plain text (.txt)' },
  ],
  sheet: [
    { ext: 'xlsx', label: 'Excel (.xlsx)' },
    { ext: 'ods', label: 'OpenDocument spreadsheet (.ods)' },
    { ext: 'csv', label: 'CSV (.csv), first sheet only' },
  ],
  slides: [
    { ext: 'pptx', label: 'PowerPoint (.pptx)' },
    { ext: 'odp', label: 'OpenDocument presentation (.odp)' },
  ],
};

export const FAMILY_NAME: Record<Family, string> = { text: 'documents', sheet: 'spreadsheets', slides: 'presentations' };

export const OFFICE_ACCEPT = Object.values(EXT)
  .flat()
  .map((e) => `.${e}`)
  .join(',');

export function familyOf(name: string): Family | null {
  const dot = name.lastIndexOf('.');
  if (dot <= 0) return null;
  const ext = name.slice(dot + 1).toLowerCase();
  return (Object.keys(EXT) as Family[]).find((f) => EXT[f].includes(ext)) ?? null;
}

export function targetsFor(family: Family): Target[] {
  return TARGETS[family];
}

export function mixedFamilies(names: string[]): boolean {
  return new Set(names.map(familyOf)).size > 1;
}
