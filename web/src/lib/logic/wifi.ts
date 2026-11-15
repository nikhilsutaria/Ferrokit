// Wi-Fi network QR payloads (the ZXing "WIFI:" format understood by iOS and Android cameras).

export type WifiSecurity = 'WPA' | 'WEP' | 'nopass';

export interface WifiNetwork {
  ssid: string;
  password: string;
  security: WifiSecurity;
  hidden: boolean;
}

/** Backslash-escape the characters that are special in the WIFI: format: \ ; , : " */
export function escapeWifi(value: string): string {
  return value.replace(/[\\;,:"]/g, (c) => `\\${c}`);
}

export function wifiPayload({ ssid, password, security, hidden }: WifiNetwork): string {
  let out = `WIFI:T:${security};S:${escapeWifi(ssid)};`;
  if (security !== 'nopass') out += `P:${escapeWifi(password)};`;
  if (hidden) out += 'H:true;';
  return `${out};`;
}

/** Returns a message describing what's wrong, or null when the network can be encoded. */
export function wifiProblem({ ssid, password, security }: WifiNetwork): string | null {
  if (!ssid) return 'Enter the network name (SSID).';
  if (new TextEncoder().encode(ssid).length > 32) return 'Network names can be at most 32 bytes long; this one is longer.';
  if (security === 'WPA') {
    if (password.length < 8) return 'WPA passwords are at least 8 characters long.';
    if (password.length > 63 && !/^[0-9a-fA-F]{64}$/.test(password)) return 'WPA passwords are at most 63 characters (or exactly 64 hex digits).';
  }
  if (security === 'WEP' && ![5, 13, 10, 26].includes(password.length)) {
    return 'WEP keys are 5 or 13 characters, or 10 or 26 hex digits.';
  }
  return null;
}
