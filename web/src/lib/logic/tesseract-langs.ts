// Tesseract's language and script codes → names people recognise.

const NAMES: Record<string, string> = {
  afr: 'Afrikaans', amh: 'Amharic', ara: 'Arabic', asm: 'Assamese', aze: 'Azerbaijani', aze_cyrl: 'Azerbaijani (Cyrillic)',
  bel: 'Belarusian', ben: 'Bengali', bod: 'Tibetan', bos: 'Bosnian', bre: 'Breton', bul: 'Bulgarian', cat: 'Catalan',
  ceb: 'Cebuano', ces: 'Czech', chi_sim: 'Chinese (Simplified)', chi_tra: 'Chinese (Traditional)', chr: 'Cherokee',
  cos: 'Corsican', cym: 'Welsh', dan: 'Danish', deu: 'German', div: 'Dhivehi', dzo: 'Dzongkha', ell: 'Greek',
  eng: 'English', enm: 'English (Middle, 1100–1500)', epo: 'Esperanto', est: 'Estonian', eus: 'Basque', fao: 'Faroese',
  fas: 'Persian', fil: 'Filipino', fin: 'Finnish', fra: 'French', frk: 'German (Fraktur)', frm: 'French (Middle, 1400–1600)',
  fry: 'Frisian', gla: 'Scottish Gaelic', gle: 'Irish', glg: 'Galician', grc: 'Greek (Ancient)', guj: 'Gujarati',
  hat: 'Haitian Creole', heb: 'Hebrew', hin: 'Hindi', hrv: 'Croatian', hun: 'Hungarian', hye: 'Armenian', iku: 'Inuktitut',
  ind: 'Indonesian', isl: 'Icelandic', ita: 'Italian', ita_old: 'Italian (Old)', jav: 'Javanese', jpn: 'Japanese',
  kan: 'Kannada', kat: 'Georgian', kat_old: 'Georgian (Old)', kaz: 'Kazakh', khm: 'Khmer', kir: 'Kyrgyz',
  kmr: 'Kurdish (Kurmanji)', kor: 'Korean', lao: 'Lao', lat: 'Latin', lav: 'Latvian', lit: 'Lithuanian',
  ltz: 'Luxembourgish', mal: 'Malayalam', mar: 'Marathi', mkd: 'Macedonian', mlt: 'Maltese', mon: 'Mongolian',
  mri: 'Māori', msa: 'Malay', mya: 'Burmese', nep: 'Nepali', nld: 'Dutch', nor: 'Norwegian', oci: 'Occitan', ori: 'Odia',
  pan: 'Punjabi', pol: 'Polish', por: 'Portuguese', pus: 'Pashto', que: 'Quechua', ron: 'Romanian', rus: 'Russian',
  san: 'Sanskrit', sin: 'Sinhala', slk: 'Slovak', slv: 'Slovenian', snd: 'Sindhi', spa: 'Spanish', spa_old: 'Spanish (Old)',
  sqi: 'Albanian', srp: 'Serbian', srp_latn: 'Serbian (Latin)', sun: 'Sundanese', swa: 'Swahili', swe: 'Swedish',
  syr: 'Syriac', tam: 'Tamil', tat: 'Tatar', tel: 'Telugu', tgk: 'Tajik', tha: 'Thai', tir: 'Tigrinya', ton: 'Tongan',
  tur: 'Turkish', uig: 'Uyghur', ukr: 'Ukrainian', urd: 'Urdu', uzb: 'Uzbek', uzb_cyrl: 'Uzbek (Cyrillic)',
  vie: 'Vietnamese', yid: 'Yiddish', yor: 'Yoruba',
};

/** Script models (capitalised codes) read any language written in that script. */
const SCRIPTS: Record<string, string> = {
  HanS: 'Chinese characters, simplified',
  HanT: 'Chinese characters, traditional',
  Canadian_Aboriginal: 'Canadian Aboriginal syllabics',
};

const isScript = (code: string) => /^[A-Z]/.test(code);

export function langName(code: string): string {
  const vertical = code.endsWith('_vert');
  const base = vertical ? code.slice(0, -'_vert'.length) : code;
  let name: string;
  if (isScript(base)) {
    const script = SCRIPTS[base] ?? base.replace(/_/g, ' ');
    name = vertical ? `${script} script` : `${script} script (any language)`;
  } else {
    name = NAMES[base] ?? '';
    if (!name) return code;
  }
  return vertical ? `${name} (vertical)` : name;
}

/** English first, then languages by name, then script models. */
export function sortLangs(codes: string[]): string[] {
  const rank = (c: string) => (c === 'eng' ? 0 : isScript(c) ? 2 : 1);
  return [...codes].sort((a, b) => rank(a) - rank(b) || langName(a).localeCompare(langName(b)));
}

const fold = (s: string) => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();

/** Whether `query` finds `code`, by name or code, ignoring case and accents ("maori" finds Māori). */
export function matchesLang(code: string, query: string): boolean {
  const q = fold(query.trim());
  return !q || fold(langName(code)).includes(q) || fold(code).includes(q);
}
