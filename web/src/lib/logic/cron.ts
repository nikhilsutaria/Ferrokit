// Cron expression parser and next-run calculator (local time).
//
// Supports 5 fields (minute hour day-of-month month day-of-week) or 6 with seconds first;
// `*`, `?`, lists, ranges, steps, month and weekday names, and the @yearly-style macros.
// Like Vixie cron, when both day-of-month and day-of-week are restricted a day matches if
// EITHER matches; a field counts as unrestricted when it starts with `*` or `?`.

export interface CronSchedule {
  /** The expression in plain field form (macros expanded), suitable for describing. */
  normalized: string;
  hasSeconds: boolean;
  seconds: boolean[];
  minutes: boolean[];
  hours: boolean[];
  /** Index 1–31. */
  daysOfMonth: boolean[];
  /** Index 1–12. */
  months: boolean[];
  /** Index 0–6, Sunday = 0. */
  daysOfWeek: boolean[];
  domRestricted: boolean;
  dowRestricted: boolean;
}

const MACROS: Record<string, string> = {
  '@yearly': '0 0 1 1 *',
  '@annually': '0 0 1 1 *',
  '@monthly': '0 0 1 * *',
  '@weekly': '0 0 * * 0',
  '@daily': '0 0 * * *',
  '@midnight': '0 0 * * *',
  '@hourly': '0 * * * *',
};

const MONTH_NAMES = ['JAN', 'FEB', 'MAR', 'APR', 'MAY', 'JUN', 'JUL', 'AUG', 'SEP', 'OCT', 'NOV', 'DEC'];
const DAY_NAMES = ['SUN', 'MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT'];

interface FieldSpec {
  name: string;
  min: number;
  max: number;
  names?: string[];
  /** Offset added to a name's index (months start at 1). */
  nameBase?: number;
  allowQuestion?: boolean;
}

const SPECS = {
  second: { name: 'seconds', min: 0, max: 59 },
  minute: { name: 'minute', min: 0, max: 59 },
  hour: { name: 'hour', min: 0, max: 23 },
  dom: { name: 'day of month', min: 1, max: 31, allowQuestion: true },
  month: { name: 'month', min: 1, max: 12, names: MONTH_NAMES, nameBase: 1 },
  // 7 is accepted as another way to write Sunday.
  dow: { name: 'day of week', min: 0, max: 7, names: DAY_NAMES, nameBase: 0, allowQuestion: true },
} satisfies Record<string, FieldSpec>;

