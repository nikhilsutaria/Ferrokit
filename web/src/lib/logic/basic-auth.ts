// HTTP Basic authentication: "Basic " + Base64(username ":" password), with UTF-8 text (RFC 7617).

import { base64ToBytes, encodeBase64 } from './base64';

/** The header value, e.g. "Basic dXNlcjpwYXNz". Throws if the username holds a colon. */
export function basicAuthValue(username: string, password: string): string {
  if (username.includes(':')) {
    throw new Error('The username can’t contain a colon, because Basic auth uses the first colon to split it from the password.');
  }
  return `Basic ${encodeBase64(`${username}:${password}`)}`;
}

export function curlExample(value: string, url = 'https://example.com'): string {
  return `curl -H 'Authorization: ${value}' ${url}`;
}

export interface Credentials {
  username: string;
  password: string;
}

/** Read a header line, a header value or bare Base64 back into a username and password. */
export function parseBasicAuth(header: string): Credentials {
  let s = header.trim();
  if (!s) throw new Error('Paste an Authorization header or its Base64 value.');
  s = s.replace(/^authorization\s*:\s*/i, '');
  if (/^basic$/i.test(s)) throw new Error('Paste the Base64 value that comes after “Basic” too.');
  const scheme = /^(\S+)\s+(\S.*)$/s.exec(s);
  if (scheme) {
    if (scheme[1].toLowerCase() !== 'basic') {
      throw new Error(`This is a “${scheme[1]}” header, not Basic. Only Basic headers hold a username and password.`);
    }
    s = scheme[2].trim();
  }
  const bytes = base64ToBytes(s);
  let text: string;
  try {
    text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    throw new Error('The value decodes to bytes that aren’t UTF-8 text, so it isn’t a username and password.');
  }
  const colon = text.indexOf(':');
  if (colon < 0) {
    throw new Error('The decoded value has no colon, so it can’t be split into a username and password.');
  }
  return { username: text.slice(0, colon), password: text.slice(colon + 1) };
}
