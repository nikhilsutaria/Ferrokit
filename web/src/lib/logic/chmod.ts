// Unix permission modes: a number from 0 to 0o7777 as octal, symbolic and bits.

export const SETUID = 0o4000;
export const SETGID = 0o2000;
export const STICKY = 0o1000;

export const WHO = ['owner', 'group', 'others'] as const;
export const PERMS = ['read', 'write', 'execute'] as const;

/** The bit for a who/permission pair, e.g. owner+read = 0o400. */
export function bit(who: number, perm: number): number {
  return 1 << ((2 - who) * 3 + (2 - perm));
}

export function parseOctal(text: string): number {
  const s = text.trim().replace(/^0o/i, '');
  if (!/^[0-7]{1,4}$/.test(s)) throw new Error('Use 3 or 4 octal digits (0–7), such as 755 or 4755.');
  return parseInt(s, 8);
}

/** Three digits, or four when a special bit is set. */
export function toOctal(mode: number): string {
  const s = (mode & 0o7777).toString(8);
  return mode & 0o7000 ? s.padStart(4, '0') : s.padStart(3, '0');
}

export function toSymbolic(mode: number): string {
  const ch = (on: number, c: string) => (mode & on ? c : '-');
  const exec = (x: number, special: number, lower: string) => {
    if (mode & special) return mode & x ? lower : lower.toUpperCase();
    return mode & x ? 'x' : '-';
  };
  return (
    ch(0o400, 'r') + ch(0o200, 'w') + exec(0o100, SETUID, 's') +
    ch(0o040, 'r') + ch(0o020, 'w') + exec(0o010, SETGID, 's') +
    ch(0o004, 'r') + ch(0o002, 'w') + exec(0o001, STICKY, 't')
  );
}

/** Parse "rwxr-xr-x" (an optional leading type character such as - or d is ignored). */
export function parseSymbolic(text: string): number {
  let s = text.trim();
  if (s.length === 10) s = s.slice(1);
  if (s.length !== 9) throw new Error('Use nine characters like rwxr-xr-x.');
  let mode = 0;
  const specials = [SETUID, SETGID, STICKY];
  for (let who = 0; who < 3; who++) {
    const [r, w, x] = s.slice(who * 3, who * 3 + 3);
    if (r === 'r') mode |= bit(who, 0);
    else if (r !== '-') throw new Error(`Position ${who * 3 + 1} must be r or -.`);
    if (w === 'w') mode |= bit(who, 1);
    else if (w !== '-') throw new Error(`Position ${who * 3 + 2} must be w or -.`);
    const sp = who === 2 ? 't' : 's';
    if (x === 'x') mode |= bit(who, 2);
    else if (x === sp) mode |= bit(who, 2) | specials[who];
    else if (x === sp.toUpperCase()) mode |= specials[who];
    else if (x !== '-') throw new Error(`Position ${who * 3 + 3} must be x, ${sp}, ${sp.toUpperCase()} or -.`);
  }
  return mode;
}

export function chmodCommand(mode: number, file = 'file'): string {
  return `chmod ${toOctal(mode)} ${file}`;
}