function parseValue(raw: string, spec: FieldSpec): number {
  const upper = raw.toUpperCase();
  if (spec.names) {
    const i = spec.names.indexOf(upper);
    if (i >= 0) return i + (spec.nameBase ?? 0);
  }
  if (!/^\d+$/.test(raw)) {
    if (/[LW#]/i.test(raw)) throw new Error(`“${raw}” in the ${spec.name} field uses L, W or #, which aren’t supported here.`);
    const hint = spec.names ? ` Use a number or a name like ${spec.names[0]}.` : '';
    throw new Error(`“${raw}” isn’t a valid ${spec.name}.${hint}`);
  }
  const n = Number(raw);
  if (n < spec.min || n > spec.max) throw new Error(`${spec.name[0].toUpperCase()}${spec.name.slice(1)} must be between ${spec.min} and ${spec.max}; got ${n}.`);
  return n;
}

export function parseField(text: string, spec: FieldSpec): { values: boolean[]; restricted: boolean } {
  const values = new Array<boolean>(spec.max + 1).fill(false);
  if (!text) throw new Error(`The ${spec.name} field is empty.`);
  const restricted = !(text.startsWith('*') || text.startsWith('?'));
  for (const item of text.split(',')) {
    if (!item) throw new Error(`The ${spec.name} field has an empty item; check for doubled or trailing commas.`);
    const [rangePart, stepPart, extra] = item.split('/');
    if (extra !== undefined) throw new Error(`“${item}” has more than one “/”.`);
    let step = 1;
    if (stepPart !== undefined) {
      if (!/^\d+$/.test(stepPart) || Number(stepPart) === 0) throw new Error(`The step in “${item}” must be a whole number of 1 or more.`);
      step = Number(stepPart);
    }
    let lo: number;
    let hi: number;
    if (rangePart === '*' || rangePart === '?') {
      if (rangePart === '?' && !spec.allowQuestion) throw new Error(`“?” is only allowed in the day-of-month and day-of-week fields.`);
      if (rangePart === '?' && stepPart !== undefined) throw new Error(`“?” can’t have a step.`);
      lo = spec.min;
      hi = spec.max === 7 ? 6 : spec.max;
    } else if (rangePart.includes('-')) {
      const [a, b, more] = rangePart.split('-');
      if (more !== undefined || !a || !b) throw new Error(`“${rangePart}” isn’t a valid range; write it like 1-5.`);
      lo = parseValue(a, spec);
      hi = parseValue(b, spec);
      if (hi < lo) throw new Error(`The range “${rangePart}” runs backwards; put the smaller value first.`);
    } else {
      lo = parseValue(rangePart, spec);
      // "5/15" means from 5 to the end, every 15.
      hi = stepPart !== undefined ? (spec.max === 7 ? 6 : spec.max) : lo;
    }
    for (let v = lo; v <= hi; v += step) values[v] = true;
  }
  if (spec.max === 7) {
    if (values[7]) values[0] = true;
    values.length = 7;
  }
  return { values, restricted };
}

export function parseCron(expression: string): CronSchedule {
  let expr = expression.trim().replace(/\s+/g, ' ');
  if (!expr) throw new Error('Enter a cron expression, such as */5 * * * *.');
  if (expr.startsWith('@')) {
    const macro = MACROS[expr.toLowerCase()];
    if (expr.toLowerCase() === '@reboot') throw new Error('@reboot runs once at startup, so it has no schedule to show.');
    if (!macro) throw new Error(`“${expr}” isn’t a known shortcut. Try @yearly, @monthly, @weekly, @daily or @hourly.`);
    expr = macro;
  }
  const f = expr.split(' ');
  if (f.length !== 5 && f.length !== 6) {
    throw new Error(`Expected 5 fields (minute hour day month weekday) or 6 with seconds first; found ${f.length}.`);
  }
  const hasSeconds = f.length === 6;
  const [sec, min, hour, dom, month, dow] = hasSeconds ? f : ['0', ...f];
  const d = parseField(dom, SPECS.dom);
  const w = parseField(dow, SPECS.dow);
  return {
    normalized: expr,
    hasSeconds,
    seconds: parseField(sec, SPECS.second).values,
    minutes: parseField(min, SPECS.minute).values,
    hours: parseField(hour, SPECS.hour).values,
    daysOfMonth: d.values,
    months: parseField(month, SPECS.month).values,
    daysOfWeek: w.values,
    domRestricted: d.restricted,
    dowRestricted: w.restricted,
  };
}

function dayMatches(s: CronSchedule, date: Date): boolean {
  const dom = s.daysOfMonth[date.getDate()];
  const dow = s.daysOfWeek[date.getDay()];
  if (s.domRestricted && s.dowRestricted) return dom || dow;
  if (s.domRestricted) return dom;
  if (s.dowRestricted) return dow;
  return true;
}

function nextIn(values: boolean[], after: number): number {
  for (let v = after + 1; v < values.length; v++) if (values[v]) return v;
  return -1;
}

/**
 * The next `count` run times strictly after `from`, in the local time zone. Stops early if the
 * schedule can never run (such as 30 February) or there are no runs within `years` years.
 */
export function nextRuns(schedule: CronSchedule, from: Date, count = 10, years = 30): Date[] {
  const out: Date[] = [];
  let d = new Date(Math.floor(from.getTime() / 1000) * 1000 + 1000);
  const limitYear = from.getFullYear() + years;
  let guard = 0;

  const advance = (next: Date) => {
    // Never move backwards, even across daylight-saving changes.
    d = next.getTime() > d.getTime() ? next : new Date(d.getTime() + 1000);
  };

  while (out.length < count && guard++ < 200_000) {
    if (d.getFullYear() > limitYear) break;
    const y = d.getFullYear();
    const mo = d.getMonth();
    const day = d.getDate();
    const h = d.getHours();
    const mi = d.getMinutes();
    const se = d.getSeconds();

    if (!schedule.months[mo + 1]) {
      advance(new Date(y, mo + 1, 1));
      continue;
    }
    if (!dayMatches(schedule, d)) {
      advance(new Date(y, mo, day + 1));
      continue;
    }
    if (!schedule.hours[h]) {
      const nh = nextIn(schedule.hours, h);
      advance(nh < 0 ? new Date(y, mo, day + 1) : new Date(y, mo, day, nh));
      continue;
    }
    if (!schedule.minutes[mi]) {
      const nm = nextIn(schedule.minutes, mi);
      advance(nm < 0 ? new Date(y, mo, day, h + 1) : new Date(d.getTime() + (nm - mi) * 60_000 - se * 1000));
      continue;
    }
    if (!schedule.seconds[se]) {
      const ns = nextIn(schedule.seconds, se);
      advance(ns < 0 ? new Date(d.getTime() + (60 - se) * 1000) : new Date(d.getTime() + (ns - se) * 1000));
      continue;
    }
    out.push(d);
    d = new Date(d.getTime() + 1000);
  }
  return out;
}
