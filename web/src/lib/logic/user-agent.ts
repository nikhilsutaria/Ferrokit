// Turn a user-agent string into readable browser, engine, OS, device and CPU lines (ua-parser-js v1).

import UAParser from 'ua-parser-js';

export interface UserAgentInfo {
  browser: string;
  engine: string;
  os: string;
  device: string;
  cpu: string;
  /** The crawler's name when the string looks like a bot, else null. */
  bot: string | null;
}

const UNKNOWN = 'Unknown';

function join(...parts: (string | undefined)[]): string {
  return parts.filter(Boolean).join(' ') || UNKNOWN;
}

function capitalise(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export function describeUserAgent(ua: string): UserAgentInfo {
  const r = new UAParser(ua).getResult();
  const d = r.device;
  const kind = d.type ? capitalise(d.type === 'smarttv' ? 'smart TV' : d.type) : 'Desktop';
  const model = [d.vendor, d.model].filter(Boolean).join(' ');
  const bot = /([\w.-]*(?:bot|crawler|spider|slurp)[\w.-]*)/i.exec(ua.replace(/https?:\/\/\S+/g, ''));
  return {
    browser: join(r.browser.name, r.browser.version),
    engine: join(r.engine.name, r.engine.version),
    os: join(r.os.name, r.os.version),
    device: model ? `${kind}, ${model}` : kind,
    cpu: r.cpu.architecture || UNKNOWN,
    bot: bot ? bot[1] : null,
  };
}
